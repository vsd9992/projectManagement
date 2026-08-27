use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "user_business_unit_roles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub business_unit_id: Uuid,
    /// One of: "sales_design", "delivery", "finance" (enforced by a DB CHECK constraint).
    pub role: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::business_unit::Entity",
        from = "Column::BusinessUnitId",
        to = "super::business_unit::Column::Id"
    )]
    BusinessUnit,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::business_unit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BusinessUnit.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
