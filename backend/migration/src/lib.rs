pub use sea_orm_migration::prelude::*;

mod m20260827_000001_create_core_schema;
mod m20260827_000002_add_sales_design_workstream;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260827_000001_create_core_schema::Migration),
            Box::new(m20260827_000002_add_sales_design_workstream::Migration),
        ]
    }
}
