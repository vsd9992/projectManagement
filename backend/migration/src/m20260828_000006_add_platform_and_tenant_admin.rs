use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
-- Tenant lifecycle: a platform manager can pause (reversible — e.g. a lapsed
-- subscription) or delete (soft, terminal) a tenant. Every session lookup
-- (internal and Client Portal) checks this and rejects non-active tenants.
ALTER TABLE tenants ADD COLUMN status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'deleted'));
ALTER TABLE tenants ADD COLUMN paused_at TIMESTAMPTZ;
ALTER TABLE tenants ADD COLUMN deleted_at TIMESTAMPTZ;

-- Tenant admin: authority across the WHOLE tenant (every business unit),
-- unlike user_business_unit_role which is per-BU. The signing-up user
-- becomes their tenant's founding admin automatically.
ALTER TABLE users ADD COLUMN is_tenant_admin BOOLEAN NOT NULL DEFAULT false;

-- Platform managers are intentionally NOT tenant-scoped data — no tenant_id,
-- no RLS (there is nothing to scope by; enforcement is that only
-- routes/platform.rs ever queries these tables). Kept fully separate from
-- the tenant `sessions` table rather than extending it with a third
-- principal type, since a platform manager session has no natural tenant_id
-- and mixing "who can nuke a customer's data" auth with tenant auth
-- infrastructure is exactly the kind of shared-blast-radius mistake worth
-- avoiding deliberately.
CREATE TABLE platform_admins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE platform_admin_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    platform_admin_id UUID NOT NULL REFERENCES platform_admins(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);
"#;

const DOWN_SQL: &str = r#"
DROP TABLE IF EXISTS platform_admin_sessions;
DROP TABLE IF EXISTS platform_admins;
ALTER TABLE users DROP COLUMN IF EXISTS is_tenant_admin;
ALTER TABLE tenants DROP COLUMN IF EXISTS deleted_at;
ALTER TABLE tenants DROP COLUMN IF EXISTS paused_at;
ALTER TABLE tenants DROP COLUMN IF EXISTS status;
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
