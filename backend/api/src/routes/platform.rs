use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audit,
    auth::{password, session, session::AuthenticatedPlatformAdmin},
    error::AppError,
    state::AppState,
};

fn platform_session_cookie(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((session::PLATFORM_SESSION_COOKIE_NAME, token))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .build()
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct PlatformLoginRequest {
    pub email: String,
    pub password: String,
}

/// Logs a platform admin in and sets the (separately-named) platform
/// session cookie.
#[utoipa::path(
    post,
    path = "/api/platform/auth/login",
    tag = "platform",
    request_body = PlatformLoginRequest,
    responses(
        (status = 200, description = "Logged in, platform session cookie set"),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
    )
)]
pub async fn platform_login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<PlatformLoginRequest>,
) -> Result<CookieJar, AppError> {
    let admin = entity::prelude::PlatformAdmin::find()
        .filter(entity::platform_admin::Column::Email.eq(&req.email))
        .one(&state.admin_db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !password::verify_password(&req.password, &admin.password_hash) {
        return Err(AppError::Unauthorized);
    }

    let token = session::create_platform_session(&state.admin_db, admin.id).await?;
    Ok(jar.add(platform_session_cookie(token, state.cookie_secure)))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct TenantSummary {
    pub id: Uuid,
    pub name: String,
    pub status: String,
}

/// Tenant name/status only — a platform manager oversees tenant lifecycle,
/// not tenant business data (leads, quotations, financials). That boundary
/// is deliberate, not an oversight: nothing in routes::platform ever
/// touches a tenant-scoped table.
#[utoipa::path(
    get,
    path = "/api/platform/tenants",
    tag = "platform",
    responses(
        (status = 200, description = "List tenants (name/status only)", body = Vec<TenantSummary>),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
    )
)]
pub async fn list_tenants(
    State(state): State<AppState>,
    _admin: AuthenticatedPlatformAdmin,
) -> Result<Json<Vec<TenantSummary>>, AppError> {
    let tenants = entity::prelude::Tenant::find().all(&state.admin_db).await?;
    Ok(Json(
        tenants
            .into_iter()
            .map(|t| TenantSummary {
                id: t.id,
                name: t.name,
                status: t.status,
            })
            .collect(),
    ))
}

/// Every pause/resume/delete is written to the *target tenant's* audit_log
/// (via admin_db, which bypasses RLS — no tenant context to SET LOCAL here
/// since a platform admin isn't a member of any tenant) — this is exactly
/// the kind of high-privilege action the project's traceability priority
/// exists for. Attributed as `Actor::System` with the platform_admin_id
/// recorded in the payload rather than a queryable actor column: adding a
/// third actor-attribution FK for a rare, high-privilege action wasn't
/// judged worth a further schema change here.
async fn transition_tenant_status(
    state: &AppState,
    admin_id: Uuid,
    tenant_id: Uuid,
    from_allowed: &[&str],
    to: &str,
) -> Result<entity::tenant::Model, AppError> {
    let from_allowed: Vec<String> = from_allowed.iter().map(|s| s.to_string()).collect();
    let to = to.to_string();
    state
        .admin_db
        .transaction::<_, entity::tenant::Model, AppError>(|txn| {
            Box::pin(async move {
                let tenant = entity::prelude::Tenant::find_by_id(tenant_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if !from_allowed.contains(&tenant.status) {
                    return Err(AppError::BadRequest(format!(
                        "cannot transition tenant from '{}' to '{}'",
                        tenant.status, to
                    )));
                }
                let before = serde_json::json!({ "status": tenant.status });

                let mut am: entity::tenant::ActiveModel = tenant.into();
                am.status = Set(to.clone());
                match to.as_str() {
                    "paused" => am.paused_at = Set(Some(chrono::Utc::now().into())),
                    "deleted" => am.deleted_at = Set(Some(chrono::Utc::now().into())),
                    "active" => am.paused_at = Set(None),
                    _ => {}
                }
                let updated = am.update(txn).await?;

                audit::record(
                    txn,
                    tenant_id,
                    "tenant",
                    tenant_id,
                    "update",
                    audit::Actor::System,
                    Some(before),
                    Some(serde_json::json!({ "status": to, "platform_admin_id": admin_id })),
                )
                .await?;

                Ok(updated)
            })
        })
        .await
        .map_err(crate::error::map_txn_err)
}

#[utoipa::path(
    post,
    path = "/api/platform/tenants/{id}/pause",
    tag = "platform",
    params(("id" = Uuid, Path, description = "Tenant id")),
    responses(
        (status = 200, description = "Tenant paused", body = entity::tenant::Model),
        (status = 400, description = "bad request", body = crate::error::ErrorResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn pause_tenant(
    State(state): State<AppState>,
    admin: AuthenticatedPlatformAdmin,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<entity::tenant::Model>, AppError> {
    let tenant = transition_tenant_status(
        &state,
        admin.platform_admin_id,
        tenant_id,
        &["active"],
        "paused",
    )
    .await?;
    Ok(Json(tenant))
}

#[utoipa::path(
    post,
    path = "/api/platform/tenants/{id}/resume",
    tag = "platform",
    params(("id" = Uuid, Path, description = "Tenant id")),
    responses(
        (status = 200, description = "Tenant resumed", body = entity::tenant::Model),
        (status = 400, description = "bad request", body = crate::error::ErrorResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn resume_tenant(
    State(state): State<AppState>,
    admin: AuthenticatedPlatformAdmin,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<entity::tenant::Model>, AppError> {
    let tenant = transition_tenant_status(
        &state,
        admin.platform_admin_id,
        tenant_id,
        &["paused"],
        "active",
    )
    .await?;
    Ok(Json(tenant))
}

/// Soft delete only — sets status/deleted_at, never DROPs tenant data. No
/// "undelete" endpoint: deletion is meant to be more final than pause,
/// matching real-world SaaS convention. Terminal from either active or
/// paused.
#[utoipa::path(
    post,
    path = "/api/platform/tenants/{id}/delete",
    tag = "platform",
    params(("id" = Uuid, Path, description = "Tenant id")),
    responses(
        (status = 200, description = "Tenant soft-deleted", body = entity::tenant::Model),
        (status = 400, description = "bad request", body = crate::error::ErrorResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn delete_tenant(
    State(state): State<AppState>,
    admin: AuthenticatedPlatformAdmin,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<entity::tenant::Model>, AppError> {
    let tenant = transition_tenant_status(
        &state,
        admin.platform_admin_id,
        tenant_id,
        &["active", "paused"],
        "deleted",
    )
    .await?;
    Ok(Json(tenant))
}
