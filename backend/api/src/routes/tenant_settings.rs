use axum::{extract::State, Json};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, TransactionTrait};
use serde::{Deserialize, Serialize};

use crate::{
    audit,
    auth::session::AuthenticatedUser,
    authz,
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

/// Tenant-level configuration, narrowed scope — region_profile (existed as
/// a column, never had an API) and workstream_labels. Deliberately NOT
/// configurable approval chains, which needs the generic Approval Workflow
/// entity architecture.md describes — a separate, comparably-sized
/// undertaking. See .ai/decisions/current/
/// 2026-08-28-phase-3-audit-and-expansion.md.
#[derive(Serialize)]
pub struct TenantSettingsResponse {
    pub region_profile: String,
    pub workstream_labels: serde_json::Value,
}

pub async fn get_tenant_settings(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<TenantSettingsResponse>, AppError> {
    let tenant_id = user.tenant_id;
    let tenant = state
        .app_db
        .transaction::<_, entity::tenant::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                entity::prelude::Tenant::find_by_id(tenant_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(TenantSettingsResponse {
        region_profile: tenant.region_profile,
        workstream_labels: tenant.workstream_labels,
    }))
}

#[derive(Deserialize)]
pub struct UpdateTenantSettingsRequest {
    pub region_profile: Option<String>,
    pub workstream_labels: Option<serde_json::Value>,
}

pub async fn update_tenant_settings(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<UpdateTenantSettingsRequest>,
) -> Result<Json<TenantSettingsResponse>, AppError> {
    authz::require_tenant_admin(user)?;
    if let Some(rp) = &req.region_profile {
        if rp != "india" {
            return Err(AppError::BadRequest(
                "region_profile: only 'india' is implemented".into(),
            ));
        }
    }
    let tenant_id = user.tenant_id;

    let tenant = state
        .app_db
        .transaction::<_, entity::tenant::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let existing = entity::prelude::Tenant::find_by_id(tenant_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let before = serde_json::json!({
                    "region_profile": existing.region_profile,
                    "workstream_labels": existing.workstream_labels,
                });
                let mut am: entity::tenant::ActiveModel = existing.into();
                if let Some(rp) = req.region_profile.clone() {
                    am.region_profile = Set(rp);
                }
                if let Some(labels) = req.workstream_labels.clone() {
                    am.workstream_labels = Set(labels);
                }
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "tenant",
                    tenant_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({
                        "region_profile": updated.region_profile,
                        "workstream_labels": updated.workstream_labels,
                    })),
                )
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(TenantSettingsResponse {
        region_profile: tenant.region_profile,
        workstream_labels: tenant.workstream_labels,
    }))
}
