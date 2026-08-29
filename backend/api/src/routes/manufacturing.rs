use axum::{
    extract::{Path, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    audit,
    auth::session::AuthenticatedUser,
    authz,
    db::set_tenant,
    error::{map_txn_err, AppError, ErrorResponse},
    state::AppState,
};

const VALID_STATUSES: [&str; 3] = ["not_started", "in_progress", "completed"];

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateProductionTaskRequest {
    pub title: String,
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/production-tasks",
    tag = "manufacturing",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreateProductionTaskRequest,
    responses(
        (status = 200, description = "Production task created", body = ProductionTaskModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_production_task(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateProductionTaskRequest>,
) -> Result<Json<ProductionTaskModel>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let title = req.title.clone();

    let model = state
        .app_db
        .transaction::<_, ProductionTaskModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                authz::require_project_workstream(
                    txn,
                    project_id,
                    entity::workstream_type::WorkstreamType::Manufacturing,
                )
                .await?;
                let am = entity::production_task::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    title: Set(title.clone()),
                    status: Set("not_started".to_string()),
                    created_by: Set(user.user_id),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "production_task",
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
    path = "/api/projects/{project_id}/production-tasks",
    tag = "manufacturing",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "List production tasks", body = Vec<ProductionTaskModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn list_production_tasks(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<ProductionTaskModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<ProductionTaskModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::ProductionTask::find()
                    .filter(entity::production_task::Column::ProjectId.eq(project_id))
                    .order_by_asc(entity::production_task::Column::CreatedAt)
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
pub struct ProductionTaskUpdateStatusRequest {
    pub status: String,
}

#[utoipa::path(
    post,
    path = "/api/production-tasks/{id}/status",
    tag = "manufacturing",
    params(("id" = Uuid, Path, description = "Production task id")),
    request_body = ProductionTaskUpdateStatusRequest,
    responses(
        (status = 200, description = "Status updated", body = ProductionTaskModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn update_production_task_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(req): Json<ProductionTaskUpdateStatusRequest>,
) -> Result<Json<ProductionTaskModel>, AppError> {
    if !VALID_STATUSES.contains(&req.status.as_str()) {
        return Err(AppError::BadRequest(format!(
            "status must be one of {:?}",
            VALID_STATUSES
        )));
    }
    let tenant_id = user.tenant_id;
    let new_status = req.status.clone();

    let model = state
        .app_db
        .transaction::<_, ProductionTaskModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let task = entity::prelude::ProductionTask::find_by_id(task_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    task.project_id,
                    Some("delivery"),
                )
                .await?;
                let before = serde_json::json!({ "status": task.status });
                let mut am: entity::production_task::ActiveModel = task.into();
                am.status = Set(new_status.clone());
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "production_task",
                    task_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": new_status })),
                )
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

use entity::production_task::Model as ProductionTaskModel;
