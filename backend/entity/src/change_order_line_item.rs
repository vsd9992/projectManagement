use sea_orm::entity::prelude::*;
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[sea_orm(table_name = "change_order_line_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub change_order_id: Uuid,
    /// NULL means this is a newly added line (scope extension). Set means
    /// this modifies (removed = false) or removes (removed = true) an
    /// existing line from the base quotation (scope change/reduction).
    pub original_line_item_id: Option<Uuid>,
    pub removed: bool,
    pub description: String,
    pub quantity: Decimal,
    pub unit: String,
    pub unit_rate: Decimal,
    pub amount: Decimal,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::change_order::Entity",
        from = "Column::ChangeOrderId",
        to = "super::change_order::Column::Id"
    )]
    ChangeOrder,
}

impl Related<super::change_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChangeOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
