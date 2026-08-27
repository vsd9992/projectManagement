use axum::{
    extract::{Path, State},
    Json,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    audit,
    auth::session::AuthenticatedUser,
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateLeadRequest {
    pub business_unit_id: Uuid,
    pub client_id: Uuid,
    pub title: String,
}

pub async fn create_lead(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<entity::lead::Model>, AppError> {
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
        .transaction::<_, entity::lead::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
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

pub async fn list_leads(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<entity::lead::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::lead::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                Ok(entity::prelude::Lead::find().all(txn).await?)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[derive(Deserialize)]
pub struct ConvertLeadRequest {
    pub project_name: String,
}

/// Converts a lead into a Project (using the lead's business unit + client)
/// and marks the lead converted, in one transaction.
pub async fn convert_lead(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(lead_id): Path<Uuid>,
    Json(req): Json<ConvertLeadRequest>,
) -> Result<Json<entity::project::Model>, AppError> {
    if req.project_name.trim().is_empty() {
        return Err(AppError::BadRequest("project_name is required".into()));
    }
    let tenant_id = user.tenant_id;
    let project_name = req.project_name.clone();

    let project = state
        .app_db
        .transaction::<_, entity::project::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let lead = entity::prelude::Lead::find_by_id(lead_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if lead.status == "converted" {
                    return Err(AppError::BadRequest("lead is already converted".into()));
                }

                let project_id = Uuid::new_v4();
                let project_am = entity::project::ActiveModel {
                    id: Set(project_id),
                    tenant_id: Set(tenant_id),
                    business_unit_id: Set(lead.business_unit_id),
                    client_id: Set(lead.client_id),
                    name: Set(project_name.clone()),
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
                    Some(serde_json::json!({ "name": project_name, "converted_from_lead": lead_id })),
                )
                .await?;

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

                Ok(project)
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(project))
}
