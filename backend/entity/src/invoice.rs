use sea_orm::entity::prelude::*;
use rust_decimal::Decimal;

/// Two billing methods: "milestone" (milestone_id set, certified_value_to_date
/// NULL) and "progressive"/RA-bill-style (milestone_id NULL,
/// certified_value_to_date set — the running cumulative certified value as
/// of this bill; base_amount is the incremental delta since the prior bill).
/// Enforced by the `invoices_method_shape` DB CHECK constraint.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[sea_orm(table_name = "invoices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub project_id: Uuid,
    pub milestone_id: Option<Uuid>,
    pub billing_method: String,
    pub certified_value_to_date: Option<Decimal>,
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
