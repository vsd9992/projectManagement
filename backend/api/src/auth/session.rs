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

/// Creates a session row for `user_id`/`tenant_id` and returns the plaintext
/// token to set as the cookie value. Only the SHA-256 hash of the token is
/// stored, so a database leak alone does not yield usable session tokens.
///
/// Takes the admin (BYPASSRLS) connection deliberately: session lifecycle is
/// a cross-cutting auth concern, not tenant business data, and at login time
/// no tenant context has been established on the app connection yet.
pub async fn create_session(
    admin_db: &sea_orm::DatabaseConnection,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<String, AppError> {
    let token = generate_token();
    let token_hash = hash_token(&token);
    let now = Utc::now();

    let am = entity::session::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        user_id: Set(user_id),
        token_hash: Set(token_hash),
        created_at: Set(now.into()),
        expires_at: Set((now + Duration::days(SESSION_TTL_DAYS)).into()),
    };
    am.insert(admin_db).await?;
    Ok(token)
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

/// The authenticated principal for a request, resolved from the session
/// cookie. Looked up via the BYPASSRLS admin connection, since the tenant
/// context that RLS would need isn't known until this lookup completes.
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

        Ok(AuthenticatedUser {
            user_id: session.user_id,
            tenant_id: session.tenant_id,
        })
    }
}
