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

// ---- Site Tasks ----

const TASK_STATUSES: [&str; 3] = ["not_started", "in_progress", "done"];

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateSiteTaskRequest {
    pub title: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SiteTaskResponse {
    #[serde(flatten)]
    pub task: SiteTaskModel,
    /// The linked schedule_tasks row's id — use this (not the site task's
    /// own id) to create dependencies via POST /schedule-tasks/:id/dependencies
    /// or set planned/actual dates via POST /schedule-tasks/:id/dates.
    pub schedule_task_id: Uuid,
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/site-tasks",
    tag = "site_execution",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreateSiteTaskRequest,
    responses(
        (status = 200, description = "Site task created (with linked schedule task)", body = SiteTaskResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_site_task(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateSiteTaskRequest>,
) -> Result<Json<SiteTaskResponse>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let title = req.title.clone();

    let (task, schedule_task_id) = state
        .app_db
        .transaction::<_, (SiteTaskModel, Uuid), AppError>(|txn| {
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
                    entity::workstream_type::WorkstreamType::SiteExecution,
                )
                .await?;
                let am = entity::site_task::ActiveModel {
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
                    "site_task",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "project_id": project_id, "title": title })),
                )
                .await?;

                // Every site task gets a permanently-linked schedule_tasks
                // row — schedule_task_dependencies is the sole place
                // dependency data lives now (see .ai/decisions/current/
                // 2026-08-28-phase-3-audit-and-expansion.md).
                let schedule_task_id = Uuid::new_v4();
                let sched_am = entity::schedule_task::ActiveModel {
                    id: Set(schedule_task_id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    workstream_type: Set(entity::workstream_type::WorkstreamType::SiteExecution),
                    title: Set(title.clone()),
                    status: Set("not_started".to_string()),
                    planned_start_date: Set(None),
                    planned_end_date: Set(None),
                    actual_start_date: Set(None),
                    actual_end_date: Set(None),
                    site_task_id: Set(Some(id)),
                    production_task_id: Set(None),
                    design_revision_id: Set(None),
                    purchase_order_id: Set(None),
                    spawned_by_change_order_id: Set(None),
                    created_by: Set(user.user_id),
                    created_at: Set(chrono::Utc::now().into()),
                };
                sched_am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "schedule_task",
                    schedule_task_id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "project_id": project_id, "title": title, "site_task_id": id })),
                )
                .await?;

                Ok((model, schedule_task_id))
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(SiteTaskResponse { task, schedule_task_id }))
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/site-tasks",
    tag = "site_execution",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "List site tasks", body = Vec<SiteTaskModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn list_site_tasks(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<SiteTaskModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<SiteTaskModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::SiteTask::find()
                    .filter(entity::site_task::Column::ProjectId.eq(project_id))
                    .order_by_asc(entity::site_task::Column::CreatedAt)
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
pub struct SiteTaskUpdateStatusRequest {
    pub status: String,
}

#[utoipa::path(
    post,
    path = "/api/site-tasks/{id}/status",
    tag = "site_execution",
    params(("id" = Uuid, Path, description = "Site task id")),
    request_body = SiteTaskUpdateStatusRequest,
    responses(
        (status = 200, description = "Status updated (kept in sync on the linked schedule task)", body = SiteTaskModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn update_site_task_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(req): Json<SiteTaskUpdateStatusRequest>,
) -> Result<Json<SiteTaskModel>, AppError> {
    if !TASK_STATUSES.contains(&req.status.as_str()) {
        return Err(AppError::BadRequest(format!(
            "status must be one of {:?}",
            TASK_STATUSES
        )));
    }
    let tenant_id = user.tenant_id;
    let new_status = req.status.clone();

    let model = state
        .app_db
        .transaction::<_, SiteTaskModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let task = entity::prelude::SiteTask::find_by_id(task_id)
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
                let mut am: entity::site_task::ActiveModel = task.into();
                am.status = Set(new_status.clone());
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "site_task",
                    task_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": new_status })),
                )
                .await?;

                // Keep the linked schedule_task's status in sync so it
                // doesn't diverge into its own dual-status system.
                if let Some(sched) = entity::prelude::ScheduleTask::find()
                    .filter(entity::schedule_task::Column::SiteTaskId.eq(task_id))
                    .one(txn)
                    .await?
                {
                    let sched_before = serde_json::json!({ "status": sched.status });
                    let sched_id = sched.id;
                    let mut sched_am: entity::schedule_task::ActiveModel = sched.into();
                    sched_am.status = Set(new_status.clone());
                    sched_am.update(txn).await?;
                    audit::record(
                        txn,
                        tenant_id,
                        "schedule_task",
                        sched_id,
                        "update",
                        audit::Actor::User(user.user_id),
                        Some(sched_before),
                        Some(serde_json::json!({ "status": new_status })),
                    )
                    .await?;
                }

                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

// Site-task-to-site-task dependencies used to live here
// (add_site_task_dependency/list_site_task_dependencies). They now live in
// routes::schedule as a generalized graph spanning all four workstreams —
// see .ai/decisions/current/2026-08-28-phase-3-audit-and-expansion.md.
// create_site_task above links every site task to a schedule_task row;
// use its schedule_task_id with POST/GET /schedule-tasks/:id/dependencies.

// ---- Daily Logs ----

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateDailyLogRequest {
    pub log_date: chrono::NaiveDate,
    pub notes: String,
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/daily-logs",
    tag = "site_execution",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreateDailyLogRequest,
    responses(
        (status = 200, description = "Daily log created", body = DailyLogModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_daily_log(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateDailyLogRequest>,
) -> Result<Json<DailyLogModel>, AppError> {
    if req.notes.trim().is_empty() {
        return Err(AppError::BadRequest("notes is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let notes = req.notes.clone();

    let model = state
        .app_db
        .transaction::<_, DailyLogModel, AppError>(|txn| {
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
                    entity::workstream_type::WorkstreamType::SiteExecution,
                )
                .await?;
                let am = entity::daily_log::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    log_date: Set(req.log_date),
                    notes: Set(notes.clone()),
                    logged_by: Set(user.user_id),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "daily_log",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "project_id": project_id, "log_date": req.log_date })),
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
    path = "/api/projects/{project_id}/daily-logs",
    tag = "site_execution",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "List daily logs", body = Vec<DailyLogModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn list_daily_logs(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<DailyLogModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<DailyLogModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::DailyLog::find()
                    .filter(entity::daily_log::Column::ProjectId.eq(project_id))
                    .order_by_asc(entity::daily_log::Column::LogDate)
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

// ---- Punch List ----

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreatePunchListItemRequest {
    pub description: String,
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/punch-list",
    tag = "site_execution",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreatePunchListItemRequest,
    responses(
        (status = 200, description = "Punch list item raised", body = PunchListItemModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_punch_list_item(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreatePunchListItemRequest>,
) -> Result<Json<PunchListItemModel>, AppError> {
    if req.description.trim().is_empty() {
        return Err(AppError::BadRequest("description is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let description = req.description.clone();

    let model = state
        .app_db
        .transaction::<_, PunchListItemModel, AppError>(|txn| {
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
                    entity::workstream_type::WorkstreamType::SiteExecution,
                )
                .await?;
                let am = entity::punch_list_item::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    description: Set(description.clone()),
                    status: Set("open".to_string()),
                    raised_by: Set(user.user_id),
                    raised_at: Set(chrono::Utc::now().into()),
                    closed_by: Set(None),
                    closed_at: Set(None),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "punch_list_item",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "project_id": project_id, "description": description })),
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
    path = "/api/projects/{project_id}/punch-list",
    tag = "site_execution",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "List punch list items", body = Vec<PunchListItemModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn list_punch_list_items(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<PunchListItemModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<PunchListItemModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::PunchListItem::find()
                    .filter(entity::punch_list_item::Column::ProjectId.eq(project_id))
                    .order_by_asc(entity::punch_list_item::Column::RaisedAt)
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[utoipa::path(
    post,
    path = "/api/punch-list/{id}/close",
    tag = "site_execution",
    params(("id" = Uuid, Path, description = "Punch list item id")),
    responses(
        (status = 200, description = "Punch list item closed", body = PunchListItemModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn close_punch_list_item(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(item_id): Path<Uuid>,
) -> Result<Json<PunchListItemModel>, AppError> {
    let tenant_id = user.tenant_id;
    let model = state
        .app_db
        .transaction::<_, PunchListItemModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let item = entity::prelude::PunchListItem::find_by_id(item_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    item.project_id,
                    Some("delivery"),
                )
                .await?;
                if item.status == "closed" {
                    return Err(AppError::BadRequest("punch list item is already closed".into()));
                }
                let before = serde_json::json!({ "status": item.status });
                let mut am: entity::punch_list_item::ActiveModel = item.into();
                am.status = Set("closed".to_string());
                am.closed_by = Set(Some(user.user_id));
                am.closed_at = Set(Some(chrono::Utc::now().into()));
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "punch_list_item",
                    item_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": "closed" })),
                )
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

// ---- Site Queries (basic RFI log) ----

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateSiteQueryRequest {
    pub subject: String,
    pub question: String,
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/site-queries",
    tag = "site_execution",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreateSiteQueryRequest,
    responses(
        (status = 200, description = "Site query raised", body = SiteQueryModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_site_query(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateSiteQueryRequest>,
) -> Result<Json<SiteQueryModel>, AppError> {
    if req.subject.trim().is_empty() || req.question.trim().is_empty() {
        return Err(AppError::BadRequest("subject and question are required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let subject = req.subject.clone();

    let model = state
        .app_db
        .transaction::<_, SiteQueryModel, AppError>(|txn| {
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
                    entity::workstream_type::WorkstreamType::SiteExecution,
                )
                .await?;
                let am = entity::site_query::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    subject: Set(subject.clone()),
                    question: Set(req.question.clone()),
                    status: Set("open".to_string()),
                    raised_by: Set(user.user_id),
                    raised_at: Set(chrono::Utc::now().into()),
                    answer: Set(None),
                    answered_by: Set(None),
                    answered_at: Set(None),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "site_query",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "project_id": project_id, "subject": subject })),
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
    path = "/api/projects/{project_id}/site-queries",
    tag = "site_execution",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "List site queries", body = Vec<SiteQueryModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn list_site_queries(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<SiteQueryModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<SiteQueryModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::SiteQuery::find()
                    .filter(entity::site_query::Column::ProjectId.eq(project_id))
                    .order_by_asc(entity::site_query::Column::RaisedAt)
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
pub struct AnswerSiteQueryRequest {
    pub answer: String,
}

#[utoipa::path(
    post,
    path = "/api/site-queries/{id}/answer",
    tag = "site_execution",
    params(("id" = Uuid, Path, description = "Site query id")),
    request_body = AnswerSiteQueryRequest,
    responses(
        (status = 200, description = "Site query answered", body = SiteQueryModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn answer_site_query(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(query_id): Path<Uuid>,
    Json(req): Json<AnswerSiteQueryRequest>,
) -> Result<Json<SiteQueryModel>, AppError> {
    if req.answer.trim().is_empty() {
        return Err(AppError::BadRequest("answer is required".into()));
    }
    let tenant_id = user.tenant_id;
    let answer = req.answer.clone();

    let model = state
        .app_db
        .transaction::<_, SiteQueryModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let query = entity::prelude::SiteQuery::find_by_id(query_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    query.project_id,
                    Some("delivery"),
                )
                .await?;
                if query.status == "answered" {
                    return Err(AppError::BadRequest("site query is already answered".into()));
                }
                let before = serde_json::json!({ "status": query.status });
                let mut am: entity::site_query::ActiveModel = query.into();
                am.status = Set("answered".to_string());
                am.answer = Set(Some(answer.clone()));
                am.answered_by = Set(Some(user.user_id));
                am.answered_at = Set(Some(chrono::Utc::now().into()));
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "site_query",
                    query_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": "answered", "answer": answer })),
                )
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

use entity::daily_log::Model as DailyLogModel;
use entity::punch_list_item::Model as PunchListItemModel;
use entity::site_query::Model as SiteQueryModel;
use entity::site_task::Model as SiteTaskModel;
