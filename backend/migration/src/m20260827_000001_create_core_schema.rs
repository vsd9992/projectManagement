use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TYPE workstream_type AS ENUM ('design', 'manufacturing', 'procurement', 'site_execution');

CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE business_units (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_business_units_tenant ON business_units(tenant_id);

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_users_tenant ON users(tenant_id);

CREATE TABLE user_business_unit_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    business_unit_id UUID NOT NULL REFERENCES business_units(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('sales_design', 'delivery', 'finance')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, business_unit_id, role)
);
CREATE INDEX idx_ubur_tenant ON user_business_unit_roles(tenant_id);
CREATE INDEX idx_ubur_user ON user_business_unit_roles(user_id);

CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_sessions_tenant ON sessions(tenant_id);

CREATE TABLE clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_clients_tenant ON clients(tenant_id);

CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    business_unit_id UUID NOT NULL REFERENCES business_units(id) ON DELETE CASCADE,
    client_id UUID NOT NULL REFERENCES clients(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_projects_tenant ON projects(tenant_id);
CREATE INDEX idx_projects_business_unit ON projects(business_unit_id);

CREATE TABLE project_workstreams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    workstream_type workstream_type NOT NULL,
    status TEXT NOT NULL DEFAULT 'not_started',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (project_id, workstream_type)
);
CREATE INDEX idx_project_workstreams_tenant ON project_workstreams(tenant_id);
CREATE INDEX idx_project_workstreams_project ON project_workstreams(project_id);

CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('create', 'update', 'delete')),
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    before_data JSONB,
    after_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_log_tenant ON audit_log(tenant_id);
CREATE INDEX idx_audit_log_entity ON audit_log(entity_type, entity_id);

-- Row-level security: every tenant-scoped table is isolated by tenant_id.
-- The application connects as a role WITHOUT BYPASSRLS and sets the tenant
-- context per request via `SET LOCAL app.tenant_id = '<uuid>'` inside the
-- request's transaction (see .ai/decisions/current/2026-08-27-tenant-isolation-shared-schema-rls.md).
-- A separate BYPASSRLS role is used only for platform-admin/cross-tenant
-- tooling and the login-time lookup of a user by email (before a tenant
-- context can be established).
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['business_units', 'users', 'user_business_unit_roles', 'sessions', 'clients', 'projects', 'project_workstreams', 'audit_log']
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
DROP TABLE IF EXISTS audit_log;
DROP TABLE IF EXISTS project_workstreams;
DROP TABLE IF EXISTS projects;
DROP TABLE IF EXISTS clients;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS user_business_unit_roles;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS business_units;
DROP TABLE IF EXISTS tenants;
DROP TYPE IF EXISTS workstream_type;
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
