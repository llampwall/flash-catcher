use crate::event::FlashEvent;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;

const BROADCAST_CAP: usize = 256;

pub struct Store {
    pub data_dir: PathBuf,
    events_path: PathBuf,
    writer: Arc<Mutex<std::io::BufWriter<std::fs::File>>>,
    tx: broadcast::Sender<FlashEvent>,
    pub total_events: Arc<AtomicU64>,
}

impl Store {
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("create data dir {:?}", data_dir))?;

        let events_path = data_dir.join("events.jsonl");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)
            .with_context(|| format!("open events.jsonl at {:?}", events_path))?;

        let (tx, _) = broadcast::channel(BROADCAST_CAP);

        Ok(Self {
            data_dir,
            events_path,
            writer: Arc::new(Mutex::new(std::io::BufWriter::new(file))),
            tx,
            total_events: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Persist an event to JSONL, optionally broadcasting it to live SSE
    /// subscribers. Forensic/state events (e.g. ETW data-collection-start)
    /// pass `broadcast=false` so the live dashboard isn't polluted with
    /// "events" that represent processes already running at trace start.
    pub async fn append(&self, event: &FlashEvent, broadcast: bool) -> Result<()> {
        let line = serde_json::to_string(event).context("serialize FlashEvent")?;
        {
            let mut w = self.writer.lock().await;
            writeln!(*w, "{}", line).context("write event line")?;
            w.flush().context("flush events.jsonl")?;
        }
        self.total_events.fetch_add(1, Ordering::Relaxed);
        if broadcast {
            // Lagging subscribers drop events rather than blocking
            let _ = self.tx.send(event.clone());
        }
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<FlashEvent> {
        self.tx.subscribe()
    }

    pub async fn read_all(data_dir: impl AsRef<Path>) -> Result<Vec<FlashEvent>> {
        let data_dir = data_dir.as_ref();
        let mut events: Vec<FlashEvent> = Vec::new();

        // Collect all matching files sorted by name (oldest first since timestamps sort lexically)
        let mut paths: Vec<PathBuf> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(data_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if name.starts_with("events") && (name.ends_with(".jsonl") || name.ends_with(".jsonl.gz")) {
                    paths.push(path);
                }
            }
        }
        paths.sort();

        for path in &paths {
            if path.extension().and_then(|e| e.to_str()) == Some("gz") {
                read_gz_jsonl(&path, &mut events);
            } else {
                read_plain_jsonl(&path, &mut events);
            }
        }

        Ok(events)
    }

    pub async fn rotate_if_needed(&self, max_bytes: u64) -> Result<()> {
        let meta = match std::fs::metadata(&self.events_path) {
            Ok(m) => m,
            Err(_) => return Ok(()),
        };
        if meta.len() < max_bytes {
            return Ok(());
        }

        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let rotated_name = format!("events-{}.jsonl", ts);
        let rotated_path = self.data_dir.join(&rotated_name);
        let gz_path = self.data_dir.join(format!("{}.gz", rotated_name));

        // Lock writer so no new writes race with rename
        let mut w = self.writer.lock().await;
        w.flush().ok();

        drop(w); // unlock while we rename — brief window, acceptable

        std::fs::rename(&self.events_path, &rotated_path)
            .context("rename events.jsonl for rotation")?;

        // Gzip the rotated file
        if let Err(e) = gzip_file(&rotated_path, &gz_path) {
            tracing::warn!("Failed to gzip rotated log: {e}");
        } else {
            let _ = std::fs::remove_file(&rotated_path);
        }

        // Reopen the writer
        let new_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)
            .context("reopen events.jsonl after rotation")?;

        let mut w = self.writer.lock().await;
        *w = std::io::BufWriter::new(new_file);

        tracing::info!("Rotated events log to {:?}", gz_path);
        Ok(())
    }
}

fn read_plain_jsonl(path: &Path, out: &mut Vec<FlashEvent>) {
    use std::io::BufRead;
    let Ok(file) = std::fs::File::open(path) else { return };
    for line in std::io::BufReader::new(file).lines().flatten() {
        if let Ok(ev) = serde_json::from_str::<FlashEvent>(&line) {
            out.push(ev);
        }
    }
}

fn read_gz_jsonl(path: &Path, out: &mut Vec<FlashEvent>) {
    use flate2::read::GzDecoder;
    use std::io::BufRead;
    let Ok(file) = std::fs::File::open(path) else { return };
    let decoder = GzDecoder::new(file);
    for line in std::io::BufReader::new(decoder).lines().flatten() {
        if let Ok(ev) = serde_json::from_str::<FlashEvent>(&line) {
            out.push(ev);
        }
    }
}

fn gzip_file(src: &Path, dst: &Path) -> Result<()> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::copy;

    let mut input = std::fs::File::open(src).context("open for gzip")?;
    let output = std::fs::File::create(dst).context("create gz")?;
    let mut encoder = GzEncoder::new(output, Compression::default());
    copy(&mut input, &mut encoder).context("compress")?;
    encoder.finish().context("gzip finish")?;
    Ok(())
}
