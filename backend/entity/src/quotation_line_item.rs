use sea_orm::entity::prelude::*;
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "quotation_line_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub quotation_id: Uuid,
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
        belongs_to = "super::quotation::Entity",
        from = "Column::QuotationId",
        to = "super::quotation::Column::Id"
    )]
    Quotation,
}

impl Related<super::quotation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Quotation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
