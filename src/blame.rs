use crate::event::{BlameChain, BlameNode};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct BlameCache {
    nodes: Arc<DashMap<u32, CachedNode>>,
}

#[derive(Debug, Clone)]
struct CachedNode {
    ppid: u32,
    name: String,
    exe_path: Option<String>,
    exited: bool,
}

impl BlameCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, pid: u32, ppid: u32, name: &str, exe_path: Option<&str>) {
        self.nodes.insert(
            pid,
            CachedNode {
                ppid,
                name: name.to_string(),
                exe_path: exe_path.map(|s| s.to_string()),
                exited: false,
            },
        );
    }

    pub fn mark_exited(&self, pid: u32) {
        if let Some(mut entry) = self.nodes.get_mut(&pid) {
            entry.exited = true;
        }
    }

    pub fn walk(&self, pid: u32) -> BlameChain {
        let mut ancestors: Vec<BlameNode> = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut current_ppid = {
            if let Some(node) = self.nodes.get(&pid) {
                node.ppid
            } else {
                // Fallback: look up in system snapshot
                crate::process::snapshot_ppid(pid).unwrap_or(0)
            }
        };

        while current_ppid != 0 && !visited.contains(&current_ppid) {
            visited.insert(current_ppid);

            if let Some(node) = self.nodes.get(&current_ppid) {
                ancestors.push(BlameNode {
                    pid: current_ppid,
                    name: node.name.clone(),
                    exe_path: node.exe_path.clone(),
                });
                let next = node.ppid;
                drop(node);
                current_ppid = next;
            } else {
                // Not in cache — try toolhelp snapshot for still-live ancestors
                let name = crate::process::snapshot_name(current_ppid);
                let ppid = crate::process::snapshot_ppid(current_ppid).unwrap_or(0);
                if let Some(n) = name {
                    ancestors.push(BlameNode {
                        pid: current_ppid,
                        name: n,
                        exe_path: None,
                    });
                    current_ppid = ppid;
                } else {
                    break;
                }
            }
        }

        let key = chain_key(&ancestors);
        BlameChain { ancestors, key }
    }

    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        self.nodes.len()
    }
}

pub fn chain_key(nodes: &[BlameNode]) -> String {
    if nodes.is_empty() {
        return String::from("unknown");
    }
    nodes
        .iter()
        .map(|n| n.name.to_lowercase())
        .collect::<Vec<_>>()
        .join("<-")
}
