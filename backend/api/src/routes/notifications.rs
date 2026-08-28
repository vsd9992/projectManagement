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
    error::{map_txn_err, AppError},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListNotificationsQuery {
    #[serde(default)]
    pub unread_only: bool,
}

pub async fn list_my_notifications(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(q): Query<ListNotificationsQuery>,
) -> Result<Json<Vec<entity::notification::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::notification::Model>, AppError>(|txn| {
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

pub async fn mark_notification_read(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<entity::notification::Model>, AppError> {
    let tenant_id = user.tenant_id;
    let model = state
        .app_db
        .transaction::<_, entity::notification::Model, AppError>(|txn| {
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
