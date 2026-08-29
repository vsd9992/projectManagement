use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
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
    #[schema(value_type = Option<serde_json::Value>)]
    pub before_data: Option<Json>,
    #[schema(value_type = Option<serde_json::Value>)]
    pub after_data: Option<Json>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
