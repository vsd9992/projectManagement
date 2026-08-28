use sea_orm::{ActiveModelTrait, DatabaseTransaction, DbErr, Set};
use serde_json::Value as Json;
use uuid::Uuid;

/// Who performed an audited action. Kept explicit (rather than a bare
/// `Option<Uuid>`) since audit_log can attribute an entry to either an
/// internal user or a Client Portal user, but never both — see the
/// `audit_log_single_actor` CHECK constraint added in
/// m20260827_000002_add_sales_design_workstream.
#[derive(Clone, Copy)]
pub enum Actor {
    User(Uuid),
    ClientUser(Uuid),
    /// No actor exists yet — e.g. the tenant-creation entry during signup,
    /// where the user row that will own the signup doesn't exist until the
    /// insert immediately after it.
    System,
}

/// Records one audit-log entry within the given transaction. Must be called
/// in the same transaction as the entity change it describes, so that either
/// both commit or both roll back together — an entity change without a
/// corresponding audit entry is exactly the gap the traceability priority
/// (.ai/project/risks.md risk #4) exists to prevent.
pub async fn record(
    txn: &DatabaseTransaction,
    tenant_id: Uuid,
    entity_type: &str,
    entity_id: Uuid,
    action: &str,
    actor: Actor,
    before: Option<Json>,
    after: Option<Json>,
) -> Result<(), DbErr> {
    let (actor_user_id, actor_client_user_id) = match actor {
        Actor::User(id) => (Some(id), None),
        Actor::ClientUser(id) => (None, Some(id)),
        Actor::System => (None, None),
    };
    let am = entity::audit_log::ActiveModel {
        tenant_id: Set(tenant_id),
        entity_type: Set(entity_type.to_string()),
        entity_id: Set(entity_id),
        action: Set(action.to_string()),
        actor_user_id: Set(actor_user_id),
        actor_client_user_id: Set(actor_client_user_id),
        before_data: Set(before),
        after_data: Set(after),
        created_at: Set(chrono::Utc::now().into()),
        ..Default::default()
    };
    am.insert(txn).await?;
    Ok(())
}
