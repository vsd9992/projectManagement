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
    error::{map_txn_err, AppError, ErrorResponse},
    state::AppState,
};

fn session_cookie(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((session::SESSION_COOKIE_NAME, token))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .build()
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SignupRequest {
    pub tenant_name: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SignupResponse {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}

/// Creates a brand-new tenant and its founding admin user, and logs them in.
#[utoipa::path(
    post,
    path = "/api/auth/signup",
    tag = "auth",
    request_body = SignupRequest,
    responses(
        (status = 200, description = "Tenant and founding admin created", body = SignupResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
    )
)]
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
                    status: Set("active".to_string()),
                    paused_at: Set(None),
                    deleted_at: Set(None),
                    workstream_labels: Set(serde_json::json!({})),
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
                    // The signing-up user is the tenant's founding admin.
                    is_tenant_admin: Set(true),
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
        jar.add(session_cookie(token, state.cookie_secure)),
        Json(SignupResponse { tenant_id, user_id }),
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Logs an internal user in and sets the session cookie. Empty body on
/// success — call `GET /api/auth/me` to fetch the resulting identity.
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Logged in, session cookie set"),
        (status = 401, description = "unauthorized", body = ErrorResponse),
    )
)]
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
    Ok(jar.add(session_cookie(token, state.cookie_secure)))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateTeammateRequest {
    pub email: String,
    pub password: String,
}

/// Creates a second internal user within the caller's own tenant — the
/// missing piece that made "multiple branches, separate teams" impossible
/// to actually exercise: signup always creates a brand-new tenant, so
/// without this a tenant could only ever have one internal user.
/// Tenant-admin only. No initial business-unit role is assigned; that's a
/// separate call to `POST /business-units/:id/roles`.
#[utoipa::path(
    post,
    path = "/api/users",
    tag = "auth",
    request_body = CreateTeammateRequest,
    responses(
        (status = 200, description = "Teammate created", body = SignupResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_teammate(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateTeammateRequest>,
) -> Result<Json<SignupResponse>, AppError> {
    crate::authz::require_tenant_admin(user)?;
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
                    is_tenant_admin: Set(false),
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

/// Logs a Client Portal user in and sets the (shared-name) session cookie.
#[utoipa::path(
    post,
    path = "/api/auth/client-login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Logged in, session cookie set"),
        (status = 401, description = "unauthorized", body = ErrorResponse),
    )
)]
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
    Ok(jar.add(session_cookie(token, state.cookie_secure)))
}

/// Logs the current session out (internal or Client Portal) and clears the
/// cookie.
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    responses((status = 204, description = "Logged out"))
)]
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
/// decision was chosen for, but hadn't actually been built until now.
/// Tenant-admin only; tenant-level RLS is what stops it from reaching
/// another tenant's users.
#[utoipa::path(
    post,
    path = "/api/users/{id}/revoke-sessions",
    tag = "auth",
    params(("id" = Uuid, Path, description = "Target user id")),
    responses(
        (status = 204, description = "Sessions revoked"),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn revoke_user_sessions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(target_user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    crate::authz::require_tenant_admin(user)?;
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetTenantAdminRequest {
    pub is_tenant_admin: bool,
}

/// Promotes or demotes `target_user_id` to/from tenant admin. Tenant-admin
/// only — an existing admin must grant the status; there is no other path
/// to becoming one besides being the user who originally signed up. A user
/// cannot demote themselves if they're the tenant's only remaining admin,
/// to avoid a tenant permanently locking itself out of its own admin tier.
#[utoipa::path(
    post,
    path = "/api/users/{id}/admin",
    tag = "auth",
    params(("id" = Uuid, Path, description = "Target user id")),
    request_body = SetTenantAdminRequest,
    responses(
        (status = 200, description = "Admin status updated", body = UserModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn set_tenant_admin(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(target_user_id): Path<Uuid>,
    Json(req): Json<SetTenantAdminRequest>,
) -> Result<Json<UserModel>, AppError> {
    crate::authz::require_tenant_admin(user)?;
    let tenant_id = user.tenant_id;
    let new_value = req.is_tenant_admin;

    let model = state
        .app_db
        .transaction::<_, UserModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let target = entity::prelude::User::find_by_id(target_user_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;

                if !new_value && target.is_tenant_admin {
                    let other_admins = entity::prelude::User::find()
                        .filter(entity::user::Column::IsTenantAdmin.eq(true))
                        .filter(entity::user::Column::Id.ne(target_user_id))
                        .one(txn)
                        .await?;
                    if other_admins.is_none() {
                        return Err(AppError::BadRequest(
                            "cannot demote the tenant's only remaining admin".into(),
                        ));
                    }
                }

                let before = serde_json::json!({ "is_tenant_admin": target.is_tenant_admin });
                let mut am: entity::user::ActiveModel = target.into();
                am.is_tenant_admin = Set(new_value);
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "user",
                    target_user_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({ "is_tenant_admin": new_value })),
                )
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

/// Current internal user's identity — the only way a frontend can know
/// who's logged in (and their tenant/admin status) after a page refresh,
/// since the session cookie is httpOnly and login/signup return no
/// persisted identity. Deliberately a narrow DTO, never the raw
/// `user::Model`, so `password_hash` can never leak even by accident.
#[derive(Serialize, utoipa::ToSchema)]
pub struct MeResponse {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub is_tenant_admin: bool,
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "Current user identity", body = MeResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
    )
)]
pub async fn me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<MeResponse>, AppError> {
    let tenant_id = user.tenant_id;
    let model = state
        .app_db
        .transaction::<_, UserModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                entity::prelude::User::find_by_id(user.user_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::Unauthorized)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(MeResponse {
        user_id: model.id,
        tenant_id: model.tenant_id,
        email: model.email,
        is_tenant_admin: model.is_tenant_admin,
    }))
}

use entity::user::Model as UserModel;
