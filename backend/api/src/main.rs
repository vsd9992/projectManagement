mod audit;
mod auth;
mod billing;
mod config;
mod db;
mod error;
mod routes;
mod state;

use sea_orm::Database;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use config::AppConfig;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = AppConfig::from_env()?;

    let app_db = Database::connect(&config.database_url_app).await?;
    let admin_db = Database::connect(&config.database_url_admin).await?;
    let state = AppState { app_db, admin_db };

    let app = routes::router()
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "listening");
    axum::serve(listener, app).await?;

    Ok(())
}
