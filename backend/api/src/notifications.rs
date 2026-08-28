use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::error::AppError;

/// Creates one in-app notification per recipient: everyone with a role in
/// `project_id`'s business unit, plus every tenant admin. No email/SMS —
/// in-app only, per .ai/decisions/current/2026-08-28-phase-3-audit-and-
/// expansion.md. Not itself audit-logged — it's a derived side effect of
/// the already-audited entity change that triggered it, not one of the
/// core audited entities in its own right.
pub async fn notify_project_team(
    txn: &DatabaseTransaction,
    tenant_id: Uuid,
    project_id: Uuid,
    schedule_task_id: Uuid,
    message: &str,
) -> Result<(), AppError> {
    let project = entity::prelude::Project::find_by_id(project_id)
        .one(txn)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut recipients: std::collections::HashSet<Uuid> =
        entity::prelude::UserBusinessUnitRole::find()
            .filter(entity::user_business_unit_role::Column::BusinessUnitId.eq(project.business_unit_id))
            .all(txn)
            .await?
            .into_iter()
            .map(|r| r.user_id)
            .collect();

    let admins: Vec<Uuid> = entity::prelude::User::find()
        .filter(entity::user::Column::TenantId.eq(tenant_id))
        .filter(entity::user::Column::IsTenantAdmin.eq(true))
        .all(txn)
        .await?
        .into_iter()
        .map(|u| u.id)
        .collect();
    recipients.extend(admins);

    for recipient_user_id in recipients {
        let am = entity::notification::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            recipient_user_id: Set(recipient_user_id),
            project_id: Set(Some(project_id)),
            schedule_task_id: Set(Some(schedule_task_id)),
            message: Set(message.to_string()),
            is_read: Set(false),
            created_at: Set(chrono::Utc::now().into()),
            read_at: Set(None),
        };
        am.insert(txn).await?;
    }
    Ok(())
}
