use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
CREATE TABLE vendors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    contact_email TEXT,
    contact_phone TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_vendors_tenant ON vendors(tenant_id);

-- Procurement is internal-facing only in MVP (no vendor portal/login) — see
-- .ai/decisions/current/2026-08-27-mvp-finance-and-vendor-access-scope.md.
CREATE TABLE purchase_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    vendor_id UUID NOT NULL REFERENCES vendors(id) ON DELETE RESTRICT,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'delivered')),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    delivered_at TIMESTAMPTZ
);
CREATE INDEX idx_po_tenant ON purchase_orders(tenant_id);
CREATE INDEX idx_po_project ON purchase_orders(project_id);

CREATE TABLE purchase_order_line_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    purchase_order_id UUID NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    quotation_line_item_id UUID REFERENCES quotation_line_items(id) ON DELETE SET NULL,
    description TEXT NOT NULL,
    quantity NUMERIC(14, 2) NOT NULL,
    unit TEXT NOT NULL,
    unit_rate NUMERIC(14, 2) NOT NULL,
    amount NUMERIC(14, 2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_poli_tenant ON purchase_order_line_items(tenant_id);
CREATE INDEX idx_poli_po ON purchase_order_line_items(purchase_order_id);

-- Simplified Manufacturing depth for MVP (full shop-floor/QC/dispatch is a
-- later standalone-vertical build) — see
-- .ai/decisions/current/2026-08-27-mvp-scope-turnkey-interiors-first.md.
CREATE TABLE production_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'not_started' CHECK (status IN ('not_started', 'in_progress', 'completed')),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_production_tasks_tenant ON production_tasks(tenant_id);
CREATE INDEX idx_production_tasks_project ON production_tasks(project_id);

CREATE TABLE site_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'not_started' CHECK (status IN ('not_started', 'in_progress', 'done')),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_site_tasks_tenant ON site_tasks(tenant_id);
CREATE INDEX idx_site_tasks_project ON site_tasks(project_id);

-- Explicit dependency links between site tasks — the mechanism from
-- .ai/project/architecture.md ("cross-workstream dependencies are explicit
-- links... not an assumed global stage order"). Scoped to site-task-to-
-- site-task for M4; a richer cross-entity graph (e.g. a task depending on a
-- PO delivery) is future work once Schedule entities exist.
CREATE TABLE site_task_dependencies (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    task_id UUID NOT NULL REFERENCES site_tasks(id) ON DELETE CASCADE,
    depends_on_task_id UUID NOT NULL REFERENCES site_tasks(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (task_id, depends_on_task_id),
    CHECK (task_id <> depends_on_task_id)
);
CREATE INDEX idx_std_tenant ON site_task_dependencies(tenant_id);

CREATE TABLE daily_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    log_date DATE NOT NULL,
    notes TEXT NOT NULL,
    logged_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_daily_logs_tenant ON daily_logs(tenant_id);
CREATE INDEX idx_daily_logs_project ON daily_logs(project_id);

CREATE TABLE punch_list_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'closed')),
    raised_by UUID NOT NULL REFERENCES users(id),
    raised_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_by UUID REFERENCES users(id),
    closed_at TIMESTAMPTZ
);
CREATE INDEX idx_punch_list_tenant ON punch_list_items(tenant_id);
CREATE INDEX idx_punch_list_project ON punch_list_items(project_id);

-- "Basic RFI/query log" per requirements.md — full formal RFI/submittal/
-- transmittal routing is deferred to the standalone Civil vertical build.
CREATE TABLE site_queries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    subject TEXT NOT NULL,
    question TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'answered')),
    raised_by UUID NOT NULL REFERENCES users(id),
    raised_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    answer TEXT,
    answered_by UUID REFERENCES users(id),
    answered_at TIMESTAMPTZ
);
CREATE INDEX idx_site_queries_tenant ON site_queries(tenant_id);
CREATE INDEX idx_site_queries_project ON site_queries(project_id);

DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'vendors', 'purchase_orders', 'purchase_order_line_items',
        'production_tasks', 'site_tasks', 'site_task_dependencies',
        'daily_logs', 'punch_list_items', 'site_queries'
    ]
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
DROP TABLE IF EXISTS site_queries;
DROP TABLE IF EXISTS punch_list_items;
DROP TABLE IF EXISTS daily_logs;
DROP TABLE IF EXISTS site_task_dependencies;
DROP TABLE IF EXISTS site_tasks;
DROP TABLE IF EXISTS production_tasks;
DROP TABLE IF EXISTS purchase_order_line_items;
DROP TABLE IF EXISTS purchase_orders;
DROP TABLE IF EXISTS vendors;
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
