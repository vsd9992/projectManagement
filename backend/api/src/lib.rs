pub mod audit;
pub mod auth;
pub mod authz;
pub mod billing;
pub mod config;
pub mod db;
pub mod error;
pub mod notifications;
pub mod routes;
pub mod state;

use axum::Router;
use tower_http::trace::TraceLayer;

use state::AppState;

/// Builds the full Axum app (routes + middleware) for a given `AppState`.
/// Shared by `main.rs` (real server, real DB pools) and integration tests
/// (test DB pools, in-process requests via `tower::ServiceExt::oneshot`) so
/// tests exercise the exact same router as production, not a reimplementation.
pub fn build_app(state: AppState) -> Router {
    routes::router()
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
