use axum::{
    extract::{Path, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audit,
    auth::session::AuthenticatedUser,
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateDesignAssetRequest {
    pub title: String,
}

pub async fn create_design_asset(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateDesignAssetRequest>,
) -> Result<Json<entity::design_asset::Model>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let title = req.title.clone();

    let model = state
        .app_db
        .transaction::<_, entity::design_asset::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                if entity::prelude::Project::find_by_id(project_id).one(txn).await?.is_none() {
                    return Err(AppError::NotFound);
                }
                let am = entity::design_asset::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    title: Set(title.clone()),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "design_asset",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "project_id": project_id, "title": title })),
                )
                .await?;
                Ok(model)
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(model))
}

pub async fn list_design_assets(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::design_asset::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::design_asset::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let items = entity::prelude::DesignAsset::find()
                    .filter(entity::design_asset::Column::ProjectId.eq(project_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[derive(Deserialize)]
pub struct SubmitRevisionRequest {
    pub notes: Option<String>,
}

/// Submits a new versioned revision under a design asset. Starts life as
/// "submitted" — client approval/rejection is a separate action via the
/// Client Portal endpoints, not something the submitting internal user sets.
pub async fn submit_design_revision(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(design_asset_id): Path<Uuid>,
    Json(req): Json<SubmitRevisionRequest>,
) -> Result<Json<entity::design_revision::Model>, AppError> {
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let notes = req.notes.clone();

    let model = state
        .app_db
        .transaction::<_, entity::design_revision::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                if entity::prelude::DesignAsset::find_by_id(design_asset_id).one(txn).await?.is_none() {
                    return Err(AppError::NotFound);
                }

                let next_version = entity::prelude::DesignRevision::find()
                    .filter(entity::design_revision::Column::DesignAssetId.eq(design_asset_id))
                    .order_by_desc(entity::design_revision::Column::Version)
                    .one(txn)
                    .await?
                    .map(|r| r.version + 1)
                    .unwrap_or(1);

                let am = entity::design_revision::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    design_asset_id: Set(design_asset_id),
                    version: Set(next_version),
                    notes: Set(notes.clone()),
                    status: Set("submitted".to_string()),
                    submitted_by: Set(user.user_id),
                    submitted_at: Set(chrono::Utc::now().into()),
                    decided_by: Set(None),
                    decided_at: Set(None),
                    decision_notes: Set(None),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "design_revision",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "design_asset_id": design_asset_id, "version": next_version })),
                )
                .await?;
                Ok(model)
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(model))
}

pub async fn list_design_revisions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(design_asset_id): Path<Uuid>,
) -> Result<Json<Vec<entity::design_revision::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::design_revision::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let items = entity::prelude::DesignRevision::find()
                    .filter(entity::design_revision::Column::DesignAssetId.eq(design_asset_id))
                    .order_by_asc(entity::design_revision::Column::Version)
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[derive(Serialize)]
pub struct DesignAssetWithRevisions {
    #[serde(flatten)]
    pub asset: entity::design_asset::Model,
    pub revisions: Vec<entity::design_revision::Model>,
}
