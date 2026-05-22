// flash-watcher UI client.
// Loads initial rows via /api/rows, then subscribes to /api/stream (SSE) for live
// updates. Re-aggregates client-side on every incoming event so the view stays
// in sync without re-fetching /api/rows on every tick.

const state = {
  rows: new Map(),       // key -> BlameChainRow
  paused: false,
  sortBy: 'most-recent',
  filterClass: 'all',
  expanded: new Set(),   // expanded chain keys
};

const els = {};

function init() {
  throw new Error('not implemented: cache DOM nodes, wire controls, kick off loadInitial + connectStream');
}

async function loadInitial() {
  throw new Error('not implemented: fetch /api/rows?sort=<state.sortBy>, populate state.rows, renderRows');
}

function connectStream() {
  throw new Error('not implemented: new EventSource("/api/stream"), onmessage applyEvent(), onerror reconnect backoff');
}

function applyEvent(event) {
  throw new Error('not implemented: fold one FlashEvent into state.rows by blame.key, renderRows()');
}

function renderRows() {
  throw new Error('not implemented: sort state.rows per state.sortBy, filter per state.filterClass, render tbody');
}

async function expandRow(chainKey) {
  throw new Error('not implemented: fetch /api/events?chain=<key>&limit=50, render <template#detail-template>');
}

function collapseRow(chainKey) {
  throw new Error('not implemented: remove detail section, drop from state.expanded');
}

function formatRelative(timestamp) {
  throw new Error('not implemented: "3s ago", "2m ago", "1h ago"');
}

function formatDuration(ms) {
  throw new Error('not implemented: "47ms", "1.2s", "3m"');
}

async function refreshHealth() {
  throw new Error('not implemented: GET /api/health, update topbar status pills');
}

init();
