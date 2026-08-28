use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
-- App-level check in billing::create_invoice already rejects a second
-- invoice against the same milestone; this closes the TOCTOU gap between
-- that check and the insert. Partial (WHERE milestone_id IS NOT NULL) so
-- it stays valid once milestone_id becomes nullable for progressive
-- billing (see the follow-up migration adding that column).
CREATE UNIQUE INDEX idx_invoices_one_per_milestone
    ON invoices(milestone_id)
    WHERE milestone_id IS NOT NULL;
"#;

const DOWN_SQL: &str = r#"
DROP INDEX IF EXISTS idx_invoices_one_per_milestone;
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
