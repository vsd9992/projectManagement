use axum::{
    extract::{Path, State},
    Json,
};
use entity::workstream_type::WorkstreamType;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audit,
    auth::session::AuthenticatedUser,
    authz,
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateProjectRequest {
    pub name: String,
    pub business_unit_id: Uuid,
    pub client_id: Uuid,
    /// Arbitrary, non-empty subset of the workstream catalog — this is the
    /// mechanism that lets one Project entity represent a pure-manufacturing
    /// job, a pure-civil job, or a full turnkey blend without a rigid
    /// project-type enum (see .ai/decisions/current/2026-08-27-composable-workstream-project-model.md).
    pub workstreams: Vec<WorkstreamType>,
    /// "milestone" (default) or "progressive" (RA-bill-style) — which
    /// billing method this project's invoices use. See .ai/decisions/
    /// current/2026-08-28-phase-3-audit-and-expansion.md.
    #[serde(default = "default_billing_method")]
    pub billing_method: String,
}

fn default_billing_method() -> String {
    "milestone".to_string()
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ProjectResponse {
    #[serde(flatten)]
    pub project: entity::project::Model,
    pub workstreams: Vec<entity::project_workstream::Model>,
}

#[utoipa::path(
    post,
    path = "/api/projects",
    tag = "projects",
    request_body = CreateProjectRequest,
    responses(
        (status = 200, description = "Project created", body = ProjectResponse),
        (status = 400, description = "bad request", body = crate::error::ErrorResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
    )
)]
pub async fn create_project(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateProjectRequest>,
) -> Result<Json<ProjectResponse>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    if req.workstreams.is_empty() {
        return Err(AppError::BadRequest(
            "at least one workstream must be enabled".into(),
        ));
    }

    let tenant_id = user.tenant_id;
    let project_id = Uuid::new_v4();
    let name = req.name.clone();
    let business_unit_id = req.business_unit_id;
    let client_id = req.client_id;
    let workstream_types = req.workstreams.clone();
    if req.billing_method != "milestone" && req.billing_method != "progressive" {
        return Err(AppError::BadRequest(
            "billing_method must be 'milestone' or 'progressive'".into(),
        ));
    }
    let billing_method = req.billing_method.clone();

    let (project, workstreams) = state
        .app_db
        .transaction::<_, (entity::project::Model, Vec<entity::project_workstream::Model>), AppError>(
            |txn| {
                Box::pin(async move {
                    set_tenant(txn, tenant_id).await?;
                    authz::require_business_unit_role(txn, user, business_unit_id, None)
                        .await?;

                    let project_am = entity::project::ActiveModel {
                        id: Set(project_id),
                        tenant_id: Set(tenant_id),
                        business_unit_id: Set(business_unit_id),
                        client_id: Set(client_id),
                        name: Set(name.clone()),
                        billing_method: Set(billing_method.clone()),
                        created_at: Set(chrono::Utc::now().into()),
                    };
                    let project = project_am.insert(txn).await?;
                    audit::record(
                        txn,
                        tenant_id,
                        "project",
                        project_id,
                        "create",
                        audit::Actor::User(user.user_id),
                        None,
                        Some(serde_json::json!({
                            "name": name,
                            "business_unit_id": business_unit_id,
                            "client_id": client_id,
                            "billing_method": billing_method,
                        })),
                    )
                    .await?;

                    let mut workstreams = Vec::with_capacity(workstream_types.len());
                    for wt in workstream_types {
                        let ws_id = Uuid::new_v4();
                        let ws_am = entity::project_workstream::ActiveModel {
                            id: Set(ws_id),
                            tenant_id: Set(tenant_id),
                            project_id: Set(project_id),
                            workstream_type: Set(wt.clone()),
                            status: Set("not_started".to_string()),
                            created_at: Set(chrono::Utc::now().into()),
                        };
                        let ws_model = ws_am.insert(txn).await?;
                        audit::record(
                            txn,
                            tenant_id,
                            "project_workstream",
                            ws_id,
                            "create",
                            audit::Actor::User(user.user_id),
                            None,
                            Some(serde_json::json!({
                                "project_id": project_id,
                                "workstream_type": wt,
                            })),
                        )
                        .await?;
                        workstreams.push(ws_model);
                    }

                    Ok((project, workstreams))
                })
            },
        )
        .await
        .map_err(map_txn_err)?;

    Ok(Json(ProjectResponse {
        project,
        workstreams,
    }))
}

#[utoipa::path(
    get,
    path = "/api/projects/{id}",
    tag = "projects",
    params(("id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "Project detail", body = ProjectResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn get_project(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ProjectResponse>, AppError> {
    let tenant_id = user.tenant_id;

    let result = state
        .app_db
        .transaction::<_, Option<(entity::project::Model, Vec<entity::project_workstream::Model>)>, AppError>(
            |txn| {
                Box::pin(async move {
                    set_tenant(txn, tenant_id).await?;
                    let project = entity::prelude::Project::find_by_id(project_id)
                        .one(txn)
                        .await?;
                    let Some(project) = project else {
                        return Ok(None);
                    };
                    authz::require_business_unit_role(
                        txn,
                        user,
                        project.business_unit_id,
                        None,
                    )
                    .await?;
                    let workstreams = entity::prelude::ProjectWorkstream::find()
                        .filter(entity::project_workstream::Column::ProjectId.eq(project_id))
                        .all(txn)
                        .await?;
                    Ok(Some((project, workstreams)))
                })
            },
        )
        .await
        .map_err(map_txn_err)?;

    let (project, workstreams) = result.ok_or(AppError::NotFound)?;
    Ok(Json(ProjectResponse {
        project,
        workstreams,
    }))
}

#[utoipa::path(
    get,
    path = "/api/projects",
    tag = "projects",
    responses(
        (status = 200, description = "List projects", body = Vec<entity::project::Model>),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
    )
)]
pub async fn list_projects(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<entity::project::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::project::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let bu_ids = authz::accessible_business_units(txn, user, None).await?;
                let items = entity::prelude::Project::find()
                    .filter(entity::project::Column::BusinessUnitId.is_in(bu_ids))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(items))
}
