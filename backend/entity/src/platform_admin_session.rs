use sea_orm::entity::prelude::*;

/// Deliberately separate from the tenant `sessions` table — see
/// platform_admin.rs and the migration that created this table for why.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[sea_orm(table_name = "platform_admin_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub platform_admin_id: Uuid,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub created_at: DateTimeWithTimeZone,
    pub expires_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::platform_admin::Entity",
        from = "Column::PlatformAdminId",
        to = "super::platform_admin::Column::Id"
    )]
    PlatformAdmin,
}

impl Related<super::platform_admin::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PlatformAdmin.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
