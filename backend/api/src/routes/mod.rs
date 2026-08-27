mod auth;
mod business_units;
mod clients;
mod projects;

use axum::{
    routing::{get, post},
    Router,
};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/auth/signup", post(auth::signup))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route(
            "/business-units",
            get(business_units::list_business_units).post(business_units::create_business_unit),
        )
        .route(
            "/clients",
            get(clients::list_clients).post(clients::create_client),
        )
        .route(
            "/projects",
            get(projects::list_projects).post(projects::create_project),
        )
        .route("/projects/:id", get(projects::get_project))
}

async fn health() -> &'static str {
    "ok"
}
