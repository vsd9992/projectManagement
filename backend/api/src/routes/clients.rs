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
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateClientRequest {
    pub name: String,
}

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

#[derive(Deserialize)]
pub struct CreateClientUserRequest {
    pub email: String,
    pub password: String,
}

/// Creates a Client Portal login for a contact at `client_id`. Internal-user
/// authenticated: for now an internal Sales/Design user sets the initial
/// password directly rather than an email-invite flow, which is out of
/// scope for this milestone.
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
                let items = entity::prelude::Client::find().all(txn).await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(items))
}
