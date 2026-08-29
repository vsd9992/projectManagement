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
    authz,
    db::set_tenant,
    error::{map_txn_err, AppError, ErrorResponse},
    state::AppState,
};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateDesignAssetRequest {
    pub title: String,
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/design-assets",
    tag = "design",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreateDesignAssetRequest,
    responses(
        (status = 200, description = "Design asset created", body = DesignAssetModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_design_asset(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateDesignAssetRequest>,
) -> Result<Json<DesignAssetModel>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let title = req.title.clone();

    let model = state
        .app_db
        .transaction::<_, DesignAssetModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("sales_design"),
                )
                .await?;
                authz::require_project_workstream(
                    txn,
                    project_id,
                    entity::workstream_type::WorkstreamType::Design,
                )
                .await?;
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

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/design-assets",
    tag = "design",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "List design assets", body = Vec<DesignAssetModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn list_design_assets(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<DesignAssetModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<DesignAssetModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("sales_design"),
                )
                .await?;
                let items = entity::prelude::DesignAsset::find()
                    .filter(entity::design_asset::Column::ProjectId.eq(project_id))
                    .order_by_asc(entity::design_asset::Column::CreatedAt)
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SubmitRevisionRequest {
    pub notes: Option<String>,
}

/// Submits a new versioned revision under a design asset. Starts life as
/// "submitted" — client approval/rejection is a separate action via the
/// Client Portal endpoints, not something the submitting internal user sets.
#[utoipa::path(
    post,
    path = "/api/design-assets/{id}/revisions",
    tag = "design",
    params(("id" = Uuid, Path, description = "Design asset id")),
    request_body = SubmitRevisionRequest,
    responses(
        (status = 200, description = "Revision submitted", body = DesignRevisionModel),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn submit_design_revision(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(design_asset_id): Path<Uuid>,
    Json(req): Json<SubmitRevisionRequest>,
) -> Result<Json<DesignRevisionModel>, AppError> {
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let notes = req.notes.clone();

    let model = state
        .app_db
        .transaction::<_, DesignRevisionModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let asset = entity::prelude::DesignAsset::find_by_id(design_asset_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    asset.project_id,
                    Some("sales_design"),
                )
                .await?;

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

#[utoipa::path(
    get,
    path = "/api/design-assets/{id}/revisions",
    tag = "design",
    params(("id" = Uuid, Path, description = "Design asset id")),
    responses(
        (status = 200, description = "List design revisions", body = Vec<DesignRevisionModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn list_design_revisions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(design_asset_id): Path<Uuid>,
) -> Result<Json<Vec<DesignRevisionModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<DesignRevisionModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let asset = entity::prelude::DesignAsset::find_by_id(design_asset_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    asset.project_id,
                    Some("sales_design"),
                )
                .await?;
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

#[derive(Serialize, utoipa::ToSchema)]
pub struct DesignAssetWithRevisions {
    #[serde(flatten)]
    pub asset: DesignAssetModel,
    pub revisions: Vec<DesignRevisionModel>,
}

use entity::design_asset::Model as DesignAssetModel;
use entity::design_revision::Model as DesignRevisionModel;
