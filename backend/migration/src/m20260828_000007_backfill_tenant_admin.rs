use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Tenants created before m20260828_000006 have no tenant admin — the
// is_tenant_admin column defaulted to false for existing rows, and only
// new signups auto-promote their founder. Without this backfill, any
// pre-existing tenant is permanently locked out of org-management actions
// (create BU, assign roles, add teammates) with no path forward via the
// API. Promotes each existing tenant's earliest-created user, mirroring
// what signup does for new tenants.
const UP_SQL: &str = r#"
UPDATE users SET is_tenant_admin = true
WHERE id IN (
    SELECT DISTINCT ON (tenant_id) id
    FROM users
    ORDER BY tenant_id, created_at ASC
)
AND NOT EXISTS (
    SELECT 1 FROM users u2 WHERE u2.tenant_id = users.tenant_id AND u2.is_tenant_admin
);
"#;

// Not meaningfully reversible (would need to know which rows this specific
// migration touched vs. admins granted afterward through normal use) —
// intentionally a no-op down, consistent with "this backfill is a one-way
// data correction, not a schema change to undo."
const DOWN_SQL: &str = "";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(UP_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !DOWN_SQL.is_empty() {
            manager
                .get_connection()
                .execute_unprepared(DOWN_SQL)
                .await?;
        }
        Ok(())
    }
}
