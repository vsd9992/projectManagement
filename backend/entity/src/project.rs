use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub business_unit_id: Uuid,
    pub client_id: Uuid,
    pub name: String,
    /// One of: "milestone", "progressive" (DB CHECK constraint) — which
    /// billing method this project's invoices use. See
    /// .ai/decisions/current/2026-08-28-phase-3-audit-and-expansion.md.
    pub billing_method: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
    #[sea_orm(
        belongs_to = "super::business_unit::Entity",
        from = "Column::BusinessUnitId",
        to = "super::business_unit::Column::Id"
    )]
    BusinessUnit,
    #[sea_orm(
        belongs_to = "super::client::Entity",
        from = "Column::ClientId",
        to = "super::client::Column::Id"
    )]
    Client,
    #[sea_orm(has_many = "super::project_workstream::Entity")]
    ProjectWorkstream,
}

impl Related<super::business_unit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BusinessUnit.def()
    }
}

impl Related<super::client::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Client.def()
    }
}

impl Related<super::project_workstream::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProjectWorkstream.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
