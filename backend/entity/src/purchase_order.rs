use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "purchase_orders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub project_id: Uuid,
    pub vendor_id: Uuid,
    pub title: String,
    /// One of: "open", "delivered" (DB CHECK constraint).
    pub status: String,
    pub created_by: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub delivered_at: Option<DateTimeWithTimeZone>,
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
        belongs_to = "super::vendor::Entity",
        from = "Column::VendorId",
        to = "super::vendor::Column::Id"
    )]
    Vendor,
    #[sea_orm(has_many = "super::purchase_order_line_item::Entity")]
    PurchaseOrderLineItem,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl Related<super::vendor::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vendor.def()
    }
}

impl Related<super::purchase_order_line_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PurchaseOrderLineItem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
