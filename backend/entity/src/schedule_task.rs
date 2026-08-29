use sea_orm::entity::prelude::*;

/// Generalizes the old site-execution-only task+dependency model across all
/// four workstreams, with planned/actual dates. Every `site_task` has a
/// linked row here (`site_task_id` set) — dependency data lives exclusively
/// in `schedule_task_dependencies` now, not on the leaf entities. See
/// .ai/decisions/current/2026-08-28-phase-3-audit-and-expansion.md.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[sea_orm(table_name = "schedule_tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub project_id: Uuid,
    pub workstream_type: super::workstream_type::WorkstreamType,
    pub title: String,
    /// One of: "not_started", "in_progress", "done" (DB CHECK constraint).
    pub status: String,
    pub planned_start_date: Option<Date>,
    pub planned_end_date: Option<Date>,
    pub actual_start_date: Option<Date>,
    pub actual_end_date: Option<Date>,
    pub site_task_id: Option<Uuid>,
    pub production_task_id: Option<Uuid>,
    pub design_revision_id: Option<Uuid>,
    pub purchase_order_id: Option<Uuid>,
    pub spawned_by_change_order_id: Option<Uuid>,
    pub created_by: Uuid,
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
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
