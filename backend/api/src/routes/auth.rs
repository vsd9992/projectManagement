use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audit,
    auth::{password, session, session::AuthenticatedUser},
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
                    // Only "india" is implemented today (see api::billing);
                    // hardcoded here until a second region profile exists.
                    region_profile: Set("india".to_string()),
                };
                tenant.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "tenant",
                    tenant_id,
                    "create",
                    // No actor yet: the user row that will own this signup
                    // doesn't exist until the insert below, and audit_log's
                    // actor_user_id FK can't point at a not-yet-inserted row.
                    audit::Actor::System,
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
                    audit::Actor::User(user_id),
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

#[derive(Deserialize)]
pub struct CreateTeammateRequest {
    pub email: String,
    pub password: String,
}

/// Creates a second internal user within the caller's own tenant — the
/// missing piece that made "multiple branches, separate teams" impossible
/// to actually exercise: signup always creates a brand-new tenant, so
/// without this a tenant could only ever have one internal user. Any
/// authenticated tenant user can invite a teammate for now (no distinct
/// admin/owner role exists yet — see
/// .ai/decisions/current/2026-08-28-no-rbac-enforcement-yet.md). No initial
/// business-unit role is assigned; that's a separate call to
/// `POST /business-units/:id/roles`.
pub async fn create_teammate(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateTeammateRequest>,
) -> Result<Json<SignupResponse>, AppError> {
    if req.email.trim().is_empty() {
        return Err(AppError::BadRequest("email is required".into()));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }

    let existing = entity::prelude::User::find()
        .filter(entity::user::Column::Email.eq(&req.email))
        .one(&state.admin_db)
        .await?;
    if existing.is_some() {
        return Err(AppError::BadRequest("email is already registered".into()));
    }

    let tenant_id = user.tenant_id;
    let new_user_id = Uuid::new_v4();
    let email = req.email.clone();
    let password_hash = password::hash_password(&req.password)?;

    state
        .app_db
        .transaction::<_, (), AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let am = entity::user::ActiveModel {
                    id: Set(new_user_id),
                    tenant_id: Set(tenant_id),
                    email: Set(email.clone()),
                    password_hash: Set(password_hash),
                    created_at: Set(chrono::Utc::now().into()),
                };
                am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "user",
                    new_user_id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "email": email })),
                )
                .await?;
                Ok(())
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(SignupResponse {
        tenant_id,
        user_id: new_user_id,
    }))
}

pub async fn client_login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<CookieJar, AppError> {
    let client_user = entity::prelude::ClientUser::find()
        .filter(entity::client_user::Column::Email.eq(&req.email))
        .one(&state.admin_db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !password::verify_password(&req.password, &client_user.password_hash) {
        return Err(AppError::Unauthorized);
    }

    let token =
        session::create_client_session(&state.admin_db, client_user.tenant_id, client_user.id)
            .await?;
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

/// Revokes every session belonging to `target_user_id` — the "instantly
/// kill a departing employee's access" capability the session-based auth
/// decision was chosen for, but hadn't actually been built until now (see
/// .ai/decisions/current/2026-08-28-no-rbac-enforcement-yet.md). Any
/// authenticated tenant user can revoke any other tenant user's sessions —
/// there is no distinct admin/owner role yet to restrict this to further;
/// tenant-level RLS is what stops it from reaching another tenant's users.
pub async fn revoke_user_sessions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(target_user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let tenant_id = user.tenant_id;
    state
        .app_db
        .transaction::<_, (), AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                if entity::prelude::User::find_by_id(target_user_id)
                    .one(txn)
                    .await?
                    .is_none()
                {
                    return Err(AppError::NotFound);
                }
                let result = entity::prelude::Session::delete_many()
                    .filter(entity::session::Column::UserId.eq(target_user_id))
                    .exec(txn)
                    .await?;
                audit::record(
                    txn,
                    tenant_id,
                    "session",
                    target_user_id,
                    "delete",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({
                        "revoked_sessions_for_user_id": target_user_id,
                        "count": result.rows_affected,
                    })),
                )
                .await?;
                Ok(())
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(StatusCode::NO_CONTENT)
}
