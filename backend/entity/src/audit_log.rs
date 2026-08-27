use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "audit_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub tenant_id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    /// One of: "create", "update", "delete" (enforced by a DB CHECK constraint).
    pub action: String,
    pub actor_user_id: Option<Uuid>,
    pub actor_client_user_id: Option<Uuid>,
    pub before_data: Option<Json>,
    pub after_data: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
