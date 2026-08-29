use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Duration, NaiveDate};
use std::collections::{HashSet, VecDeque};
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
    error::{map_txn_err, AppError, ErrorResponse},
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateScheduleTaskRequest {
    pub title: String,
    pub workstream_type: WorkstreamType,
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/schedule-tasks",
    tag = "schedule",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreateScheduleTaskRequest,
    responses(
        (status = 200, description = "Standalone schedule task created", body = ScheduleTaskModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_schedule_task(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateScheduleTaskRequest>,
) -> Result<Json<ScheduleTaskModel>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let title = req.title.clone();
    let workstream_type = req.workstream_type.clone();

    let model = state
        .app_db
        .transaction::<_, ScheduleTaskModel, AppError>(|txn| {
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

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/schedule-tasks",
    tag = "schedule",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "List schedule tasks", body = Vec<ScheduleTaskModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn list_schedule_tasks(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<ScheduleTaskModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<ScheduleTaskModel>, AppError>(|txn| {
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateScheduleTaskStatusRequest {
    pub status: String,
}

#[utoipa::path(
    post,
    path = "/api/schedule-tasks/{id}/status",
    tag = "schedule",
    params(("id" = Uuid, Path, description = "Schedule task id")),
    request_body = UpdateScheduleTaskStatusRequest,
    responses(
        (status = 200, description = "Status updated", body = ScheduleTaskModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn update_schedule_task_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(req): Json<UpdateScheduleTaskStatusRequest>,
) -> Result<Json<ScheduleTaskModel>, AppError> {
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
        .transaction::<_, ScheduleTaskModel, AppError>(|txn| {
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateDatesRequest {
    pub planned_start_date: Option<NaiveDate>,
    pub planned_end_date: Option<NaiveDate>,
    pub actual_start_date: Option<NaiveDate>,
    pub actual_end_date: Option<NaiveDate>,
}

/// A task's "effective end" — its actual completion date if recorded,
/// otherwise its plan. Comparing this before/after an update is what
/// decides whether a change is a real schedule slip worth propagating
/// (moving later), not just an edit that happens to touch the date fields.
fn effective_end(task: &ScheduleTaskModel) -> Option<NaiveDate> {
    task.actual_end_date.or(task.planned_end_date)
}

/// Full-replace of a schedule task's 4 date fields (no partial-PATCH
/// semantics — matches this codebase's convention elsewhere). If the
/// task's effective end moved later, triggers basic forward-pass
/// propagation to dependents (see propagate_shift) and returns which
/// tasks were shifted, for the caller to notify in a later stage.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct UpdateDatesResponse {
    #[serde(flatten)]
    pub task: ScheduleTaskModel,
    pub shifted_dependent_task_ids: Vec<Uuid>,
}

#[utoipa::path(
    post,
    path = "/api/schedule-tasks/{id}/dates",
    tag = "schedule",
    params(("id" = Uuid, Path, description = "Schedule task id")),
    request_body = UpdateDatesRequest,
    responses(
        (status = 200, description = "Dates updated; may cascade a forward-pass shift to dependents", body = UpdateDatesResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn update_schedule_task_dates(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(req): Json<UpdateDatesRequest>,
) -> Result<Json<UpdateDatesResponse>, AppError> {
    let tenant_id = user.tenant_id;

    let (task, shifted_dependent_task_ids) = state
        .app_db
        .transaction::<_, (ScheduleTaskModel, Vec<Uuid>), AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let existing = entity::prelude::ScheduleTask::find_by_id(task_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    existing.project_id,
                    Some(role_for_workstream(&existing.workstream_type)),
                )
                .await?;
                let before = serde_json::json!({
                    "planned_start_date": existing.planned_start_date,
                    "planned_end_date": existing.planned_end_date,
                    "actual_start_date": existing.actual_start_date,
                    "actual_end_date": existing.actual_end_date,
                });
                let old_effective_end = effective_end(&existing);

                let mut am: entity::schedule_task::ActiveModel = existing.into();
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

                let new_effective_end = effective_end(&updated);
                let mut shifted = Vec::new();
                if let (Some(old_end), Some(new_end)) = (old_effective_end, new_effective_end) {
                    if new_end > old_end {
                        let delta_days = (new_end - old_end).num_days();
                        shifted =
                            propagate_shift(txn, tenant_id, user.user_id, task_id, delta_days)
                                .await?;

                        // Notify the project team about shifted tasks that
                        // haven't started yet — a task already underway or
                        // finished doesn't need a "your schedule moved"
                        // alert. This *is* the significance threshold: a
                        // task only appears in `shifted` when propagate_shift
                        // actually moved it, not a separate heuristic.
                        for shifted_id in &shifted {
                            let shifted_task = entity::prelude::ScheduleTask::find_by_id(*shifted_id)
                                .one(txn)
                                .await?
                                .ok_or(AppError::NotFound)?;
                            if shifted_task.status != "done" && shifted_task.actual_start_date.is_none() {
                                let message = format!(
                                    "Schedule task '{}' shifted to start {} due to a dependency delay.",
                                    shifted_task.title,
                                    shifted_task
                                        .planned_start_date
                                        .map(|d| d.to_string())
                                        .unwrap_or_else(|| "an unspecified date".to_string()),
                                );
                                crate::notifications::notify_project_team(
                                    txn,
                                    tenant_id,
                                    updated.project_id,
                                    *shifted_id,
                                    &message,
                                )
                                .await?;
                            }
                        }
                    }
                }

                Ok((updated, shifted))
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(UpdateDatesResponse { task, shifted_dependent_task_ids }))
}

/// Basic forward-pass propagation (explicitly not full CPM/critical-path —
/// see .ai/decisions/current/2026-08-28-phase-3-audit-and-expansion.md): a
/// BFS from `root_id` over direct dependents at each level. A dependent
/// shifts (by the same fixed `delta_days` throughout the cascade) only if
/// its planned start can no longer follow its precedent's new effective
/// end — if it already had enough slack, it absorbs the delta and the
/// cascade stops there, rather than propagating further through it. Cycle
/// detection at edge-insert time already guarantees the dependency graph
/// is a DAG, so `visited` here is purely to avoid double-shifting a shared
/// descendant in a diamond-shaped graph, not to prevent infinite loops.
async fn propagate_shift(
    txn: &DatabaseTransaction,
    tenant_id: Uuid,
    user_id: Uuid,
    root_id: Uuid,
    delta_days: i64,
) -> Result<Vec<Uuid>, AppError> {
    let mut shifted_ids = Vec::new();
    let mut visited: HashSet<Uuid> = HashSet::new();
    visited.insert(root_id);
    let mut queue: VecDeque<Uuid> = VecDeque::new();
    queue.push_back(root_id);

    while let Some(precedent_id) = queue.pop_front() {
        let precedent = entity::prelude::ScheduleTask::find_by_id(precedent_id)
            .one(txn)
            .await?
            .ok_or(AppError::NotFound)?;
        let Some(precedent_end) = effective_end(&precedent) else {
            continue;
        };

        let edges = entity::prelude::ScheduleTaskDependency::find()
            .filter(entity::schedule_task_dependency::Column::DependsOnTaskId.eq(precedent_id))
            .all(txn)
            .await?;

        for edge in edges {
            let dep_id = edge.task_id;
            if visited.contains(&dep_id) {
                continue;
            }
            visited.insert(dep_id);

            let dep_task = entity::prelude::ScheduleTask::find_by_id(dep_id)
                .one(txn)
                .await?
                .ok_or(AppError::NotFound)?;

            let needs_shift = matches!(dep_task.planned_start_date, Some(start) if start < precedent_end);
            if !needs_shift {
                continue;
            }

            let before = serde_json::json!({
                "planned_start_date": dep_task.planned_start_date,
                "planned_end_date": dep_task.planned_end_date,
            });
            let new_start = dep_task.planned_start_date.map(|d| d + Duration::days(delta_days));
            let new_end = dep_task.planned_end_date.map(|d| d + Duration::days(delta_days));
            let mut am: entity::schedule_task::ActiveModel = dep_task.into();
            am.planned_start_date = Set(new_start);
            am.planned_end_date = Set(new_end);
            am.update(txn).await?;
            audit::record(
                txn,
                tenant_id,
                "schedule_task",
                dep_id,
                "update",
                audit::Actor::User(user_id),
                Some(before),
                Some(serde_json::json!({
                    "planned_start_date": new_start,
                    "planned_end_date": new_end,
                    "shifted_due_to_precedent": precedent_id,
                })),
            )
            .await?;

            shifted_ids.push(dep_id);
            queue.push_back(dep_id);
        }
    }

    Ok(shifted_ids)
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AddScheduleDependencyRequest {
    pub depends_on_task_id: Uuid,
}

#[utoipa::path(
    post,
    path = "/api/schedule-tasks/{id}/dependencies",
    tag = "schedule",
    params(("id" = Uuid, Path, description = "Schedule task id (the dependent)")),
    request_body = AddScheduleDependencyRequest,
    responses(
        (status = 200, description = "Dependency edge added", body = ScheduleTaskDependencyModel),
        (status = 400, description = "bad request (self-dependency, cross-project, or would create a cycle)", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn add_schedule_task_dependency(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(req): Json<AddScheduleDependencyRequest>,
) -> Result<Json<ScheduleTaskDependencyModel>, AppError> {
    if task_id == req.depends_on_task_id {
        return Err(AppError::BadRequest(
            "a task cannot depend on itself".into(),
        ));
    }
    let tenant_id = user.tenant_id;
    let depends_on_task_id = req.depends_on_task_id;

    let model = state
        .app_db
        .transaction::<_, ScheduleTaskDependencyModel, AppError>(|txn| {
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

#[utoipa::path(
    get,
    path = "/api/schedule-tasks/{id}/dependencies",
    tag = "schedule",
    params(("id" = Uuid, Path, description = "Schedule task id")),
    responses(
        (status = 200, description = "List dependency edges for this task", body = Vec<ScheduleTaskDependencyModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn list_schedule_task_dependencies(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
) -> Result<Json<Vec<ScheduleTaskDependencyModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<ScheduleTaskDependencyModel>, AppError>(|txn| {
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

use entity::schedule_task::Model as ScheduleTaskModel;
use entity::schedule_task_dependency::Model as ScheduleTaskDependencyModel;
