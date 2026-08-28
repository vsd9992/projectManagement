use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
-- Workstream(s) a Change Order requests enabling on its project, alongside
-- (or instead of) BOQ line changes. Approval enables them on the project
-- atomically with the re-baseline (see .ai/decisions/current/
-- 2026-08-28-workstream-enforcement-and-expansion.md) — this is the only
-- way to add a workstream to an already-existing project; enforcement of
-- project_workstreams membership at the API layer would otherwise be a
-- dead end for legitimate mid-project scope growth.
CREATE TABLE change_order_workstreams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    change_order_id UUID NOT NULL REFERENCES change_orders(id) ON DELETE CASCADE,
    workstream_type workstream_type NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (change_order_id, workstream_type)
);
CREATE INDEX idx_cow_tenant ON change_order_workstreams(tenant_id);
CREATE INDEX idx_cow_change_order ON change_order_workstreams(change_order_id);

ALTER TABLE change_order_workstreams ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON change_order_workstreams USING (tenant_id = current_setting('app.tenant_id', true)::uuid);
"#;

const DOWN_SQL: &str = r#"
DROP TABLE IF EXISTS change_order_workstreams;
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
