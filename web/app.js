// flash-watcher UI client
// Loads initial rows via /api/rows, then subscribes to /api/stream (SSE).
// Client-side re-aggregates on every incoming event so the view stays
// in sync without re-fetching /api/rows on every tick.

const state = {
  rows: new Map(),        // key -> BlameChainRow
  paused: false,
  sortBy: 'most-recent',
  filterClass: 'all',
  expanded: new Set(),    // expanded chain keys
  eventSource: null,
  reconnectDelay: 1000,
};

const els = {};

function init() {
  els.statusEtw    = document.getElementById('status-etw');
  els.statusEvents = document.getElementById('status-events');
  els.statusRows   = document.getElementById('status-rows');
  els.sortSelect   = document.getElementById('sort-select');
  els.filterClass  = document.getElementById('filter-class');
  els.pauseBtn     = document.getElementById('pause-btn');
  els.clearBtn     = document.getElementById('clear-btn');
  els.rowsBody     = document.getElementById('rows-body');

  els.sortSelect.addEventListener('change', () => {
    state.sortBy = els.sortSelect.value;
    renderRows();
  });

  els.filterClass.addEventListener('change', () => {
    state.filterClass = els.filterClass.value;
    renderRows();
  });

  els.pauseBtn.addEventListener('click', () => {
    state.paused = !state.paused;
    els.pauseBtn.textContent = state.paused ? 'Resume' : 'Pause';
  });

  els.clearBtn.addEventListener('click', () => {
    els.rowsBody.innerHTML = '';
    state.expanded.clear();
  });

  loadInitial();
  connectStream();

  // Poll health every 10s
  refreshHealth();
  setInterval(refreshHealth, 10_000);
}

async function loadInitial() {
  try {
    const res = await fetch('/api/rows?sort=' + state.sortBy);
    if (!res.ok) return;
    const rows = await res.json();
    state.rows.clear();
    for (const row of rows) {
      state.rows.set(row.key, row);
    }
    renderRows();
  } catch (e) {
    console.error('loadInitial failed', e);
  }
}

function connectStream() {
  if (state.eventSource) {
    state.eventSource.close();
  }
  const es = new EventSource('/api/stream');
  state.eventSource = es;

  es.onmessage = (evt) => {
    if (state.paused) return;
    try {
      const event = JSON.parse(evt.data);
      applyEvent(event);
    } catch (e) {
      // skip malformed
    }
  };

  es.onerror = () => {
    es.close();
    state.eventSource = null;
    // Reconnect with backoff
    setTimeout(() => {
      state.reconnectDelay = Math.min(state.reconnectDelay * 2, 30_000);
      connectStream();
    }, state.reconnectDelay);
  };

  es.onopen = () => {
    state.reconnectDelay = 1000;
  };
}

function applyEvent(event) {
  const key = event.blame && event.blame.key ? event.blame.key : 'unknown';
  const existing = state.rows.get(key);

  if (existing) {
    existing.count += 1;
    if (event.visible_flash) existing.visible_count = (existing.visible_count || 0) + 1;
    existing.last_seen = event.spawned_at;
    if (event.lifetime_ms) existing.total_console_time_ms = (existing.total_console_time_ms || 0) + event.lifetime_ms;
    existing.recent_event_ids = existing.recent_event_ids || [];
    existing.recent_event_ids.push(event.event_id);
    if (existing.recent_event_ids.length > 50) existing.recent_event_ids.shift();
  } else {
    state.rows.set(key, {
      key,
      chain_display: key,
      classification: event.classification || 'unknown',
      count: 1,
      visible_count: event.visible_flash ? 1 : 0,
      first_seen: event.spawned_at,
      last_seen: event.spawned_at,
      total_console_time_ms: event.lifetime_ms || 0,
      recent_event_ids: [event.event_id],
    });
  }

  renderRows();
}

function sortedRows() {
  const rows = Array.from(state.rows.values());
  switch (state.sortBy) {
    case 'highest-count':
      return rows.sort((a, b) => b.count - a.count);
    case 'longest-lifetime':
      return rows.sort((a, b) => (b.total_console_time_ms || 0) - (a.total_console_time_ms || 0));
    default:
      return rows.sort((a, b) => {
        const ta = a.last_seen ? new Date(a.last_seen).getTime() : 0;
        const tb = b.last_seen ? new Date(b.last_seen).getTime() : 0;
        return tb - ta;
      });
  }
}

function filteredRows(rows) {
  if (state.filterClass === 'all') return rows;
  return rows.filter(r => r.classification === state.filterClass);
}

function renderRows() {
  const rows = filteredRows(sortedRows());

  // Update status counts
  els.statusEvents.textContent = totalEvents() + ' events';
  els.statusRows.textContent   = state.rows.size + ' chains';

  // Diff the DOM: keep existing rows, update counts, add new ones
  const existingKeys = new Set();
  for (const tr of els.rowsBody.querySelectorAll('tr.row')) {
    existingKeys.add(tr.dataset.key);
  }

  // Remove rows no longer visible after filter
  for (const tr of els.rowsBody.querySelectorAll('tr.row')) {
    if (!rows.find(r => r.key === tr.dataset.key)) {
      const detailTr = tr.nextElementSibling;
      if (detailTr && detailTr.classList.contains('detail-row')) detailTr.remove();
      tr.remove();
    }
  }

  // Build a fragment and append/update
  let prevTr = null;
  for (const row of rows) {
    let tr = els.rowsBody.querySelector(`tr.row[data-key="${CSS.escape(row.key)}"]`);
    if (!tr) {
      tr = buildRowEl(row);
      if (prevTr) {
        prevTr.after(tr);
      } else {
        els.rowsBody.prepend(tr);
      }
    } else {
      updateRowEl(tr, row);
    }
    prevTr = tr;
    // Keep detail row after its parent if expanded
    if (state.expanded.has(row.key) && prevTr) {
      const detailTr = prevTr.nextElementSibling;
      if (detailTr && detailTr.classList.contains('detail-row')) {
        prevTr = detailTr;
      }
    }
  }
}

