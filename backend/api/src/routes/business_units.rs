use axum::{extract::State, Json};
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
pub struct CreateBusinessUnitRequest {
    pub name: String,
}

pub async fn create_business_unit(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateBusinessUnitRequest>,
) -> Result<Json<entity::business_unit::Model>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let name = req.name.clone();

    let model = state
        .app_db
        .transaction::<_, entity::business_unit::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let am = entity::business_unit::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    name: Set(name.clone()),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "business_unit",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "name": name })),
                )
                .await?;
                Ok(model)
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(model))
}

pub async fn list_business_units(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<entity::business_unit::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::business_unit::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let items = entity::prelude::BusinessUnit::find().all(txn).await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(items))
}
