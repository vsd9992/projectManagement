use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, Utc};
use rand::RngCore;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

pub const SESSION_COOKIE_NAME: &str = "session_token";
const SESSION_TTL_DAYS: i64 = 30;

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

async fn create_session_row(
    admin_db: &sea_orm::DatabaseConnection,
    tenant_id: Uuid,
    user_id: Option<Uuid>,
    client_user_id: Option<Uuid>,
) -> Result<String, AppError> {
    let token = generate_token();
    let token_hash = hash_token(&token);
    let now = Utc::now();

    let am = entity::session::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        user_id: Set(user_id),
        client_user_id: Set(client_user_id),
        token_hash: Set(token_hash),
        created_at: Set(now.into()),
        expires_at: Set((now + Duration::days(SESSION_TTL_DAYS)).into()),
    };
    am.insert(admin_db).await?;
    Ok(token)
}

/// Creates a session for an internal (business-unit-scoped) user. Takes the
/// admin (BYPASSRLS) connection deliberately: session lifecycle is a
/// cross-cutting auth concern, not tenant business data, and at login time no
/// tenant context has been established on the app connection yet.
pub async fn create_session(
    admin_db: &sea_orm::DatabaseConnection,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<String, AppError> {
    create_session_row(admin_db, tenant_id, Some(user_id), None).await
}

/// Creates a session for an external Client Portal user.
pub async fn create_client_session(
    admin_db: &sea_orm::DatabaseConnection,
    tenant_id: Uuid,
    client_user_id: Uuid,
) -> Result<String, AppError> {
    create_session_row(admin_db, tenant_id, None, Some(client_user_id)).await
}

/// Deletes the session matching `token` (logout). No-op if it doesn't exist.
pub async fn delete_session(
    admin_db: &sea_orm::DatabaseConnection,
    token: &str,
) -> Result<(), AppError> {
    let token_hash = hash_token(token);
    entity::prelude::Session::delete_many()
        .filter(entity::session::Column::TokenHash.eq(token_hash))
        .exec(admin_db)
        .await?;
    Ok(())
}

async fn lookup_session(
    app_state: &AppState,
    parts: &Parts,
) -> Result<entity::session::Model, AppError> {
    let jar = CookieJar::from_headers(&parts.headers);
    let token = jar
        .get(SESSION_COOKIE_NAME)
        .map(|c: &Cookie| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;
    let token_hash = hash_token(&token);

    let session = entity::prelude::Session::find()
        .filter(entity::session::Column::TokenHash.eq(token_hash))
        .one(&app_state.admin_db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if session.expires_at < Utc::now() {
        return Err(AppError::Unauthorized);
    }
    Ok(session)
}

/// The authenticated internal-user principal for a request, resolved from
/// the session cookie. Rejects a session that belongs to a Client Portal
/// user instead.
#[derive(Clone, Copy, Debug)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let session = lookup_session(&app_state, parts).await?;
        let user_id = session.user_id.ok_or(AppError::Unauthorized)?;
        Ok(AuthenticatedUser {
            user_id,
            tenant_id: session.tenant_id,
        })
    }
}

/// The authenticated Client Portal principal for a request. Rejects a
/// session that belongs to an internal user instead. `client_id` is the
/// client this person represents — every client-facing handler must filter
/// by it in addition to the tenant scoping RLS already provides, since RLS
/// alone would let a client see every project in the tenant, not just their
/// own (see .ai/decisions/current/2026-08-27-auth-session-based-single-login.md).
#[derive(Clone, Copy, Debug)]
pub struct AuthenticatedClientUser {
    pub client_user_id: Uuid,
    pub client_id: Uuid,
    pub tenant_id: Uuid,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthenticatedClientUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let session = lookup_session(&app_state, parts).await?;
        let client_user_id = session.client_user_id.ok_or(AppError::Unauthorized)?;

        let client_user = entity::prelude::ClientUser::find_by_id(client_user_id)
            .one(&app_state.admin_db)
            .await?
            .ok_or(AppError::Unauthorized)?;

        Ok(AuthenticatedClientUser {
            client_user_id,
            client_id: client_user.client_id,
            tenant_id: session.tenant_id,
        })
    }
}