function totalEvents() {
  let n = 0;
  for (const r of state.rows.values()) n += r.count;
  return n;
}

function buildRowEl(row) {
  const tr = document.createElement('tr');
  tr.className = 'row';
  tr.dataset.key = row.key;
  tr.dataset.class = row.classification;
  tr.innerHTML = rowHTML(row);
  tr.addEventListener('click', () => toggleExpand(row.key, tr));
  return tr;
}

function updateRowEl(tr, row) {
  tr.dataset.class = row.classification;
  tr.innerHTML = rowHTML(row);
  tr.addEventListener('click', () => toggleExpand(row.key, tr));
}

function rowHTML(row) {
  return `
    <td title="${escHtml(row.key)}">${escHtml(shortKey(row.key))}</td>
    <td>${row.count}</td>
    <td>${row.visible_count || 0}</td>
    <td>${formatRelative(row.last_seen)}</td>
    <td>${formatDuration(row.total_console_time_ms || 0)}</td>
    <td>${escHtml(row.classification)}</td>
  `;
}

function shortKey(key) {
  const parts = key.split('<-');
  if (parts.length > 4) {
    return parts[0] + '<-…<-' + parts.slice(-2).join('<-');
  }
  return key;
}

async function toggleExpand(chainKey, tr) {
  if (state.expanded.has(chainKey)) {
    collapseRow(chainKey, tr);
  } else {
    await expandRow(chainKey, tr);
  }
}

async function expandRow(chainKey, tr) {
  state.expanded.add(chainKey);
  try {
    const res = await fetch('/api/events?chain=' + encodeURIComponent(chainKey) + '&limit=50');
    if (!res.ok) return;
    const events = await res.json();

    const template = document.getElementById('detail-template');
    const clone = template.content.cloneNode(true);

    clone.querySelector('.detail-chain').textContent = chainKey;
    clone.querySelector('.detail-count').textContent = events.length + ' events shown';

    const tbody = clone.querySelector('.detail-events-body');
    for (const ev of events) {
      const td = document.createElement('tr');
      td.innerHTML = `
        <td>${formatRelative(ev.spawned_at)}</td>
        <td>${ev.lifetime_ms != null ? formatDuration(ev.lifetime_ms) : '—'}</td>
        <td>${ev.exit_code != null ? ev.exit_code : '—'}</td>
        <td>${escHtml(ev.process && ev.process.subsystem || '—')}</td>
        <td>${escHtml(ev.process && ev.process.stdio && ev.process.stdio.stdout || '—')}</td>
        <td class="cmdline">${escHtml(ev.process && ev.process.command_line || ev.process && ev.process.name || '—')}</td>
      `;
      tbody.appendChild(td);
    }

    const detailTr = document.createElement('tr');
    detailTr.className = 'detail-row';
    const td = document.createElement('td');
    td.colSpan = 6;
    td.appendChild(clone);
    detailTr.appendChild(td);
    tr.after(detailTr);
  } catch (e) {
    console.error('expandRow failed', e);
    state.expanded.delete(chainKey);
  }
}

function collapseRow(chainKey, tr) {
  state.expanded.delete(chainKey);
  const detailTr = tr.nextElementSibling;
  if (detailTr && detailTr.classList.contains('detail-row')) {
    detailTr.remove();
  }
}

function formatRelative(timestamp) {
  if (!timestamp) return '—';
  const ms = Date.now() - new Date(timestamp).getTime();
  if (ms < 0) return 'just now';
  if (ms < 2000) return 'just now';
  if (ms < 60_000) return Math.floor(ms / 1000) + 's ago';
  if (ms < 3_600_000) return Math.floor(ms / 60_000) + 'm ago';
  return Math.floor(ms / 3_600_000) + 'h ago';
}

function formatDuration(ms) {
  if (ms == null || ms < 0) return '—';
  if (ms < 1000) return ms + 'ms';
  if (ms < 60_000) return (ms / 1000).toFixed(1) + 's';
  return Math.floor(ms / 60_000) + 'm';
}

async function refreshHealth() {
  try {
    const res = await fetch('/api/health');
    if (!res.ok) return;
    const h = await res.json();
    if (els.statusEtw) {
      els.statusEtw.textContent = 'ETW: ' + (h.etw_session_active ? 'active' : 'inactive (view mode)');
      els.statusEtw.dataset.state = h.etw_session_active ? 'ok' : 'warn';
    }
    if (els.statusEvents) {
      els.statusEvents.textContent = h.total_events + ' events';
    }
    if (els.statusRows) {
      els.statusRows.textContent = h.aggregator_rows + ' chains';
    }
  } catch (_) {
    if (els.statusEtw) {
      els.statusEtw.textContent = 'ETW: unknown';
      els.statusEtw.dataset.state = 'bad';
    }
  }
}

function escHtml(s) {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

init();
