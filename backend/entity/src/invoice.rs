use sea_orm::entity::prelude::*;
use rust_decimal::Decimal;

/// Milestone-based billing only for now (the generic engine's other
/// methods — progressive RA-style, lump-sum — aren't implemented yet; see
/// the migration that created this table for why).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "invoices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub project_id: Uuid,
    pub milestone_id: Uuid,
    pub base_amount: Decimal,
    pub retention_percent: Decimal,
    pub gst_amount: Decimal,
    pub gst_tds_amount: Decimal,
    pub retention_amount: Decimal,
    pub gross_amount: Decimal,
    pub net_payable: Decimal,
    /// One of: "raised", "paid" (DB CHECK constraint).
    pub status: String,
    pub raised_by: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub paid_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id"
    )]
    Project,
    #[sea_orm(
        belongs_to = "super::milestone::Entity",
        from = "Column::MilestoneId",
        to = "super::milestone::Column::Id"
    )]
    Milestone,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl Related<super::milestone::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Milestone.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
