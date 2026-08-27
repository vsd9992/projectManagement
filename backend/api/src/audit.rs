use sea_orm::{ActiveModelTrait, DatabaseTransaction, DbErr, Set};
use serde_json::Value as Json;
use uuid::Uuid;

/// Records one audit-log entry within the given transaction. Must be called
/// in the same transaction as the entity change it describes, so that either
/// both commit or both roll back together — an entity change without a
/// corresponding audit entry is exactly the gap the traceability priority
/// (.ai/project/risks.md risk #4) exists to prevent.
#[allow(clippy::too_many_arguments)]
pub async fn record(
    txn: &DatabaseTransaction,
    tenant_id: Uuid,
    entity_type: &str,
    entity_id: Uuid,
    action: &str,
    actor_user_id: Option<Uuid>,
    before: Option<Json>,
    after: Option<Json>,
) -> Result<(), DbErr> {
    let am = entity::audit_log::ActiveModel {
        tenant_id: Set(tenant_id),
        entity_type: Set(entity_type.to_string()),
        entity_id: Set(entity_id),
        action: Set(action.to_string()),
        actor_user_id: Set(actor_user_id),
        before_data: Set(before),
        after_data: Set(after),
        created_at: Set(chrono::Utc::now().into()),
        ..Default::default()
    };
    am.insert(txn).await?;
    Ok(())
}
