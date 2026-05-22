use crate::event::FlashEvent;
use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::sync::broadcast;

/// Append-only JSONL store. Writes to `data_dir/events.jsonl`, rotates by size
/// (gz on rotate). Also re-broadcasts every appended event to subscribers,
/// which is how the SSE endpoint streams live data.
pub struct Store {
    data_dir: PathBuf,
    tx: broadcast::Sender<FlashEvent>,
}

impl Store {
    pub fn open(_data_dir: impl AsRef<Path>) -> Result<Self> {
        unimplemented!("mkdir -p data_dir, open events.jsonl in append+create mode, build broadcast channel")
    }

    /// Append one event and notify subscribers. Errors are surfaced — caller
    /// decides whether to abort the collector or keep going on disk failure.
    pub async fn append(&self, _event: &FlashEvent) -> Result<()> {
        unimplemented!("serialize as JSON line, write+flush, send to broadcast")
    }

    /// Subscribe to the live stream. Returns a receiver of new events.
    pub fn subscribe(&self) -> broadcast::Receiver<FlashEvent> {
        self.tx.subscribe()
    }

    /// Read all events from the on-disk JSONL files in `data_dir`, oldest first.
    /// Used by the web UI on initial page load to backfill the table.
    pub async fn read_all(_data_dir: impl AsRef<Path>) -> Result<Vec<FlashEvent>> {
        unimplemented!("glob data_dir/events*.jsonl(.gz)? files, parse line-by-line, return Vec")
    }

    /// Rotate the active JSONL file when it exceeds `max_bytes`.
    /// Compresses the rotated file to .gz to save space.
    pub async fn rotate_if_needed(&self, _max_bytes: u64) -> Result<()> {
        unimplemented!("stat events.jsonl; if > max, rename to events-<ts>.jsonl, gzip, reopen writer")
    }
}
