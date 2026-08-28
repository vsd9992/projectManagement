use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
ALTER TABLE purchase_orders DROP CONSTRAINT purchase_orders_status_check;
ALTER TABLE purchase_orders ALTER COLUMN status SET DEFAULT 'pending_approval';
ALTER TABLE purchase_orders ADD CONSTRAINT purchase_orders_status_check
    CHECK (status IN ('pending_approval', 'open', 'delivered', 'rejected'));

ALTER TABLE purchase_orders ADD COLUMN decided_by UUID REFERENCES users(id);
ALTER TABLE purchase_orders ADD COLUMN decided_at TIMESTAMPTZ;
ALTER TABLE purchase_orders ADD COLUMN decision_notes TEXT;

-- Existing 'open'/'delivered' rows (created before this migration, under
-- the old open-by-default behavior) remain valid under the new CHECK as-is
-- — no data migration needed. They're implicitly "already approved" (no
-- decided_by/decided_at set), which correctly reflects that the approval
-- step didn't exist when they were created.
"#;

const DOWN_SQL: &str = r#"
ALTER TABLE purchase_orders DROP COLUMN IF EXISTS decision_notes;
ALTER TABLE purchase_orders DROP COLUMN IF EXISTS decided_at;
ALTER TABLE purchase_orders DROP COLUMN IF EXISTS decided_by;
ALTER TABLE purchase_orders DROP CONSTRAINT IF EXISTS purchase_orders_status_check;
ALTER TABLE purchase_orders ALTER COLUMN status SET DEFAULT 'open';
ALTER TABLE purchase_orders ADD CONSTRAINT purchase_orders_status_check
    CHECK (status IN ('open', 'delivered'));
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
