use sea_orm::entity::prelude::*;

/// Intentionally NOT tenant-scoped — a platform admin operates above every
/// tenant (pause/resume/delete lifecycle only, no access to tenant business
/// data). No RLS: there's no tenant_id to scope by. Enforcement is that
/// only routes::platform ever queries this table.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[sea_orm(table_name = "platform_admins")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::platform_admin_session::Entity")]
    PlatformAdminSession,
}

impl Related<super::platform_admin_session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PlatformAdminSession.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
