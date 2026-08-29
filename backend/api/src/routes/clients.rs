use axum::{
    extract::{Path, State},
    Json,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    audit,
    auth::{password, session::AuthenticatedUser},
    authz,
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateClientRequest {
    pub name: String,
}

#[utoipa::path(
    post,
    path = "/api/clients",
    tag = "clients",
    request_body = CreateClientRequest,
    responses(
        (status = 200, description = "Client created", body = entity::client::Model),
        (status = 400, description = "bad request", body = crate::error::ErrorResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
    )
)]
pub async fn create_client(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateClientRequest>,
) -> Result<Json<entity::client::Model>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let name = req.name.clone();

    let model = state
        .app_db
        .transaction::<_, entity::client::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_any_business_unit_role(txn, user, Some("sales_design")).await?;
                let am = entity::client::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    name: Set(name.clone()),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "client",
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateClientUserRequest {
    pub email: String,
    pub password: String,
}

/// Creates a Client Portal login for a contact at `client_id`. Internal-user
/// authenticated: for now an internal Sales/Design user sets the initial
/// password directly rather than an email-invite flow, which is out of
/// scope for this milestone.
#[utoipa::path(
    post,
    path = "/api/clients/{client_id}/users",
    tag = "clients",
    params(("client_id" = Uuid, Path, description = "Client id")),
    request_body = CreateClientUserRequest,
    responses(
        (status = 200, description = "Client Portal login created", body = entity::client_user::Model),
        (status = 400, description = "bad request", body = crate::error::ErrorResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn create_client_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(client_id): Path<Uuid>,
    Json(req): Json<CreateClientUserRequest>,
) -> Result<Json<entity::client_user::Model>, AppError> {
    if req.email.trim().is_empty() {
        return Err(AppError::BadRequest("email is required".into()));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let email = req.email.clone();
    let password_hash = password::hash_password(&req.password)?;

    let model = state
        .app_db
        .transaction::<_, entity::client_user::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_any_business_unit_role(txn, user, Some("sales_design")).await?;

                // Confirms client_id belongs to this tenant (RLS-scoped read).
                if entity::prelude::Client::find_by_id(client_id)
                    .one(txn)
                    .await?
                    .is_none()
                {
                    return Err(AppError::NotFound);
                }

                let am = entity::client_user::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    client_id: Set(client_id),
                    email: Set(email.clone()),
                    password_hash: Set(password_hash),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "client_user",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "email": email, "client_id": client_id })),
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
    path = "/api/clients",
    tag = "clients",
    responses(
        (status = 200, description = "List clients", body = Vec<entity::client::Model>),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
    )
)]
pub async fn list_clients(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<entity::client::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::client::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_any_business_unit_role(txn, user, None).await?;
                let items = entity::prelude::Client::find().all(txn).await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(items))
}
