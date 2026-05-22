mod admin;
mod aggregate;
mod blame;
mod classify;
mod cli;
mod conhost;
mod etw;
mod event;
mod process;
mod store;
mod web;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use std::sync::Arc;

use aggregate::Aggregator;
use blame::BlameCache;
use classify::classify;
use conhost::ConhostPairer;
use event::{FlashEvent, Subsystem};
use parking_lot::Mutex;
use store::Store;
use web::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Run(args) => run_collector(args).await,
        Command::View(args) => run_viewer(args).await,
        Command::ClassifyRules(args) => print_classify_rules(args),
    }
}

async fn run_collector(args: cli::RunArgs) -> Result<()> {
    if !args.skip_admin_check {
        admin::require_elevation_or_relaunch()?;
    }

    let store = Arc::new(Store::open(&args.data_dir)?);
    let aggregator = Arc::new(Mutex::new(Aggregator::new()));

    // Replay existing JSONL into the aggregator for backfill
    tracing::info!("Replaying existing events from {:?}", args.data_dir);
    let history = Store::read_all(&args.data_dir).await.unwrap_or_default();
    {
        let mut agg = aggregator.lock();
        for ev in &history {
            agg.ingest(ev);
        }
    }
    let historical_count = history.len();
    store.total_events.fetch_add(historical_count as u64, std::sync::atomic::Ordering::Relaxed);
    tracing::info!("Replayed {} historical events", historical_count);

    let state = AppState::new(store.clone(), aggregator.clone(), true);

    // Pre-populate recent_events ring with last N historical events
    {
        let take = history.len().min(2000);
        let skip = history.len().saturating_sub(take);
        for ev in history.into_iter().skip(skip) {
            state.push_recent_event(ev);
        }
    }

    // Start ETW kernel session
    let mut rx = etw::start_kernel_session()?;
    tracing::info!("ETW kernel session started");

    let blame_cache = BlameCache::new();
    let conhost_pairer = ConhostPairer::new();

    // Rotation timer
    let store_rot = store.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = store_rot.rotate_if_needed(50 * 1024 * 1024).await {
                tracing::warn!("Rotation error: {}", e);
            }
        }
    });

    // Web server
    let bind = args.bind.clone();
    let state_web = state.clone();
    tokio::spawn(async move {
        if let Err(e) = web::serve(state_web, &bind).await {
            tracing::error!("Web server error: {}", e);
        }
    });

    if args.open {
        open_browser(&args.bind);
    }

    // Collector loop — process ETW events
    while let Some(raw) = rx.recv().await {
        match &raw {
            etw::RawEvent::ProcessStart {
                pid,
                ppid,
                image_file_name,
                command_line,
                timestamp,
            } => {
                blame_cache.record(*pid, *ppid, image_file_name, None);

                let process_info = match etw::enrich_raw(&raw) {
                    Ok(Some(info)) => info,
                    Ok(None) => continue,
                    Err(e) => {
                        tracing::debug!("enrich failed for pid {}: {}", pid, e);
                        // Build minimal ProcessInfo from ETW data
                        event::ProcessInfo {
                            pid: *pid,
                            ppid: *ppid,
                            name: image_file_name.clone(),
                            exe_path: None,
                            command_line: if command_line.is_empty() {
                                None
                            } else {
                                Some(command_line.clone())
                            },
                            working_directory: None,
                            session_id: 0,
                            integrity_level: event::IntegrityLevel::Unknown,
                            subsystem: event::Subsystem::Unknown,
                            creation_flags: 0,
                            stdio: event::StdioHandles {
                                stdin: event::HandleKind::Unknown,
                                stdout: event::HandleKind::Unknown,
                                stderr: event::HandleKind::Unknown,
                            },
                        }
                    }
                };

                // Update exe_path in blame cache if we got it
                if let Some(ref path) = process_info.exe_path {
                    blame_cache.record(
                        *pid,
                        *ppid,
                        image_file_name,
                        Some(path.as_str()),
                    );
                }

                let blame = blame_cache.walk(*pid);
                let ancestor_names: Vec<String> = blame
                    .ancestors
                    .iter()
                    .map(|n| n.name.clone())
                    .collect();
                let (classification, rule_name) = classify(&process_info, &ancestor_names);

                // Conhost pairing
                let conhost_pair = if image_file_name.to_lowercase() == "conhost.exe" {
                    conhost_pairer.record_conhost(*pid, *timestamp, process_info.session_id);
                    None
                } else if process_info.subsystem == Subsystem::Console {
                    conhost_pairer.record_allocator(*pid, *timestamp, process_info.session_id)
                } else {
                    None
                };

                let visible_flash = process_info.subsystem == Subsystem::Console
                    && process_info.stdio.stdout != event::HandleKind::Pipe
                    && process_info.stdio.stdout != event::HandleKind::Null;

                let flash_event = FlashEvent {
                    event_id: FlashEvent::new_id(),
                    spawned_at: *timestamp,
                    exited_at: None,
                    lifetime_ms: None,
                    exit_code: None,
                    process: process_info,
                    blame,
                    conhost: conhost_pair,
                    classification,
                    classification_rule: rule_name.map(|s| s.to_string()),
                    visible_flash,
                };

                state.push_recent_event(flash_event.clone());
                aggregator.lock().ingest(&flash_event);

                if let Err(e) = store.append(&flash_event).await {
                    tracing::warn!("Store append error: {}", e);
                }
            }
            etw::RawEvent::ProcessExit {
                pid, exit_code, timestamp, ..
            } => {
                blame_cache.mark_exited(*pid);
                conhost_pairer.mark_conhost_exited(*pid, *timestamp);

                // Update the most recent flash_event for this pid with exit info
                // (we update in the ring; JSONL is append-only so we don't rewrite)
                {
                    let mut ring = state.recent_events.lock();
                    if let Some(ev) = ring.iter_mut().rev().find(|e| e.process.pid == *pid) {
                        let spawned = ev.spawned_at;
                        ev.exited_at = Some(*timestamp);
                        ev.exit_code = Some(*exit_code);
                        ev.lifetime_ms = Some(
                            (timestamp.timestamp_millis() - spawned.timestamp_millis())
                                .unsigned_abs(),
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_viewer(args: cli::ViewArgs) -> Result<()> {
    // View mode: no ETW, read existing JSONL
    let store = Arc::new(Store::open(&args.data_dir)?);
    let aggregator = Arc::new(Mutex::new(Aggregator::new()));

    let history = Store::read_all(&args.data_dir).await.unwrap_or_default();
    {
        let mut agg = aggregator.lock();
        for ev in &history {
            agg.ingest(ev);
        }
    }
    let count = history.len();
    store.total_events.fetch_add(count as u64, std::sync::atomic::Ordering::Relaxed);
    tracing::info!("View mode: loaded {} historical events", count);

    let state = AppState::new(store, aggregator, false);
    {
        let take = history.len().min(2000);
        let skip = history.len().saturating_sub(take);
        for ev in history.into_iter().skip(skip) {
            state.push_recent_event(ev);
        }
    }

    if args.open {
        open_browser(&args.bind);
    }

    web::serve(state, &args.bind).await
}

fn print_classify_rules(args: cli::ClassifyRulesArgs) -> Result<()> {
    println!("{}", classify::dump_rules(args.pretty));
    Ok(())
}

fn open_browser(bind: &str) {
    let url = format!("http://{}/", bind);
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", &url])
        .spawn();
}
