use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
CREATE TABLE leads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    business_unit_id UUID NOT NULL REFERENCES business_units(id) ON DELETE CASCADE,
    client_id UUID NOT NULL REFERENCES clients(id) ON DELETE RESTRICT,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'new' CHECK (status IN ('new', 'qualified', 'converted', 'lost')),
    converted_project_id UUID REFERENCES projects(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_leads_tenant ON leads(tenant_id);

CREATE TABLE client_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    client_id UUID NOT NULL REFERENCES clients(id) ON DELETE CASCADE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_client_users_tenant ON client_users(tenant_id);
CREATE INDEX idx_client_users_client ON client_users(client_id);

-- Extend sessions to carry either principal type, per the "same session
-- mechanism, scoped differently" auth decision
-- (.ai/decisions/current/2026-08-27-auth-session-based-single-login.md).
ALTER TABLE sessions ALTER COLUMN user_id DROP NOT NULL;
ALTER TABLE sessions ADD COLUMN client_user_id UUID REFERENCES client_users(id) ON DELETE CASCADE;
ALTER TABLE sessions ADD CONSTRAINT sessions_one_principal CHECK (
    (user_id IS NOT NULL AND client_user_id IS NULL) OR
    (user_id IS NULL AND client_user_id IS NOT NULL)
);

-- Design-revision approve/reject is a Client Portal action, so audit_log
-- needs to attribute it to a client_user, not just an internal user.
ALTER TABLE audit_log ADD COLUMN actor_client_user_id UUID REFERENCES client_users(id) ON DELETE SET NULL;
ALTER TABLE audit_log ADD CONSTRAINT audit_log_single_actor CHECK (
    NOT (actor_user_id IS NOT NULL AND actor_client_user_id IS NOT NULL)
);

CREATE TABLE quotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version INT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'sent', 'approved', 'rejected', 'superseded')),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (project_id, version)
);
CREATE INDEX idx_quotations_tenant ON quotations(tenant_id);
CREATE INDEX idx_quotations_project ON quotations(project_id);

CREATE TABLE quotation_line_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    quotation_id UUID NOT NULL REFERENCES quotations(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    quantity NUMERIC(14, 2) NOT NULL,
    unit TEXT NOT NULL,
    unit_rate NUMERIC(14, 2) NOT NULL,
    amount NUMERIC(14, 2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_qli_tenant ON quotation_line_items(tenant_id);
CREATE INDEX idx_qli_quotation ON quotation_line_items(quotation_id);

CREATE TABLE design_assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_design_assets_tenant ON design_assets(tenant_id);
CREATE INDEX idx_design_assets_project ON design_assets(project_id);

-- `notes` stands in for real design file content until a document
-- storage/versioning backend is chosen (still open in architecture.md).
CREATE TABLE design_revisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    design_asset_id UUID NOT NULL REFERENCES design_assets(id) ON DELETE CASCADE,
    version INT NOT NULL,
    notes TEXT,
    status TEXT NOT NULL DEFAULT 'submitted' CHECK (status IN ('submitted', 'approved', 'rejected')),
    submitted_by UUID NOT NULL REFERENCES users(id),
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_by UUID REFERENCES client_users(id),
    decided_at TIMESTAMPTZ,
    decision_notes TEXT,
    UNIQUE (design_asset_id, version)
);
CREATE INDEX idx_design_revisions_tenant ON design_revisions(tenant_id);
CREATE INDEX idx_design_revisions_asset ON design_revisions(design_asset_id);

DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['leads', 'client_users', 'quotations', 'quotation_line_items', 'design_assets', 'design_revisions']
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
ALTER TABLE audit_log DROP CONSTRAINT IF EXISTS audit_log_single_actor;
ALTER TABLE audit_log DROP COLUMN IF EXISTS actor_client_user_id;
DROP TABLE IF EXISTS design_revisions;
DROP TABLE IF EXISTS design_assets;
DROP TABLE IF EXISTS quotation_line_items;
DROP TABLE IF EXISTS quotations;
ALTER TABLE sessions DROP CONSTRAINT IF EXISTS sessions_one_principal;
ALTER TABLE sessions DROP COLUMN IF EXISTS client_user_id;
ALTER TABLE sessions ALTER COLUMN user_id SET NOT NULL;
DROP TABLE IF EXISTS client_users;
DROP TABLE IF EXISTS leads;
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
