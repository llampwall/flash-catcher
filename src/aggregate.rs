use crate::event::{Classification, FlashEvent};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameChainRow {
    pub key: String,
    pub chain_display: String,
    pub classification: Classification,
    pub count: u64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub total_console_time_ms: u64,
    pub visible_count: u64,
    pub recent_event_ids: Vec<String>,
}

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

const MAX_RECENT_IDS: usize = 50;

impl Aggregator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ingest(&mut self, event: &FlashEvent) {
        let key = event.blame.key.clone();
        let chain_display = key.clone();

        let row = self.rows.entry(key.clone()).or_insert_with(|| BlameChainRow {
            key: key.clone(),
            chain_display: chain_display.clone(),
            classification: event.classification,
            count: 0,
            first_seen: event.spawned_at,
            last_seen: event.spawned_at,
            total_console_time_ms: 0,
            visible_count: 0,
            recent_event_ids: Vec::new(),
        });

        row.count += 1;
        if event.spawned_at < row.first_seen {
            row.first_seen = event.spawned_at;
        }
        if event.spawned_at > row.last_seen {
            row.last_seen = event.spawned_at;
        }
        if let Some(ms) = event.lifetime_ms {
            row.total_console_time_ms += ms;
        }
        if event.visible_flash {
            row.visible_count += 1;
        }
        // Keep the most severe classification seen for this chain
        if event.classification == Classification::Unknown
            || row.classification == Classification::KnownBenign
        {
            row.classification = event.classification;
        }

        row.recent_event_ids.push(event.event_id.clone());
        if row.recent_event_ids.len() > MAX_RECENT_IDS {
            row.recent_event_ids.remove(0);
        }
    }

    pub fn snapshot(&self, sort: SortBy) -> Vec<BlameChainRow> {
        let mut rows: Vec<BlameChainRow> = self.rows.values().cloned().collect();
        match sort {
            SortBy::MostRecent => rows.sort_by(|a, b| b.last_seen.cmp(&a.last_seen)),
            SortBy::HighestCount => rows.sort_by(|a, b| b.count.cmp(&a.count)),
            SortBy::LongestLifetime => {
                rows.sort_by(|a, b| b.total_console_time_ms.cmp(&a.total_console_time_ms))
            }
        }
        rows
    }

    pub fn update_lifetime(&mut self, key: &str, lifetime_ms: u64) {
        if let Some(row) = self.rows.get_mut(key) {
            row.total_console_time_ms += lifetime_ms;
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}
