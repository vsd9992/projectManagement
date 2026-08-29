use sea_orm::entity::prelude::*;

/// In-app only (no email/SMS integration exists in this app yet) — see
/// .ai/decisions/current/2026-08-28-phase-3-audit-and-expansion.md.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[sea_orm(table_name = "notifications")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub recipient_user_id: Uuid,
    pub project_id: Option<Uuid>,
    pub schedule_task_id: Option<Uuid>,
    pub message: String,
    pub is_read: bool,
    pub created_at: DateTimeWithTimeZone,
    pub read_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
