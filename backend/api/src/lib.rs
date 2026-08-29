pub mod audit;
pub mod auth;
pub mod authz;
pub mod billing;
pub mod config;
pub mod db;
pub mod error;
pub mod notifications;
pub mod openapi;
pub mod routes;
pub mod state;

use axum::{http, Router};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use state::AppState;

/// Builds the full Axum app (routes + middleware) for a given `AppState`.
/// Shared by `main.rs` (real server, real DB pools) and integration tests
/// (test DB pools, in-process requests via `tower::ServiceExt::oneshot`) so
/// tests exercise the exact same router as production, not a reimplementation.
///
/// Every route lives under `/api` (`routes::router()` itself stays
/// unprefixed) so a reverse proxy/ingress in front of a future built
/// frontend bundle has one stable rule: `/api/*` to this service, everything
/// else to static files — dev and prod share the same path structure rather
/// than only faking the prefix in a dev-only proxy.
pub fn build_app(state: AppState) -> Router {
    let cors_origin = state.cors_origin.clone();
    // Credentialed CORS (cookies) forbids wildcard `*` on Allow-Headers /
    // Allow-Methods / Allow-Origin — every one of these must be an explicit
    // list, or tower-http panics at startup (caught live on devMachine).
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(cors_origin.parse().expect(
            "CORS_ORIGIN must be a valid header value, e.g. http://192.168.1.4:5173",
        )))
        .allow_credentials(true)
        .allow_methods([http::Method::GET, http::Method::POST])
        .allow_headers([http::header::CONTENT_TYPE]);

    Router::new()
        .nest("/api", routes::router())
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
