use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter, DeriveActiveEnum, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "workstream_type")]
#[serde(rename_all = "snake_case")]
pub enum WorkstreamType {
    #[sea_orm(string_value = "design")]
    Design,
    #[sea_orm(string_value = "manufacturing")]
    Manufacturing,
    #[sea_orm(string_value = "procurement")]
    Procurement,
    #[sea_orm(string_value = "site_execution")]
    SiteExecution,
}
