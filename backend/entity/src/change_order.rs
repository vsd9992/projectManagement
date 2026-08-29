use sea_orm::entity::prelude::*;
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[sea_orm(table_name = "change_orders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub project_id: Uuid,
    pub base_quotation_id: Uuid,
    pub new_quotation_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    /// One of: "pending_client_approval", "approved", "rejected" (DB CHECK constraint).
    pub status: String,
    pub cost_impact: Decimal,
    pub requested_by: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub decided_by: Option<Uuid>,
    pub decided_at: Option<DateTimeWithTimeZone>,
    pub decision_notes: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id"
    )]
    Project,
    #[sea_orm(has_many = "super::change_order_line_item::Entity")]
    ChangeOrderLineItem,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl Related<super::change_order_line_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChangeOrderLineItem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
