use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
-- Tenant-level configuration API scope (narrowed, see .ai/decisions/
-- current/2026-08-28-phase-3-audit-and-expansion.md): region_profile
-- already existed as a column with no API to read/write it; this adds
-- workstream_labels, a per-tenant override of the 4 workstream display
-- names (e.g. a furniture tenant might relabel "site_execution" as
-- "Installation"). Configurable approval chains are explicitly NOT part
-- of this — that needs the generic Approval Workflow entity, a separate,
-- comparably-sized undertaking.
ALTER TABLE tenants ADD COLUMN workstream_labels JSONB NOT NULL DEFAULT '{}'::jsonb;
"#;

const DOWN_SQL: &str = r#"
ALTER TABLE tenants DROP COLUMN IF EXISTS workstream_labels;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(DOWN_SQL)
            .await?;
        Ok(())
    }
}
