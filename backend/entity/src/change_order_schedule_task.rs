use sea_orm::entity::prelude::*;
use super::workstream_type::WorkstreamType;

/// Staged schedule tasks a Change Order requests spawning alongside (or
/// instead of) BOQ changes — materialized into `schedule_tasks` only once
/// the client approves, mirroring `change_order_workstream`'s pattern.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[schema(as = ChangeOrderScheduleTaskModel)]
#[sea_orm(table_name = "change_order_schedule_tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub change_order_id: Uuid,
    pub title: String,
    pub workstream_type: WorkstreamType,
    #[schema(value_type = Option<String>, format = Date)]
    pub planned_start_date: Option<Date>,
    #[schema(value_type = Option<String>, format = Date)]
    pub planned_end_date: Option<Date>,
    pub depends_on_existing_task_id: Option<Uuid>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::change_order::Entity",
        from = "Column::ChangeOrderId",
        to = "super::change_order::Column::Id"
    )]
    ChangeOrder,
}

impl Related<super::change_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChangeOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
