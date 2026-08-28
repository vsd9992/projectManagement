use sea_orm::entity::prelude::*;

/// Composite-key join table: `task_id` depends on `depends_on_task_id`. No
/// `Related` impls (both FKs point at the same `schedule_task::Entity`,
/// and Rust disallows two `Related<T>` impls for the same target) —
/// queries filter this table directly instead of traversing a relation.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "schedule_task_dependencies")]
pub struct Model {
    pub tenant_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub task_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub depends_on_task_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
