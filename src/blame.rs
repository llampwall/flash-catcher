use crate::event::{BlameChain, BlameNode};
use dashmap::DashMap;
use std::sync::Arc;

/// Live ancestry cache — populated by every ProcessStart event so that
/// when a short-lived process dies before we can walk its parent chain
/// via OpenProcess, we still have the chain in memory.
#[derive(Clone, Default)]
pub struct BlameCache {
    /// pid -> (ppid, name, exe_path)
    nodes: Arc<DashMap<u32, CachedNode>>,
}

#[derive(Debug, Clone)]
struct CachedNode {
    ppid: u32,
    name: String,
    exe_path: Option<String>,
}

impl BlameCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a freshly observed process. Called once per ProcessStart event.
    pub fn record(&self, _pid: u32, _ppid: u32, _name: &str, _exe_path: Option<&str>) {
        unimplemented!("insert into nodes map; never evict (processes can be very deep)")
    }

    /// Mark a pid as exited. Kept in the map (reaped lazily) so late-arriving
    /// child events can still resolve their parent chain.
    pub fn mark_exited(&self, _pid: u32) {
        unimplemented!("optional: tombstone for later GC, do not remove immediately")
    }

    /// Walk the parent chain from `pid` up to the root and return the blame chain.
    /// If a parent is unknown to the cache, falls back to a one-shot Win32 lookup
    /// against any still-live ancestor.
    pub fn walk(&self, _pid: u32) -> BlameChain {
        unimplemented!("loop ppid lookups, build Vec<BlameNode>, compute key as joined names")
    }

    /// Number of cached nodes (for diagnostics).
    pub fn size(&self) -> usize {
        self.nodes.len()
    }
}

/// Compute the deterministic blame-chain key used for UI grouping.
/// Format: `child<-parent<-grandparent<-...<-root`, lowercased exe names.
pub fn chain_key(_nodes: &[BlameNode]) -> String {
    unimplemented!("nodes.iter().map(|n| n.name.to_lowercase()).join('<-')")
}
