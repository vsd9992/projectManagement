use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "change_order_workstreams")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub change_order_id: Uuid,
    pub workstream_type: super::workstream_type::WorkstreamType,
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
