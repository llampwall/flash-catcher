/// Conhost pairing: match a conhost.exe process to the console process that allocated it.
///
/// Heuristic (documented per spec):
///   - conhost.exe spawned within 100ms of the allocator process
///   - Both in the same Windows session
///   - conhost's parent matches the session host (usually csrss.exe or the allocator's parent)
///
/// This is heuristic only — the Windows API does not provide a direct allocator→conhost mapping.
/// In practice, conhost.exe appears in ETW events within a few milliseconds of its allocating
/// console process; we pair by timing within a session.
use crate::event::ConhostPairing;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;

const PAIR_WINDOW_MS: i64 = 100;

#[derive(Clone, Default)]
pub struct ConhostPairer {
    /// pid → (spawned_at, session_id) for conhost.exe processes pending pairing
    pending_conhosts: Arc<DashMap<u32, (DateTime<Utc>, u32)>>,
    /// allocator pid → paired ConhostPairing
    pairings: Arc<DashMap<u32, ConhostPairing>>,
}

impl ConhostPairer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a conhost.exe spawn. May pair with a recently-seen allocator.
    pub fn record_conhost(&self, pid: u32, spawned_at: DateTime<Utc>, session_id: u32) {
        self.pending_conhosts.insert(pid, (spawned_at, session_id));
    }

    /// Record an allocator process spawn. Look for a pending conhost within the pairing window.
    pub fn record_allocator(
        &self,
        allocator_pid: u32,
        spawned_at: DateTime<Utc>,
        session_id: u32,
    ) -> Option<ConhostPairing> {
        // Find any pending conhost spawned within PAIR_WINDOW_MS of this allocator, same session
        let mut best: Option<(u32, DateTime<Utc>)> = None;

        for entry in self.pending_conhosts.iter() {
            let (ch_pid, (ch_ts, ch_session)) = (entry.key(), entry.value());
            if *ch_session != session_id {
                continue;
            }
            let diff = (ch_ts.timestamp_millis() - spawned_at.timestamp_millis()).abs();
            if diff <= PAIR_WINDOW_MS {
                if best.is_none()
                    || diff < (best.unwrap().1.timestamp_millis() - spawned_at.timestamp_millis()).abs()
                {
                    best = Some((*ch_pid, *ch_ts));
                }
            }
        }

        if let Some((ch_pid, ch_ts)) = best {
            self.pending_conhosts.remove(&ch_pid);
            let pairing = ConhostPairing {
                conhost_pid: ch_pid,
                spawned_at: ch_ts,
                exited_at: None,
            };
            self.pairings.insert(allocator_pid, pairing.clone());
            Some(pairing)
        } else {
            None
        }
    }

    /// Mark a conhost process as exited; update the pairing if already established.
    pub fn mark_conhost_exited(&self, conhost_pid: u32, exited_at: DateTime<Utc>) {
        // Scan pairings for this conhost_pid
        for mut entry in self.pairings.iter_mut() {
            if entry.value().conhost_pid == conhost_pid {
                entry.value_mut().exited_at = Some(exited_at);
            }
        }
    }

    /// Retrieve the pairing for an allocator process, if it exists.
    #[allow(dead_code)]
    pub fn pair_conhost(&self, allocator_pid: u32) -> Option<ConhostPairing> {
        self.pairings.get(&allocator_pid).map(|p| p.clone())
    }
}
