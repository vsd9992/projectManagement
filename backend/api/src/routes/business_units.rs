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

const VALID_ROLES: [&str; 3] = ["sales_design", "delivery", "finance"];

#[derive(Deserialize)]
pub struct AssignRoleRequest {
    pub user_id: Uuid,
    pub role: String,
}

/// Assigns `role` to `user_id` for this business unit. Not gated by
/// existing business-unit membership — a brand-new business unit has no
/// members yet, so someone has to be able to make the first assignment.
/// Any authenticated tenant user can assign roles for now; there is no
/// distinct admin/owner role in the catalog to restrict this to (see
/// .ai/decisions/current/2026-08-28-no-rbac-enforcement-yet.md).
pub async fn assign_role(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(business_unit_id): Path<Uuid>,
    Json(req): Json<AssignRoleRequest>,
) -> Result<Json<entity::user_business_unit_role::Model>, AppError> {
    if !VALID_ROLES.contains(&req.role.as_str()) {
        return Err(AppError::BadRequest(format!(
            "role must be one of {:?}",
            VALID_ROLES
        )));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let target_user_id = req.user_id;
    let role = req.role.clone();

    let model = state
        .app_db
        .transaction::<_, entity::user_business_unit_role::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                if entity::prelude::BusinessUnit::find_by_id(business_unit_id)
                    .one(txn)
                    .await?
                    .is_none()
                {
                    return Err(AppError::NotFound);
                }
                if entity::prelude::User::find_by_id(target_user_id)
                    .one(txn)
                    .await?
                    .is_none()
                {
                    return Err(AppError::BadRequest("user_id not found".into()));
                }

                let am = entity::user_business_unit_role::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    user_id: Set(target_user_id),
                    business_unit_id: Set(business_unit_id),
                    role: Set(role.clone()),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "user_business_unit_role",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({
                        "business_unit_id": business_unit_id,
                        "user_id": target_user_id,
                        "role": role,
                    })),
                )
                .await?;
                Ok(model)
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(model))
}

pub async fn list_roles(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(business_unit_id): Path<Uuid>,
) -> Result<Json<Vec<entity::user_business_unit_role::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::user_business_unit_role::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let items = entity::prelude::UserBusinessUnitRole::find()
                    .filter(entity::user_business_unit_role::Column::BusinessUnitId.eq(business_unit_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}
