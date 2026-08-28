use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
-- Which regional tax/billing rule profile a tenant uses. Only "india" is
-- implemented (see api::billing); the column exists so a second profile can
-- be added later without a schema change — see
-- .ai/decisions/current/2026-08-27-generic-billing-engine-india-first-profile.md.
ALTER TABLE tenants ADD COLUMN region_profile TEXT NOT NULL DEFAULT 'india';

-- Minimal milestone entity: just enough for milestone-based billing to hook
-- into. The full Schedule Task/Dependency/Milestone graph from
-- architecture.md is later work; this is not that.
CREATE TABLE milestones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'completed')),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ
);
CREATE INDEX idx_milestones_tenant ON milestones(tenant_id);
CREATE INDEX idx_milestones_project ON milestones(project_id);

-- Milestone-based billing method only (the generic engine's other methods —
-- progressive RA-style, lump-sum — are not implemented yet; adding a
-- `method` column is deferred until a second method actually exists, per
-- the project's own anti-over-engineering stance in risks.md risk #2).
-- retention_percent is set per-invoice rather than as a project-level
-- setting, since MVP has no dedicated billing-settings entity yet.
CREATE TABLE invoices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    milestone_id UUID NOT NULL REFERENCES milestones(id) ON DELETE RESTRICT,
    base_amount NUMERIC(14, 2) NOT NULL,
    retention_percent NUMERIC(5, 2) NOT NULL,
    gst_amount NUMERIC(14, 2) NOT NULL,
    gst_tds_amount NUMERIC(14, 2) NOT NULL,
    retention_amount NUMERIC(14, 2) NOT NULL,
    gross_amount NUMERIC(14, 2) NOT NULL,
    net_payable NUMERIC(14, 2) NOT NULL,
    status TEXT NOT NULL DEFAULT 'raised' CHECK (status IN ('raised', 'paid')),
    raised_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    paid_at TIMESTAMPTZ
);
CREATE INDEX idx_invoices_tenant ON invoices(tenant_id);
CREATE INDEX idx_invoices_project ON invoices(project_id);

DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['milestones', 'invoices']
    LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON %I USING (tenant_id = current_setting(''app.tenant_id'', true)::uuid)',
            t
        );
    END LOOP;
END $$;
"#;

const DOWN_SQL: &str = r#"
DROP TABLE IF EXISTS invoices;
DROP TABLE IF EXISTS milestones;
ALTER TABLE tenants DROP COLUMN IF EXISTS region_profile;
"#;

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
        manager
            .get_connection()
            .execute_unprepared(DOWN_SQL)
            .await?;
        Ok(())
    }
}
