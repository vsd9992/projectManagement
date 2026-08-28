pub use sea_orm_migration::prelude::*;

mod m20260827_000001_create_core_schema;
mod m20260827_000002_add_sales_design_workstream;
mod m20260827_000003_add_change_orders;
mod m20260828_000004_add_delivery_workstreams;
mod m20260828_000005_add_billing;
mod m20260828_000006_add_platform_and_tenant_admin;
mod m20260828_000007_backfill_tenant_admin;
mod m20260828_000008_add_change_order_workstreams;
mod m20260828_000009_invoice_milestone_uniqueness;
mod m20260828_000010_add_progressive_billing;
mod m20260828_000011_add_schedule_tasks;
mod m20260828_000012_add_notifications;
mod m20260828_000013_add_po_approval;
mod m20260828_000014_add_tenant_settings;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260827_000001_create_core_schema::Migration),
            Box::new(m20260827_000002_add_sales_design_workstream::Migration),
            Box::new(m20260827_000003_add_change_orders::Migration),
            Box::new(m20260828_000004_add_delivery_workstreams::Migration),
            Box::new(m20260828_000005_add_billing::Migration),
            Box::new(m20260828_000006_add_platform_and_tenant_admin::Migration),
            Box::new(m20260828_000007_backfill_tenant_admin::Migration),
            Box::new(m20260828_000008_add_change_order_workstreams::Migration),
            Box::new(m20260828_000009_invoice_milestone_uniqueness::Migration),
            Box::new(m20260828_000010_add_progressive_billing::Migration),
            Box::new(m20260828_000011_add_schedule_tasks::Migration),
            Box::new(m20260828_000012_add_notifications::Migration),
            Box::new(m20260828_000013_add_po_approval::Migration),
            Box::new(m20260828_000014_add_tenant_settings::Migration),
        ]
    }
}
