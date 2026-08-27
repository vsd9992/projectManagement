use axum::{
    extract::{Path, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    audit,
    auth::session::AuthenticatedClientUser,
    db::set_tenant,
    error::{map_txn_err, AppError},
    routes::design::DesignAssetWithRevisions,
    state::AppState,
};

/// Lists only the projects belonging to this client's own `client_id` — RLS
/// alone would return every project in the tenant, so this filter is what
/// actually keeps a client scoped to their own data (see
/// .ai/decisions/current/2026-08-27-auth-session-based-single-login.md).
pub async fn list_my_projects(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
) -> Result<Json<Vec<entity::project::Model>>, AppError> {
    let tenant_id = client.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::project::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let items = entity::prelude::Project::find()
                    .filter(entity::project::Column::ClientId.eq(client.client_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

pub async fn list_project_design_assets(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<DesignAssetWithRevisions>>, AppError> {
    let tenant_id = client.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<DesignAssetWithRevisions>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let project = entity::prelude::Project::find_by_id(project_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if project.client_id != client.client_id {
                    return Err(AppError::NotFound);
                }

                let assets = entity::prelude::DesignAsset::find()
                    .filter(entity::design_asset::Column::ProjectId.eq(project_id))
                    .all(txn)
                    .await?;

                let mut result = Vec::with_capacity(assets.len());
                for asset in assets {
                    let revisions = entity::prelude::DesignRevision::find()
                        .filter(entity::design_revision::Column::DesignAssetId.eq(asset.id))
                        .all(txn)
                        .await?;
                    result.push(DesignAssetWithRevisions { asset, revisions });
                }
                Ok(result)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[derive(Deserialize)]
pub struct DecisionRequest {
    pub notes: Option<String>,
}

async fn decide_revision(
    state: &AppState,
    client: AuthenticatedClientUser,
    revision_id: Uuid,
    approve: bool,
    notes: Option<String>,
) -> Result<entity::design_revision::Model, AppError> {
    let tenant_id = client.tenant_id;

    state
        .app_db
        .transaction::<_, entity::design_revision::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let revision = entity::prelude::DesignRevision::find_by_id(revision_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let asset = entity::prelude::DesignAsset::find_by_id(revision.design_asset_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let project = entity::prelude::Project::find_by_id(asset.project_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if project.client_id != client.client_id {
                    return Err(AppError::NotFound);
                }
                if revision.status != "submitted" {
                    return Err(AppError::BadRequest(format!(
                        "revision is already {}",
                        revision.status
                    )));
                }

                let before = serde_json::json!({ "status": revision.status });
                let new_status = if approve { "approved" } else { "rejected" };

                let mut am: entity::design_revision::ActiveModel = revision.into();
                am.status = Set(new_status.to_string());
                am.decided_by = Set(Some(client.client_user_id));
                am.decided_at = Set(Some(chrono::Utc::now().into()));
                am.decision_notes = Set(notes.clone());
                let updated = am.update(txn).await?;

                audit::record(
                    txn,
                    tenant_id,
                    "design_revision",
                    revision_id,
                    "update",
                    audit::Actor::ClientUser(client.client_user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": new_status, "decided_by_client_user": client.client_user_id, "notes": notes })),
                )
                .await?;

                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)
}

pub async fn approve_design_revision(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(revision_id): Path<Uuid>,
    Json(req): Json<DecisionRequest>,
) -> Result<Json<entity::design_revision::Model>, AppError> {
    let model = decide_revision(&state, client, revision_id, true, req.notes).await?;
    Ok(Json(model))
}

pub async fn reject_design_revision(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(revision_id): Path<Uuid>,
    Json(req): Json<DecisionRequest>,
) -> Result<Json<entity::design_revision::Model>, AppError> {
    let model = decide_revision(&state, client, revision_id, false, req.notes).await?;
    Ok(Json(model))
}
