mod auth;
mod business_units;
mod change_orders;
mod client_portal;
mod clients;
pub mod design;
mod leads;
mod projects;
mod quotations;

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
        .route("/auth/client-login", post(auth::client_login))
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
            "/clients/:client_id/users",
            post(clients::create_client_user),
        )
        .route(
            "/leads",
            get(leads::list_leads).post(leads::create_lead),
        )
        .route("/leads/:id/convert", post(leads::convert_lead))
        .route(
            "/projects",
            get(projects::list_projects).post(projects::create_project),
        )
        .route("/projects/:id", get(projects::get_project))
        .route(
            "/projects/:project_id/quotations",
            get(quotations::list_quotations).post(quotations::create_quotation),
        )
        .route("/quotations/:id", get(quotations::get_quotation))
        .route(
            "/projects/:project_id/change-orders",
            get(change_orders::list_change_orders).post(change_orders::create_change_order),
        )
        .route(
            "/change-orders/:id",
            get(change_orders::get_change_order),
        )
        .route(
            "/projects/:project_id/design-assets",
            get(design::list_design_assets).post(design::create_design_asset),
        )
        .route(
            "/design-assets/:id/revisions",
            get(design::list_design_revisions).post(design::submit_design_revision),
        )
        .route("/client/projects", get(client_portal::list_my_projects))
        .route(
            "/client/projects/:project_id/design-assets",
            get(client_portal::list_project_design_assets),
        )
        .route(
            "/client/design-revisions/:id/approve",
            post(client_portal::approve_design_revision),
        )
        .route(
            "/client/design-revisions/:id/reject",
            post(client_portal::reject_design_revision),
        )
        .route(
            "/client/quotations/:id/approve",
            post(client_portal::approve_quotation),
        )
        .route(
            "/client/quotations/:id/reject",
            post(client_portal::reject_quotation),
        )
        .route(
            "/client/projects/:project_id/change-orders",
            get(client_portal::list_my_change_orders),
        )
        .route(
            "/client/change-orders/:id/approve",
            post(client_portal::approve_change_order),
        )
        .route(
            "/client/change-orders/:id/reject",
            post(client_portal::reject_change_order),
        )
}

async fn health() -> &'static str {
    "ok"
}
