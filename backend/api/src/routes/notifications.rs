use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::session::AuthenticatedUser,
    db::set_tenant,
    error::{map_txn_err, AppError, ErrorResponse},
    state::AppState,
};

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ListNotificationsQuery {
    #[serde(default)]
    pub unread_only: bool,
}

#[utoipa::path(
    get,
    path = "/api/notifications",
    tag = "notifications",
    params(ListNotificationsQuery),
    responses(
        (status = 200, description = "List my notifications, newest first", body = Vec<NotificationModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
    )
)]
pub async fn list_my_notifications(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(q): Query<ListNotificationsQuery>,
) -> Result<Json<Vec<NotificationModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<NotificationModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let mut query = entity::prelude::Notification::find()
                    .filter(entity::notification::Column::RecipientUserId.eq(user.user_id));
                if q.unread_only {
                    query = query.filter(entity::notification::Column::IsRead.eq(false));
                }
                let items = query
                    .order_by_desc(entity::notification::Column::CreatedAt)
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[utoipa::path(
    post,
    path = "/api/notifications/{id}/read",
    tag = "notifications",
    params(("id" = Uuid, Path, description = "Notification id")),
    responses(
        (status = 200, description = "Notification marked read", body = NotificationModel),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn mark_notification_read(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<NotificationModel>, AppError> {
    let tenant_id = user.tenant_id;
    let model = state
        .app_db
        .transaction::<_, NotificationModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let notif = entity::prelude::Notification::find_by_id(id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                // A user may only mark their own notification read — not
                // a business-unit-role check, since notifications are
                // addressed to a specific recipient, not a team.
                if notif.recipient_user_id != user.user_id {
                    return Err(AppError::NotFound);
                }
                let mut am: entity::notification::ActiveModel = notif.into();
                am.is_read = Set(true);
                am.read_at = Set(Some(chrono::Utc::now().into()));
                let updated = am.update(txn).await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

use entity::notification::Model as NotificationModel;
