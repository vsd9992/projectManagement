use axum::{extract::State, http::StatusCode, Json};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audit,
    auth::{password, session},
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

fn session_cookie(token: String) -> Cookie<'static> {
    Cookie::build((session::SESSION_COOKIE_NAME, token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .build()
}

#[derive(Deserialize)]
pub struct SignupRequest {
    pub tenant_name: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct SignupResponse {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}

pub async fn signup(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<SignupRequest>,
) -> Result<(CookieJar, Json<SignupResponse>), AppError> {
    if req.tenant_name.trim().is_empty() {
        return Err(AppError::BadRequest("tenant_name is required".into()));
    }
    if req.email.trim().is_empty() {
        return Err(AppError::BadRequest("email is required".into()));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }

    // Email is globally unique (single global login — the tenant is resolved
    // from the account, not a subdomain). Checked via the admin connection
    // since no tenant context exists yet. This is a best-effort pre-check;
    // the DB's UNIQUE constraint is the actual guarantee under a race.
    let existing = entity::prelude::User::find()
        .filter(entity::user::Column::Email.eq(&req.email))
        .one(&state.admin_db)
        .await?;
    if existing.is_some() {
        return Err(AppError::BadRequest("email is already registered".into()));
    }

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tenant_name = req.tenant_name.clone();
    let email = req.email.clone();
    let password_hash = password::hash_password(&req.password)?;

    state
        .app_db
        .transaction::<_, (), AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let tenant = entity::tenant::ActiveModel {
                    id: Set(tenant_id),
                    name: Set(tenant_name.clone()),
                    created_at: Set(chrono::Utc::now().into()),
                };
                tenant.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "tenant",
                    tenant_id,
                    "create",
                    Some(user_id),
                    None,
                    Some(serde_json::json!({ "name": tenant_name })),
                )
                .await?;

                let user = entity::user::ActiveModel {
                    id: Set(user_id),
                    tenant_id: Set(tenant_id),
                    email: Set(email.clone()),
                    password_hash: Set(password_hash),
                    created_at: Set(chrono::Utc::now().into()),
                };
                user.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "user",
                    user_id,
                    "create",
                    Some(user_id),
                    None,
                    Some(serde_json::json!({ "email": email })),
                )
                .await?;

                Ok(())
            })
        })
        .await
        .map_err(map_txn_err)?;

    let token = session::create_session(&state.admin_db, tenant_id, user_id).await?;
    Ok((
        jar.add(session_cookie(token)),
        Json(SignupResponse { tenant_id, user_id }),
    ))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<CookieJar, AppError> {
    let user = entity::prelude::User::find()
        .filter(entity::user::Column::Email.eq(&req.email))
        .one(&state.admin_db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !password::verify_password(&req.password, &user.password_hash) {
        return Err(AppError::Unauthorized);
    }

    let token = session::create_session(&state.admin_db, user.tenant_id, user.id).await?;
    Ok(jar.add(session_cookie(token)))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), AppError> {
    if let Some(cookie) = jar.get(session::SESSION_COOKIE_NAME) {
        session::delete_session(&state.admin_db, cookie.value()).await?;
    }
    let jar = jar.remove(Cookie::from(session::SESSION_COOKIE_NAME));
    Ok((jar, StatusCode::NO_CONTENT))
}
