use sea_orm::{ConnectionTrait, DatabaseTransaction, DbErr};
use uuid::Uuid;

/// Sets the tenant context for the current transaction. Must be the first
/// statement run inside any transaction that touches a tenant-scoped table,
/// since RLS policies key off `current_setting('app.tenant_id')`.
///
/// Interpolating `tenant_id` directly into the SQL string is safe here
/// specifically because it's a strongly-typed `Uuid` (its `Display` impl can
/// only ever produce hex digits and hyphens) — `SET LOCAL` does not support
/// bind parameters in PostgreSQL, so this is the standard way to set a GUC
/// from a known-safe value.
pub async fn set_tenant(txn: &DatabaseTransaction, tenant_id: Uuid) -> Result<(), DbErr> {
    txn.execute_unprepared(&format!("SET LOCAL app.tenant_id = '{tenant_id}'"))
        .await?;
    Ok(())
}
