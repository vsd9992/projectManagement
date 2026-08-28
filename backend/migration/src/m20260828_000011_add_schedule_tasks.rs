use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
CREATE TABLE schedule_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    workstream_type workstream_type NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'not_started' CHECK (status IN ('not_started', 'in_progress', 'done')),
    planned_start_date DATE,
    planned_end_date DATE,
    actual_start_date DATE,
    actual_end_date DATE,
    -- At most one of these may be set: an optional link back to the
    -- workstream-specific leaf record this schedule task represents timing
    -- for. site_task_id is always set for a site task (see backfill below —
    -- schedule_tasks is now the sole source of dependency data, replacing
    -- site_task_dependencies); the other three are populated only when a PM
    -- explicitly opts a production task/design revision/PO into the graph.
    site_task_id UUID REFERENCES site_tasks(id) ON DELETE SET NULL,
    production_task_id UUID REFERENCES production_tasks(id) ON DELETE SET NULL,
    design_revision_id UUID REFERENCES design_revisions(id) ON DELETE SET NULL,
    purchase_order_id UUID REFERENCES purchase_orders(id) ON DELETE SET NULL,
    spawned_by_change_order_id UUID REFERENCES change_orders(id) ON DELETE SET NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (num_nonnulls(site_task_id, production_task_id, design_revision_id, purchase_order_id) <= 1),
    CHECK (planned_start_date IS NULL OR planned_end_date IS NULL OR planned_start_date <= planned_end_date),
    CHECK (actual_start_date IS NULL OR actual_end_date IS NULL OR actual_start_date <= actual_end_date)
);
CREATE INDEX idx_schedule_tasks_tenant ON schedule_tasks(tenant_id);
CREATE INDEX idx_schedule_tasks_project ON schedule_tasks(project_id);
CREATE UNIQUE INDEX idx_schedule_tasks_site_task ON schedule_tasks(site_task_id) WHERE site_task_id IS NOT NULL;

-- Generalizes site_task_dependencies (site-task-to-site-task only) into a
-- graph spanning any schedule_tasks row, across all four workstreams.
CREATE TABLE schedule_task_dependencies (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    task_id UUID NOT NULL REFERENCES schedule_tasks(id) ON DELETE CASCADE,
    depends_on_task_id UUID NOT NULL REFERENCES schedule_tasks(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (task_id, depends_on_task_id),
    CHECK (task_id <> depends_on_task_id)
);
CREATE INDEX idx_sched_std_tenant ON schedule_task_dependencies(tenant_id);
CREATE INDEX idx_sched_std_depends_on ON schedule_task_dependencies(depends_on_task_id);

-- Staging table for Change Order approval spawning new schedule tasks for
-- added scope (wired up in a later Phase 3 stage) — created now since it
-- references schedule_tasks.
CREATE TABLE change_order_schedule_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    change_order_id UUID NOT NULL REFERENCES change_orders(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    workstream_type workstream_type NOT NULL,
    planned_start_date DATE,
    planned_end_date DATE,
    depends_on_existing_task_id UUID REFERENCES schedule_tasks(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_cost_tenant ON change_order_schedule_tasks(tenant_id);
CREATE INDEX idx_cost_change_order ON change_order_schedule_tasks(change_order_id);

-- Backfill: one schedule_task per existing site_task, then remap existing
-- site_task_dependencies rows via that site_task_id -> schedule_task.id
-- mapping, so no dependency data is lost by the replacement below.
INSERT INTO schedule_tasks (id, tenant_id, project_id, workstream_type, title, status, site_task_id, created_by, created_at)
SELECT gen_random_uuid(), tenant_id, project_id, 'site_execution', title, status, id, created_by, created_at
FROM site_tasks;

INSERT INTO schedule_task_dependencies (tenant_id, task_id, depends_on_task_id, created_at)
SELECT st.tenant_id, t1.id, t2.id, std.created_at
FROM site_task_dependencies std
JOIN schedule_tasks t1 ON t1.site_task_id = std.task_id
JOIN schedule_tasks t2 ON t2.site_task_id = std.depends_on_task_id
JOIN site_tasks st ON st.id = std.task_id;

DROP TABLE site_task_dependencies;

DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['schedule_tasks', 'schedule_task_dependencies', 'change_order_schedule_tasks']
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
CREATE TABLE site_task_dependencies (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    task_id UUID NOT NULL REFERENCES site_tasks(id) ON DELETE CASCADE,
    depends_on_task_id UUID NOT NULL REFERENCES site_tasks(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (task_id, depends_on_task_id),
    CHECK (task_id <> depends_on_task_id)
);
CREATE INDEX idx_std_tenant ON site_task_dependencies(tenant_id);
ALTER TABLE site_task_dependencies ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON site_task_dependencies USING (tenant_id = current_setting('app.tenant_id', true)::uuid);

DROP TABLE IF EXISTS change_order_schedule_tasks;
DROP TABLE IF EXISTS schedule_task_dependencies;
DROP TABLE IF EXISTS schedule_tasks;
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
