use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[schema(as = QuotationModel)]
#[sea_orm(table_name = "quotations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub project_id: Uuid,
    pub version: i32,
    /// One of: "draft", "sent", "approved", "rejected", "superseded" (DB CHECK constraint).
    pub status: String,
    pub created_by: Uuid,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id"
    )]
    Project,
    #[sea_orm(has_many = "super::quotation_line_item::Entity")]
    QuotationLineItem,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl Related<super::quotation_line_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::QuotationLineItem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
