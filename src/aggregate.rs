use crate::event::{Classification, FlashEvent};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One row in the landing-view table — a unique blame-chain ancestry with
/// rolling counts. Built by reducing `FlashEvent`s, keyed by `blame.key`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameChainRow {
    pub key: String,
    pub chain_display: String,
    pub classification: Classification,
    pub count: u64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    /// Sum of lifetime_ms across all events on this chain.
    pub total_console_time_ms: u64,
    /// Number of events where `visible_flash == true`.
    pub visible_count: u64,
    /// Recent event_ids (capped) for the row's expanded detail pane.
    pub recent_event_ids: Vec<String>,
}

/// Sort axes supported by the landing view.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SortBy {
    MostRecent,
    HighestCount,
    LongestLifetime,
}

#[derive(Debug, Clone, Default)]
pub struct Aggregator {
    rows: std::collections::HashMap<String, BlameChainRow>,
}

impl Aggregator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold a single event into the aggregate state. Idempotent only if
    /// `event_id`s are unique — caller is responsible for not double-feeding.
    pub fn ingest(&mut self, _event: &FlashEvent) {
        unimplemented!("upsert by blame.key, bump count, update last_seen, push recent_event_ids (cap 50)")
    }

    /// Snapshot the current rows, sorted by the requested axis.
    pub fn snapshot(&self, _sort: SortBy) -> Vec<BlameChainRow> {
        unimplemented!("clone rows.values into Vec, sort_by per axis, return")
    }

    /// Total row count (for diagnostics / health endpoint).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}
