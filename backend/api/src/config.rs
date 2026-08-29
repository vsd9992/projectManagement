#[derive(Clone)]
pub struct AppConfig {
    /// Connection string for the RLS-enforced application role (no BYPASSRLS).
    pub database_url_app: String,
    /// Connection string for the elevated role used only for platform-admin /
    /// cross-tenant lookups (e.g. resolving a user by email at login time,
    /// before a tenant context can be established). Must have BYPASSRLS.
    pub database_url_admin: String,
    pub bind_addr: String,
    /// Whether session cookies carry the `Secure` flag. Defaults to `true`
    /// (matches the original hardcoded behavior) — set `COOKIE_SECURE=false`
    /// in a plain-HTTP dev environment, since a browser silently refuses to
    /// store a `Secure` cookie over a non-HTTPS connection.
    pub cookie_secure: bool,
    /// Allowed CORS origin for the frontend dev server (credentialed CORS
    /// requires an explicit origin, not a wildcard). Set `CORS_ORIGIN` if
    /// devMachine's LAN IP ever changes from the default.
    pub cors_origin: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url_app: std::env::var("DATABASE_URL_APP")
                .map_err(|_| anyhow::anyhow!("DATABASE_URL_APP not set"))?,
            database_url_admin: std::env::var("DATABASE_URL_ADMIN")
                .map_err(|_| anyhow::anyhow!("DATABASE_URL_ADMIN not set"))?,
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            cookie_secure: std::env::var("COOKIE_SECURE")
                .map(|v| v != "false")
                .unwrap_or(true),
            cors_origin: std::env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "http://192.168.1.4:5173".to_string()),
        })
    }
}
