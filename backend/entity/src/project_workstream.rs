use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "project_workstreams")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub project_id: Uuid,
    pub workstream_type: super::workstream_type::WorkstreamType,
    /// Free-form status for M1 (e.g. "not_started", "in_progress", "done"). A real
    /// per-workstream state machine is a later milestone, not M1 scope.
    pub status: String,
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
