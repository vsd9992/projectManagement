use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
-- One billing method per project — "cumulative certified value to date" is
-- only well-defined within one consistent method's invoice history, so a
-- project commits to a method rather than mixing per-invoice.
ALTER TABLE projects ADD COLUMN billing_method TEXT NOT NULL DEFAULT 'milestone'
    CHECK (billing_method IN ('milestone', 'progressive'));

ALTER TABLE invoices ALTER COLUMN milestone_id DROP NOT NULL;
ALTER TABLE invoices ADD COLUMN billing_method TEXT NOT NULL DEFAULT 'milestone'
    CHECK (billing_method IN ('milestone', 'progressive'));
ALTER TABLE invoices ADD COLUMN certified_value_to_date NUMERIC(14, 2);
ALTER TABLE invoices ADD CONSTRAINT invoices_method_shape CHECK (
    (billing_method = 'milestone'   AND milestone_id IS NOT NULL AND certified_value_to_date IS NULL) OR
    (billing_method = 'progressive' AND milestone_id IS NULL     AND certified_value_to_date IS NOT NULL)
);
"#;

const DOWN_SQL: &str = r#"
ALTER TABLE invoices DROP CONSTRAINT IF EXISTS invoices_method_shape;
ALTER TABLE invoices DROP COLUMN IF EXISTS certified_value_to_date;
ALTER TABLE invoices DROP COLUMN IF EXISTS billing_method;
ALTER TABLE invoices ALTER COLUMN milestone_id SET NOT NULL;
ALTER TABLE projects DROP COLUMN IF EXISTS billing_method;
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
