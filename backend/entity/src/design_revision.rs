use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[schema(as = DesignRevisionModel)]
#[sea_orm(table_name = "design_revisions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub design_asset_id: Uuid,
    pub version: i32,
    pub notes: Option<String>,
    /// One of: "submitted", "approved", "rejected" (DB CHECK constraint).
    pub status: String,
    pub submitted_by: Uuid,
    #[schema(value_type = String, format = DateTime)]
    pub submitted_at: DateTimeWithTimeZone,
    pub decided_by: Option<Uuid>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub decided_at: Option<DateTimeWithTimeZone>,
    pub decision_notes: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::design_asset::Entity",
        from = "Column::DesignAssetId",
        to = "super::design_asset::Column::Id"
    )]
    DesignAsset,
}

impl Related<super::design_asset::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DesignAsset.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
