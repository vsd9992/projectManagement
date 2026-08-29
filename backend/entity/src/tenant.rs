use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[sea_orm(table_name = "tenants")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTimeWithTimeZone,
    /// Which regional tax/billing rule profile this tenant uses. Only
    /// "india" is implemented (see api::billing).
    pub region_profile: String,
    /// One of: "active", "paused", "deleted" (DB CHECK constraint). Managed
    /// only by platform admins (see routes::platform), never by tenant
    /// users. Every session lookup checks this and rejects non-active
    /// tenants.
    pub status: String,
    pub paused_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    /// Per-tenant override of the 4 workstream types' display names, e.g.
    /// `{"site_execution": "Installation"}`. Arbitrary JSON object, not
    /// validated against the workstream catalog — a purely cosmetic label,
    /// never used for authorization/routing logic.
    pub workstream_labels: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::business_unit::Entity")]
    BusinessUnit,
    #[sea_orm(has_many = "super::user::Entity")]
    User,
    #[sea_orm(has_many = "super::client::Entity")]
    Client,
}

impl Related<super::business_unit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BusinessUnit.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
