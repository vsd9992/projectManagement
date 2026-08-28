use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
CREATE TABLE change_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    base_quotation_id UUID NOT NULL REFERENCES quotations(id) ON DELETE RESTRICT,
    new_quotation_id UUID REFERENCES quotations(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending_client_approval' CHECK (status IN ('pending_client_approval', 'approved', 'rejected')),
    cost_impact NUMERIC(14, 2) NOT NULL,
    requested_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_by UUID REFERENCES client_users(id),
    decided_at TIMESTAMPTZ,
    decision_notes TEXT
);
CREATE INDEX idx_change_orders_tenant ON change_orders(tenant_id);
CREATE INDEX idx_change_orders_project ON change_orders(project_id);

-- Each row is either a new line (original_line_item_id NULL), a modification
-- (original_line_item_id set, removed = false — replaces the original in the
-- re-baselined quotation), or a removal (original_line_item_id set, removed
-- = true — dropped from the re-baselined quotation). This is the mechanism
-- that lets one Change Order represent scope extension, reduction, or both
-- at once (see .ai/project/workflows.md § Change Order / Scope Change Flow).
CREATE TABLE change_order_line_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    change_order_id UUID NOT NULL REFERENCES change_orders(id) ON DELETE CASCADE,
    original_line_item_id UUID REFERENCES quotation_line_items(id) ON DELETE SET NULL,
    removed BOOLEAN NOT NULL DEFAULT false,
    description TEXT NOT NULL,
    quantity NUMERIC(14, 2) NOT NULL,
    unit TEXT NOT NULL,
    unit_rate NUMERIC(14, 2) NOT NULL,
    amount NUMERIC(14, 2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_coli_tenant ON change_order_line_items(tenant_id);
CREATE INDEX idx_coli_change_order ON change_order_line_items(change_order_id);

DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['change_orders', 'change_order_line_items']
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
DROP TABLE IF EXISTS change_order_line_items;
DROP TABLE IF EXISTS change_orders;
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
