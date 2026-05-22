use crate::aggregate::{Aggregator, BlameChainRow, SortBy};
use crate::event::FlashEvent;
use crate::store::Store;
use anyhow::Result;
use axum::Router;
use parking_lot::Mutex;
use std::sync::Arc;

/// Shared application state passed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub aggregator: Arc<Mutex<Aggregator>>,
}

/// Build the axum Router with all endpoints + static file serving.
pub fn build_router(_state: AppState) -> Router {
    unimplemented!("compose: GET /, GET /api/rows, GET /api/events/:id, GET /api/stream (SSE), GET /api/health, static /web/*")
}

/// Bind the server on `addr` and run until shutdown.
pub async fn serve(_state: AppState, _addr: &str) -> Result<()> {
    unimplemented!("tokio::net::TcpListener::bind, axum::serve, graceful Ctrl-C shutdown")
}

// ----- Handlers (stubs) -----

/// `GET /` — serves the embedded `web/index.html`.
pub async fn index_handler() -> &'static str {
    unimplemented!("include_str!(\"../web/index.html\") with Content-Type: text/html")
}

/// `GET /api/rows?sort=most-recent|highest-count|longest-lifetime`
/// Returns the current aggregated landing-view rows.
pub async fn rows_handler() -> axum::Json<Vec<BlameChainRow>> {
    unimplemented!("read query param, aggregator.snapshot(sort), return Json")
}

/// `GET /api/events/:event_id` — full per-spawn detail for the expand pane.
pub async fn event_detail_handler() -> axum::Json<Option<FlashEvent>> {
    unimplemented!("scan store / cache for event by id")
}

/// `GET /api/events?chain=<key>&limit=<n>` — events for one blame-chain row.
pub async fn events_for_chain_handler() -> axum::Json<Vec<FlashEvent>> {
    unimplemented!("filter store events by blame.key == chain, return last `limit`")
}

/// `GET /api/stream` — SSE stream of new events as they're appended to the store.
/// The browser keeps this open and updates rows in place.
pub async fn sse_stream_handler() -> axum::response::Sse<futures::stream::BoxStream<'static, Result<axum::response::sse::Event, std::convert::Infallible>>> {
    unimplemented!("subscribe to store, map each FlashEvent into Sse::Event::json")
}

/// `GET /api/health` — collector + store diagnostics for the UI status bar.
pub async fn health_handler() -> axum::Json<HealthReport> {
    unimplemented!("etw session alive? store path? rows? subscribers?")
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthReport {
    pub etw_session_active: bool,
    pub store_path: String,
    pub total_events: u64,
    pub aggregator_rows: usize,
    pub uptime_seconds: u64,
}

/// Resolve the requested sort axis from a query string value.
pub fn parse_sort(_raw: Option<&str>) -> SortBy {
    unimplemented!("match raw against SortBy variants, default MostRecent")
}
