use axum::{
    extract::{Path, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    audit,
    auth::session::AuthenticatedUser,
    authz,
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

// ---- Site Tasks ----

const TASK_STATUSES: [&str; 3] = ["not_started", "in_progress", "done"];

#[derive(Deserialize)]
pub struct CreateSiteTaskRequest {
    pub title: String,
}

pub async fn create_site_task(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateSiteTaskRequest>,
) -> Result<Json<entity::site_task::Model>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let title = req.title.clone();

    let model = state
        .app_db
        .transaction::<_, entity::site_task::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
                    project_id,
                    Some("delivery"),
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
                Ok(model)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

pub async fn list_site_tasks(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::site_task::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::site_task::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::SiteTask::find()
                    .filter(entity::site_task::Column::ProjectId.eq(project_id))
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
pub struct UpdateStatusRequest {
    pub status: String,
}

pub async fn update_site_task_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<entity::site_task::Model>, AppError> {
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
        .transaction::<_, entity::site_task::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let task = entity::prelude::SiteTask::find_by_id(task_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
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
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

#[derive(Deserialize)]
pub struct AddDependencyRequest {
    pub depends_on_task_id: Uuid,
}

/// Declares that `task_id` depends on `depends_on_task_id` — the explicit
/// cross-task link mechanism from architecture.md (workstreams progress
/// concurrently; dependencies are declared links, not an assumed stage
/// order). Both tasks must belong to the same project.
pub async fn add_site_task_dependency(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(req): Json<AddDependencyRequest>,
) -> Result<Json<entity::site_task_dependency::Model>, AppError> {
    if task_id == req.depends_on_task_id {
        return Err(AppError::BadRequest(
            "a task cannot depend on itself".into(),
        ));
    }
    let tenant_id = user.tenant_id;
    let depends_on_task_id = req.depends_on_task_id;

    let model = state
        .app_db
        .transaction::<_, entity::site_task_dependency::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let task = entity::prelude::SiteTask::find_by_id(task_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let dep = entity::prelude::SiteTask::find_by_id(depends_on_task_id)
                    .one(txn)
                    .await?
                    .ok_or_else(|| AppError::BadRequest("depends_on_task_id not found".into()))?;
                if task.project_id != dep.project_id {
                    return Err(AppError::BadRequest(
                        "both tasks must belong to the same project".into(),
                    ));
                }
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
                    task.project_id,
                    Some("delivery"),
                )
                .await?;

                let am = entity::site_task_dependency::ActiveModel {
                    tenant_id: Set(tenant_id),
                    task_id: Set(task_id),
                    depends_on_task_id: Set(depends_on_task_id),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "site_task_dependency",
                    task_id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "task_id": task_id, "depends_on_task_id": depends_on_task_id })),
                )
                .await?;
                Ok(model)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

pub async fn list_site_task_dependencies(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
) -> Result<Json<Vec<entity::site_task_dependency::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::site_task_dependency::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let task = entity::prelude::SiteTask::find_by_id(task_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
                    task.project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::SiteTaskDependency::find()
                    .filter(entity::site_task_dependency::Column::TaskId.eq(task_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

// ---- Daily Logs ----

#[derive(Deserialize)]
pub struct CreateDailyLogRequest {
    pub log_date: chrono::NaiveDate,
    pub notes: String,
}

pub async fn create_daily_log(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateDailyLogRequest>,
) -> Result<Json<entity::daily_log::Model>, AppError> {
    if req.notes.trim().is_empty() {
        return Err(AppError::BadRequest("notes is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let notes = req.notes.clone();

    let model = state
        .app_db
        .transaction::<_, entity::daily_log::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
                    project_id,
                    Some("delivery"),
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

pub async fn list_daily_logs(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::daily_log::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::daily_log::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::DailyLog::find()
                    .filter(entity::daily_log::Column::ProjectId.eq(project_id))
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

#[derive(Deserialize)]
pub struct CreatePunchListItemRequest {
    pub description: String,
}

pub async fn create_punch_list_item(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreatePunchListItemRequest>,
) -> Result<Json<entity::punch_list_item::Model>, AppError> {
    if req.description.trim().is_empty() {
        return Err(AppError::BadRequest("description is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let description = req.description.clone();

    let model = state
        .app_db
        .transaction::<_, entity::punch_list_item::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
                    project_id,
                    Some("delivery"),
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

pub async fn list_punch_list_items(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::punch_list_item::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::punch_list_item::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::PunchListItem::find()
                    .filter(entity::punch_list_item::Column::ProjectId.eq(project_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

pub async fn close_punch_list_item(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(item_id): Path<Uuid>,
) -> Result<Json<entity::punch_list_item::Model>, AppError> {
    let tenant_id = user.tenant_id;
    let model = state
        .app_db
        .transaction::<_, entity::punch_list_item::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let item = entity::prelude::PunchListItem::find_by_id(item_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
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

#[derive(Deserialize)]
pub struct CreateSiteQueryRequest {
    pub subject: String,
    pub question: String,
}

pub async fn create_site_query(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateSiteQueryRequest>,
) -> Result<Json<entity::site_query::Model>, AppError> {
    if req.subject.trim().is_empty() || req.question.trim().is_empty() {
        return Err(AppError::BadRequest("subject and question are required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let subject = req.subject.clone();

    let model = state
        .app_db
        .transaction::<_, entity::site_query::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
                    project_id,
                    Some("delivery"),
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

pub async fn list_site_queries(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::site_query::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::site_query::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::SiteQuery::find()
                    .filter(entity::site_query::Column::ProjectId.eq(project_id))
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
pub struct AnswerSiteQueryRequest {
    pub answer: String,
}

pub async fn answer_site_query(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(query_id): Path<Uuid>,
    Json(req): Json<AnswerSiteQueryRequest>,
) -> Result<Json<entity::site_query::Model>, AppError> {
    if req.answer.trim().is_empty() {
        return Err(AppError::BadRequest("answer is required".into()));
    }
    let tenant_id = user.tenant_id;
    let answer = req.answer.clone();

    let model = state
        .app_db
        .transaction::<_, entity::site_query::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let query = entity::prelude::SiteQuery::find_by_id(query_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user.user_id,
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
