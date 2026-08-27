use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "leads")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub business_unit_id: Uuid,
    pub client_id: Uuid,
    pub title: String,
    /// One of: "new", "qualified", "converted", "lost" (DB CHECK constraint).
    pub status: String,
    pub converted_project_id: Option<Uuid>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::business_unit::Entity",
        from = "Column::BusinessUnitId",
        to = "super::business_unit::Column::Id"
    )]
    BusinessUnit,
    #[sea_orm(
        belongs_to = "super::client::Entity",
        from = "Column::ClientId",
        to = "super::client::Column::Id"
    )]
    Client,
}

impl Related<super::business_unit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BusinessUnit.def()
    }
}

impl Related<super::client::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Client.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
