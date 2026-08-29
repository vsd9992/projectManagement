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
    error::{map_txn_err, AppError, ErrorResponse},
    state::AppState,
};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateLeadRequest {
    pub business_unit_id: Uuid,
    pub client_id: Uuid,
    pub title: String,
}

#[utoipa::path(
    post,
    path = "/api/leads",
    tag = "leads",
    request_body = CreateLeadRequest,
    responses(
        (status = 200, description = "Lead created", body = LeadModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_lead(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<LeadModel>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let title = req.title.clone();
    let business_unit_id = req.business_unit_id;
    let client_id = req.client_id;

    let model = state
        .app_db
        .transaction::<_, LeadModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_business_unit_role(
                    txn,
                    user,
                    business_unit_id,
                    Some("sales_design"),
                )
                .await?;
                let am = entity::lead::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    business_unit_id: Set(business_unit_id),
                    client_id: Set(client_id),
                    title: Set(title.clone()),
                    status: Set("new".to_string()),
                    converted_project_id: Set(None),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "lead",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "title": title, "business_unit_id": business_unit_id, "client_id": client_id })),
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
    path = "/api/leads",
    tag = "leads",
    responses(
        (status = 200, description = "List leads", body = Vec<LeadModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
    )
)]
pub async fn list_leads(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<LeadModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<LeadModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let bu_ids =
                    authz::accessible_business_units(txn, user, Some("sales_design"))
                        .await?;
                Ok(entity::prelude::Lead::find()
                    .filter(entity::lead::Column::BusinessUnitId.is_in(bu_ids))
                    .all(txn)
                    .await?)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ConvertLeadRequest {
    pub project_name: String,
    /// Same requirement as direct project creation: an arbitrary, non-empty
    /// subset of the workstream catalog. Found missing during M6 scenario
    /// verification — conversion previously produced a project with zero
    /// enabled workstreams (no validation caught it, since project creation
    /// itself is the only place "at least one workstream" was enforced).
    pub workstreams: Vec<WorkstreamType>,
    /// "milestone" (default) or "progressive" — see projects::CreateProjectRequest.
    #[serde(default = "default_billing_method")]
    pub billing_method: String,
}

fn default_billing_method() -> String {
    "milestone".to_string()
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ConvertedProjectResponse {
    #[serde(flatten)]
    pub project: ProjectModel,
    pub workstreams: Vec<ProjectWorkstreamModel>,
}

/// Converts a lead into a Project (using the lead's business unit + client)
/// and marks the lead converted, in one transaction.
#[utoipa::path(
    post,
    path = "/api/leads/{id}/convert",
    tag = "leads",
    params(("id" = Uuid, Path, description = "Lead id")),
    request_body = ConvertLeadRequest,
    responses(
        (status = 200, description = "Lead converted to project", body = ConvertedProjectResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn convert_lead(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(lead_id): Path<Uuid>,
    Json(req): Json<ConvertLeadRequest>,
) -> Result<Json<ConvertedProjectResponse>, AppError> {
    if req.project_name.trim().is_empty() {
        return Err(AppError::BadRequest("project_name is required".into()));
    }
    if req.workstreams.is_empty() {
        return Err(AppError::BadRequest(
            "at least one workstream must be enabled".into(),
        ));
    }
    if req.billing_method != "milestone" && req.billing_method != "progressive" {
        return Err(AppError::BadRequest(
            "billing_method must be 'milestone' or 'progressive'".into(),
        ));
    }
    let tenant_id = user.tenant_id;
    let project_name = req.project_name.clone();
    let workstream_types = req.workstreams.clone();
    let billing_method = req.billing_method.clone();

    let (project, workstreams) = state
        .app_db
        .transaction::<_, (ProjectModel, Vec<ProjectWorkstreamModel>), AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let lead = entity::prelude::Lead::find_by_id(lead_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if lead.status == "converted" {
                    return Err(AppError::BadRequest("lead is already converted".into()));
                }
                authz::require_business_unit_role(
                    txn,
                    user,
                    lead.business_unit_id,
                    Some("sales_design"),
                )
                .await?;

                let project_id = Uuid::new_v4();
                let project_am = entity::project::ActiveModel {
                    id: Set(project_id),
                    tenant_id: Set(tenant_id),
                    business_unit_id: Set(lead.business_unit_id),
                    client_id: Set(lead.client_id),
                    name: Set(project_name.clone()),
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
                        "name": project_name,
                        "converted_from_lead": lead_id,
                        "workstreams": workstream_types,
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
                        Some(serde_json::json!({ "project_id": project_id, "workstream_type": wt })),
                    )
                    .await?;
                    workstreams.push(ws_model);
                }

                let mut lead_am: entity::lead::ActiveModel = lead.into();
                lead_am.status = Set("converted".to_string());
                lead_am.converted_project_id = Set(Some(project_id));
                lead_am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "lead",
                    lead_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(serde_json::json!({ "status": "new_or_qualified" })),
                    Some(serde_json::json!({ "status": "converted", "converted_project_id": project_id })),
                )
                .await?;

                Ok((project, workstreams))
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(ConvertedProjectResponse {
        project,
        workstreams,
    }))
}

use entity::lead::Model as LeadModel;
use entity::project::Model as ProjectModel;
use entity::project_workstream::Model as ProjectWorkstreamModel;
