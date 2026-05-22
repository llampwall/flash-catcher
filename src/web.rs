use crate::aggregate::{Aggregator, BlameChainRow, SortBy};
use crate::event::FlashEvent;
use crate::store::Store;
use anyhow::Result;
use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::{Json, Router};
use futures::stream::Stream;
use parking_lot::Mutex;
use serde::Deserialize;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

static INDEX_HTML: &str = include_str!("../web/index.html");
static APP_JS: &str = include_str!("../web/app.js");
static STYLES_CSS: &str = include_str!("../web/styles.css");

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub aggregator: Arc<Mutex<Aggregator>>,
    pub etw_active: Arc<AtomicBool>,
    pub started_at: std::time::Instant,
    pub recent_events: Arc<Mutex<VecDeque<FlashEvent>>>,
}

const MAX_RECENT_EVENTS: usize = 2000;

impl AppState {
    pub fn new(store: Arc<Store>, aggregator: Arc<Mutex<Aggregator>>, etw_active: bool) -> Self {
        Self {
            store,
            aggregator,
            etw_active: Arc::new(AtomicBool::new(etw_active)),
            started_at: std::time::Instant::now(),
            recent_events: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_RECENT_EVENTS))),
        }
    }

    pub fn push_recent_event(&self, event: FlashEvent) {
        let mut ring = self.recent_events.lock();
        if ring.len() >= MAX_RECENT_EVENTS {
            ring.pop_front();
        }
        ring.push_back(event);
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", axum::routing::get(index_handler))
        .route("/static/app.js", axum::routing::get(app_js_handler))
        .route("/static/styles.css", axum::routing::get(styles_css_handler))
        .route("/api/rows", axum::routing::get(rows_handler))
        .route("/api/events/:event_id", axum::routing::get(event_detail_handler))
        .route("/api/events", axum::routing::get(events_for_chain_handler))
        .route("/api/stream", axum::routing::get(sse_stream_handler))
        .route("/api/health", axum::routing::get(health_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn serve(state: AppState, addr: &str) -> Result<()> {
    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("bind {}: {}", addr, e))?;

    tracing::info!("Dashboard at http://{}/", addr);

    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("ctrl-c signal");
            tracing::info!("Shutting down");
        })
        .await
        .map_err(|e| anyhow::anyhow!("server error: {}", e))
}

pub async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}

async fn app_js_handler() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        APP_JS,
    )
}

async fn styles_css_handler() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        STYLES_CSS,
    )
}

#[derive(Debug, Deserialize)]
pub struct RowsQuery {
    sort: Option<String>,
}

pub async fn rows_handler(
    State(state): State<AppState>,
    Query(params): Query<RowsQuery>,
) -> Json<Vec<BlameChainRow>> {
    let sort = parse_sort(params.sort.as_deref());
    let rows = state.aggregator.lock().snapshot(sort);
    Json(rows)
}

#[derive(Debug, Deserialize)]
pub struct ChainQuery {
    chain: Option<String>,
    limit: Option<usize>,
}

pub async fn event_detail_handler(
    State(state): State<AppState>,
    Path(event_id): Path<String>,
) -> Json<Option<FlashEvent>> {
    let ring = state.recent_events.lock();
    let found = ring.iter().find(|e| e.event_id == event_id).cloned();
    Json(found)
}

pub async fn events_for_chain_handler(
    State(state): State<AppState>,
    Query(params): Query<ChainQuery>,
) -> Json<Vec<FlashEvent>> {
    let chain_key = params.chain.unwrap_or_default();
    let limit = params.limit.unwrap_or(50).min(200);

    let ring = state.recent_events.lock();
    let events: Vec<FlashEvent> = ring
        .iter()
        .filter(|e| e.blame.key == chain_key)
        .rev()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    Json(events)
}

pub async fn sse_stream_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.store.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => serde_json::to_string(&event)
            .ok()
            .map(|json| Ok(Event::default().data(json))),
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}

pub async fn health_handler(State(state): State<AppState>) -> Json<HealthReport> {
    let etw_session_active = state.etw_active.load(Ordering::Relaxed);
    let total_events = state.store.total_events.load(Ordering::Relaxed);
    let aggregator_rows = state.aggregator.lock().row_count();
    let uptime_seconds = state.started_at.elapsed().as_secs();
    let store_path = state
        .store
        .data_dir
        .to_string_lossy()
        .into_owned();

    Json(HealthReport {
        etw_session_active,
        store_path,
        total_events,
        aggregator_rows,
        uptime_seconds,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthReport {
    pub etw_session_active: bool,
    pub store_path: String,
    pub total_events: u64,
    pub aggregator_rows: usize,
    pub uptime_seconds: u64,
}

pub fn parse_sort(raw: Option<&str>) -> SortBy {
    match raw {
        Some("highest-count") => SortBy::HighestCount,
        Some("longest-lifetime") => SortBy::LongestLifetime,
        _ => SortBy::MostRecent,
    }
}
