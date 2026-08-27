use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    /// RLS-enforced connection pool used for all normal, tenant-scoped work.
    pub app_db: DatabaseConnection,
    /// BYPASSRLS connection pool used only where no tenant context can exist
    /// yet (login-time lookup by email) or for future platform-admin tooling.
    /// Never used for ordinary request handling.
    pub admin_db: DatabaseConnection,
}
