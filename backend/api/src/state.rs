use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    /// RLS-enforced connection pool used for all normal, tenant-scoped work.
    pub app_db: DatabaseConnection,
    /// BYPASSRLS connection pool used only where no tenant context can exist
    /// yet (login-time lookup by email) or for future platform-admin tooling.
    /// Never used for ordinary request handling.
    pub admin_db: DatabaseConnection,
    /// Whether session cookies carry the `Secure` flag — see
    /// `config::AppConfig::cookie_secure`.
    pub cookie_secure: bool,
    /// Allowed CORS origin for the frontend dev server — see
    /// `config::AppConfig::cors_origin`.
    pub cors_origin: String,
}
