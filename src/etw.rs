use crate::event::ProcessInfo;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use ferrisetw::parser::Parser;
use ferrisetw::provider::kernel_providers;
use ferrisetw::provider::Provider;
use ferrisetw::schema_locator::SchemaLocator;
use ferrisetw::trace::KernelTrace;
use ferrisetw::EventRecord;
use tokio::sync::mpsc;

pub const SESSION_NAME: &str = "flash-watcher-kernel";

#[derive(Debug, Clone)]
pub enum RawEvent {
    ProcessStart {
        pid: u32,
        ppid: u32,
        image_file_name: String,
        command_line: String,
        timestamp: DateTime<Utc>,
    },
    ProcessExit {
        pid: u32,
        exit_code: i32,
        timestamp: DateTime<Utc>,
    },
}

/// Convert a Windows FILETIME (100-ns intervals since 1601-01-01) to DateTime<Utc>.
fn filetime_to_utc(ts: i64) -> DateTime<Utc> {
    // 100-ns ticks between Windows epoch (1601-01-01) and Unix epoch (1970-01-01)
    const EPOCH_DIFF_100NS: i64 = 116_444_736_000_000_000;
    let unix_100ns = ts.saturating_sub(EPOCH_DIFF_100NS);
    let secs = unix_100ns / 10_000_000;
    let nanos = ((unix_100ns % 10_000_000) * 100) as u32;
    DateTime::from_timestamp(secs, nanos).unwrap_or_else(Utc::now)
}

pub fn start_kernel_session() -> Result<mpsc::Receiver<RawEvent>> {
    let (tx, rx) = mpsc::channel::<RawEvent>(1024);

    let process_callback = {
        let tx = tx.clone();
        move |record: &EventRecord, schema_locator: &SchemaLocator| {
            let opcode = record.opcode();
            // 1 = ProcessStart, 3 = DCStart (existing when trace begins)
            // 2 = ProcessExit,  4 = DCEnd
            let Ok(schema) = schema_locator.event_schema(record) else {
                return;
            };
            let parser = Parser::create(record, &schema);
            let ts = filetime_to_utc(record.raw_timestamp());

            match opcode {
                1 | 3 => {
                    let pid = parser
                        .try_parse::<u32>("ProcessId")
                        .unwrap_or_else(|_| record.process_id());
                    let ppid = parser.try_parse::<u32>("ParentId").unwrap_or(0);
                    let image = parser
                        .try_parse::<String>("ImageFileName")
                        .unwrap_or_default();
                    let cmdline = parser
                        .try_parse::<String>("CommandLine")
                        .unwrap_or_default();

                    let event = RawEvent::ProcessStart {
                        pid,
                        ppid,
                        image_file_name: image,
                        command_line: cmdline,
                        timestamp: ts,
                    };
                    if tx.blocking_send(event).is_err() {
                        // Receiver dropped — trace should be stopped
                    }
                }
                2 | 4 => {
                    let pid = parser
                        .try_parse::<u32>("ProcessId")
                        .unwrap_or_else(|_| record.process_id());
                    let exit_code = parser.try_parse::<i32>("ExitCode").unwrap_or(-1);

                    let event = RawEvent::ProcessExit {
                        pid,
                        exit_code,
                        timestamp: ts,
                    };
                    if tx.blocking_send(event).is_err() {
                        // Receiver dropped
                    }
                }
                _ => {}
            }
        }
    };

    let provider = Provider::kernel(&kernel_providers::PROCESS_PROVIDER)
        .add_callback(process_callback)
        .build();

    // Try to start; recover from leaked session (ERROR_ALREADY_EXISTS)
    let trace = match KernelTrace::new()
        .named(SESSION_NAME.to_string())
        .enable(provider)
        .start_and_process()
    {
        Ok(t) => t,
        Err(ferrisetw::trace::TraceError::EtwNativeError(
            ferrisetw::native::EvntraceNativeError::AlreadyExist,
        )) => {
            tracing::warn!(
                "ETW session '{}' already exists — stopping leaked session and retrying",
                SESSION_NAME
            );
            stop_session(SESSION_NAME).ok();

            let provider2 = Provider::kernel(&kernel_providers::PROCESS_PROVIDER)
                .add_callback(move |record: &EventRecord, schema_locator: &SchemaLocator| {
                    let _ = (record, schema_locator);
                })
                .build();
            KernelTrace::new()
                .named(SESSION_NAME.to_string())
                .enable(provider2)
                .start_and_process()
                .map_err(|e| anyhow!("ETW session retry failed: {:?}", e))?
        }
        Err(e) => return Err(anyhow!("ETW session start failed: {:?}", e)),
    };

    // Keep the trace alive for the process lifetime
    std::thread::spawn(move || {
        let _trace = trace;
        // Park forever — the trace lives as long as this thread
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    });

    Ok(rx)
}

pub fn enrich_raw(raw: &RawEvent) -> Result<Option<ProcessInfo>> {
    match raw {
        RawEvent::ProcessStart {
            pid,
            ppid,
            image_file_name,
            command_line,
            ..
        } => Ok(Some(crate::process::enrich(*pid, *ppid, image_file_name, command_line)?)),
        RawEvent::ProcessExit { .. } => Ok(None),
    }
}

pub fn stop_session(session_name: &str) -> Result<()> {
    use std::mem;
    use windows::Win32::System::Diagnostics::Etw::{
        ControlTraceW, EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_PROPERTIES, CONTROLTRACE_HANDLE,
    };
    use windows::core::PCWSTR;

    let name_wide: Vec<u16> = session_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let name_bytes = name_wide.len() * 2;
    let total_size = mem::size_of::<EVENT_TRACE_PROPERTIES>() + name_bytes;

    let mut buf = vec![0u8; total_size];

    unsafe {
        let props = buf.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
        (*props).Wnode.BufferSize = total_size as u32;
        // WNODE_FLAG_TRACED_GUID = 0x00020000
        (*props).Wnode.Flags = 0x00020000;
        (*props).LoggerNameOffset = mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;

        let name_ptr =
            buf.as_mut_ptr().add(mem::size_of::<EVENT_TRACE_PROPERTIES>()) as *mut u16;
        std::ptr::copy_nonoverlapping(name_wide.as_ptr(), name_ptr, name_wide.len());

        let result = ControlTraceW(
            CONTROLTRACE_HANDLE { Value: 0 },
            PCWSTR(name_ptr),
            props,
            EVENT_TRACE_CONTROL_STOP,
        );

        match result.ok() {
            Ok(()) => Ok(()),
            Err(e) => {
                // 0x80071069 = ERROR_WMI_INSTANCE_NOT_FOUND — session wasn't running, OK
                if e.code().0 as u32 == 0x80071069 {
                    Ok(())
                } else {
                    Err(anyhow!("ControlTraceW STOP failed: {:?}", e))
                }
            }
        }
    }
}
