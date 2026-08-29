use sea_orm::entity::prelude::*;

/// A session belongs to exactly one of `user_id` (internal user) or
/// `client_user_id` (external Client Portal user) — enforced by a DB CHECK
/// constraint. This is the "same session mechanism, scoped differently"
/// design from .ai/decisions/current/2026-08-27-auth-session-based-single-login.md.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub client_user_id: Option<Uuid>,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub created_at: DateTimeWithTimeZone,
    pub expires_at: DateTimeWithTimeZone,
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
        belongs_to = "super::client_user::Entity",
        from = "Column::ClientUserId",
        to = "super::client_user::Column::Id"
    )]
    ClientUser,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::client_user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClientUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
