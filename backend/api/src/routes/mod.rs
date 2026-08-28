mod auth;
mod billing;
mod business_units;
mod change_orders;
mod client_portal;
mod clients;
pub mod design;
mod leads;
mod manufacturing;
mod notifications;
mod platform;
mod procurement;
mod projects;
mod quotations;
mod schedule;
mod site_execution;

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
        .route("/users", post(auth::create_teammate))
        .route(
            "/users/:id/revoke-sessions",
            post(auth::revoke_user_sessions),
        )
        .route("/users/:id/admin", post(auth::set_tenant_admin))
        .route("/platform/auth/login", post(platform::platform_login))
        .route("/platform/tenants", get(platform::list_tenants))
        .route("/platform/tenants/:id/pause", post(platform::pause_tenant))
        .route(
            "/platform/tenants/:id/resume",
            post(platform::resume_tenant),
        )
        .route(
            "/platform/tenants/:id/delete",
            post(platform::delete_tenant),
        )
        .route(
            "/business-units",
            get(business_units::list_business_units).post(business_units::create_business_unit),
        )
        .route(
            "/business-units/:id/roles",
            get(business_units::list_roles).post(business_units::assign_role),
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
        // Procurement (internal-facing only in MVP — no vendor portal).
        .route(
            "/vendors",
            get(procurement::list_vendors).post(procurement::create_vendor),
        )
        .route(
            "/projects/:project_id/purchase-orders",
            get(procurement::list_purchase_orders).post(procurement::create_purchase_order),
        )
        .route(
            "/purchase-orders/:id/deliver",
            post(procurement::mark_purchase_order_delivered),
        )
        .route(
            "/purchase-orders/:id/approve",
            post(procurement::approve_purchase_order),
        )
        .route(
            "/purchase-orders/:id/reject",
            post(procurement::reject_purchase_order),
        )
        // Manufacturing (simplified "production task" depth for MVP).
        .route(
            "/projects/:project_id/production-tasks",
            get(manufacturing::list_production_tasks).post(manufacturing::create_production_task),
        )
        .route(
            "/production-tasks/:id/status",
            post(manufacturing::update_production_task_status),
        )
        // Site Execution.
        .route(
            "/projects/:project_id/site-tasks",
            get(site_execution::list_site_tasks).post(site_execution::create_site_task),
        )
        .route(
            "/site-tasks/:id/status",
            post(site_execution::update_site_task_status),
        )
        .route(
            "/projects/:project_id/schedule-tasks",
            get(schedule::list_schedule_tasks).post(schedule::create_schedule_task),
        )
        .route(
            "/schedule-tasks/:id/status",
            post(schedule::update_schedule_task_status),
        )
        .route(
            "/schedule-tasks/:id/dates",
            post(schedule::update_schedule_task_dates),
        )
        .route(
            "/schedule-tasks/:id/dependencies",
            get(schedule::list_schedule_task_dependencies)
                .post(schedule::add_schedule_task_dependency),
        )
        .route(
            "/notifications",
            get(notifications::list_my_notifications),
        )
        .route(
            "/notifications/:id/read",
            post(notifications::mark_notification_read),
        )
        .route(
            "/projects/:project_id/daily-logs",
            get(site_execution::list_daily_logs).post(site_execution::create_daily_log),
        )
        .route(
            "/projects/:project_id/punch-list",
            get(site_execution::list_punch_list_items).post(site_execution::create_punch_list_item),
        )
        .route(
            "/punch-list/:id/close",
            post(site_execution::close_punch_list_item),
        )
        .route(
            "/projects/:project_id/site-queries",
            get(site_execution::list_site_queries).post(site_execution::create_site_query),
        )
        .route(
            "/site-queries/:id/answer",
            post(site_execution::answer_site_query),
        )
        // Billing.
        .route(
            "/projects/:project_id/milestones",
            get(billing::list_milestones).post(billing::create_milestone),
        )
        .route(
            "/milestones/:id/complete",
            post(billing::complete_milestone),
        )
        .route(
            "/projects/:project_id/invoices",
            get(billing::list_invoices).post(billing::create_invoice),
        )
        .route("/invoices/:id/mark-paid", post(billing::mark_invoice_paid))
        .route(
            "/client/projects/:project_id/invoices",
            get(client_portal::list_project_invoices),
        )
}

async fn health() -> &'static str {
    "ok"
}
