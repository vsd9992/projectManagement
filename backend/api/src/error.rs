use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("tenant is paused or has been deleted")]
    TenantSuspended,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Unwraps a SeaORM transaction error into the inner `AppError`, so handlers
/// can `.map_err(map_txn_err)?` after `db.transaction(...)` instead of
/// matching the two-variant `TransactionError` at every call site.
pub fn map_txn_err(e: sea_orm::TransactionError<AppError>) -> AppError {
    match e {
        sea_orm::TransactionError::Connection(e) => AppError::Db(e),
        sea_orm::TransactionError::Transaction(e) => e,
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::TenantSuspended => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Db(e) => {
                tracing::error!(error = %e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
            AppError::Internal(e) => {
                tracing::error!(error = %e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
