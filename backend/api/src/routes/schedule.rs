use axum::{
    extract::{Path, State},
    Json,
};
use chrono::NaiveDate;
use entity::workstream_type::WorkstreamType;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, Set, Statement, TransactionTrait,
};
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

const TASK_STATUSES: [&str; 3] = ["not_started", "in_progress", "done"];

fn role_for_workstream(wt: &WorkstreamType) -> &'static str {
    match wt {
        WorkstreamType::Design => "sales_design",
        WorkstreamType::Manufacturing | WorkstreamType::Procurement | WorkstreamType::SiteExecution => {
            "delivery"
        }
    }
}

#[derive(Deserialize)]
pub struct CreateScheduleTaskRequest {
    pub title: String,
    pub workstream_type: WorkstreamType,
}

pub async fn create_schedule_task(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateScheduleTaskRequest>,
) -> Result<Json<entity::schedule_task::Model>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let title = req.title.clone();
    let workstream_type = req.workstream_type.clone();

    let model = state
        .app_db
        .transaction::<_, entity::schedule_task::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some(role_for_workstream(&workstream_type)),
                )
                .await?;
                authz::require_project_workstream(txn, project_id, workstream_type.clone()).await?;

                let am = entity::schedule_task::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    workstream_type: Set(workstream_type.clone()),
                    title: Set(title.clone()),
                    status: Set("not_started".to_string()),
                    planned_start_date: Set(None),
                    planned_end_date: Set(None),
                    actual_start_date: Set(None),
                    actual_end_date: Set(None),
                    site_task_id: Set(None),
                    production_task_id: Set(None),
                    design_revision_id: Set(None),
                    purchase_order_id: Set(None),
                    spawned_by_change_order_id: Set(None),
                    created_by: Set(user.user_id),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "schedule_task",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "project_id": project_id, "title": title, "workstream_type": workstream_type })),
                )
                .await?;
                Ok(model)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

pub async fn list_schedule_tasks(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::schedule_task::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::schedule_task::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(txn, user, project_id, None).await?;
                let items = entity::prelude::ScheduleTask::find()
                    .filter(entity::schedule_task::Column::ProjectId.eq(project_id))
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
pub struct UpdateScheduleTaskStatusRequest {
    pub status: String,
}

pub async fn update_schedule_task_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(req): Json<UpdateScheduleTaskStatusRequest>,
) -> Result<Json<entity::schedule_task::Model>, AppError> {
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
        .transaction::<_, entity::schedule_task::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let task = entity::prelude::ScheduleTask::find_by_id(task_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    task.project_id,
                    Some(role_for_workstream(&task.workstream_type)),
                )
                .await?;
                let before = serde_json::json!({ "status": task.status });
                let mut am: entity::schedule_task::ActiveModel = task.into();
                am.status = Set(new_status.clone());
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "schedule_task",
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
pub struct UpdateDatesRequest {
    pub planned_start_date: Option<NaiveDate>,
    pub planned_end_date: Option<NaiveDate>,
    pub actual_start_date: Option<NaiveDate>,
    pub actual_end_date: Option<NaiveDate>,
}

/// Full-replace of a schedule task's 4 date fields (no partial-PATCH
/// semantics — matches this codebase's convention elsewhere). Basic
/// forward-pass date-shift propagation to dependents happens here in a
/// later Phase 3 stage; for now this only updates the triggering task.
pub async fn update_schedule_task_dates(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(req): Json<UpdateDatesRequest>,
) -> Result<Json<entity::schedule_task::Model>, AppError> {
    let tenant_id = user.tenant_id;

    let model = state
        .app_db
        .transaction::<_, entity::schedule_task::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let task = entity::prelude::ScheduleTask::find_by_id(task_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    task.project_id,
                    Some(role_for_workstream(&task.workstream_type)),
                )
                .await?;
                let before = serde_json::json!({
                    "planned_start_date": task.planned_start_date,
                    "planned_end_date": task.planned_end_date,
                    "actual_start_date": task.actual_start_date,
                    "actual_end_date": task.actual_end_date,
                });
                let mut am: entity::schedule_task::ActiveModel = task.into();
                am.planned_start_date = Set(req.planned_start_date);
                am.planned_end_date = Set(req.planned_end_date);
                am.actual_start_date = Set(req.actual_start_date);
                am.actual_end_date = Set(req.actual_end_date);
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "schedule_task",
                    task_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({
                        "planned_start_date": req.planned_start_date,
                        "planned_end_date": req.planned_end_date,
                        "actual_start_date": req.actual_start_date,
                        "actual_end_date": req.actual_end_date,
                    })),
                )
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

/// Whether adding an edge `task_id -> depends_on_task_id` would close a
/// cycle — i.e. whether `depends_on_task_id` already (transitively)
/// depends on `task_id`. Must run inside the same transaction as the edge
/// insert to see uncommitted state consistently. Uses bound parameters,
/// not string interpolation.
async fn would_create_cycle(
    txn: &DatabaseTransaction,
    task_id: Uuid,
    depends_on_task_id: Uuid,
) -> Result<bool, AppError> {
    let sql = r#"
        WITH RECURSIVE reachable AS (
            SELECT depends_on_task_id AS id FROM schedule_task_dependencies WHERE task_id = $1
            UNION
            SELECT d.depends_on_task_id FROM schedule_task_dependencies d
            JOIN reachable r ON d.task_id = r.id
        )
        SELECT 1 FROM reachable WHERE id = $2 LIMIT 1
    "#;
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [depends_on_task_id.into(), task_id.into()],
    );
    let row = txn.query_one(stmt).await?;
    Ok(row.is_some())
}

#[derive(Deserialize)]
pub struct AddScheduleDependencyRequest {
    pub depends_on_task_id: Uuid,
}

pub async fn add_schedule_task_dependency(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(req): Json<AddScheduleDependencyRequest>,
) -> Result<Json<entity::schedule_task_dependency::Model>, AppError> {
    if task_id == req.depends_on_task_id {
        return Err(AppError::BadRequest(
            "a task cannot depend on itself".into(),
        ));
    }
    let tenant_id = user.tenant_id;
    let depends_on_task_id = req.depends_on_task_id;

    let model = state
        .app_db
        .transaction::<_, entity::schedule_task_dependency::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let task = entity::prelude::ScheduleTask::find_by_id(task_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let dep = entity::prelude::ScheduleTask::find_by_id(depends_on_task_id)
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
                    user,
                    task.project_id,
                    Some(role_for_workstream(&task.workstream_type)),
                )
                .await?;
                if would_create_cycle(txn, task_id, depends_on_task_id).await? {
                    return Err(AppError::BadRequest(
                        "this dependency would create a cycle".into(),
                    ));
                }

                let am = entity::schedule_task_dependency::ActiveModel {
                    tenant_id: Set(tenant_id),
                    task_id: Set(task_id),
                    depends_on_task_id: Set(depends_on_task_id),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "schedule_task_dependency",
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

pub async fn list_schedule_task_dependencies(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
) -> Result<Json<Vec<entity::schedule_task_dependency::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::schedule_task_dependency::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let task = entity::prelude::ScheduleTask::find_by_id(task_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(txn, user, task.project_id, None).await?;
                let items = entity::prelude::ScheduleTaskDependency::find()
                    .filter(entity::schedule_task_dependency::Column::TaskId.eq(task_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}
