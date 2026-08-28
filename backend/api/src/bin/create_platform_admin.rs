//! One-time bootstrap for the first platform admin. Deliberately a CLI, not
//! an HTTP endpoint — platform admins can pause/resume/delete any tenant,
//! so there is no self-service signup for this account type. Run manually
//! by a trusted operator on the target environment.
//!
//! Usage: PLATFORM_ADMIN_PASSWORD=... cargo run --bin create_platform_admin -- <email>
//! (password read from an env var, not a CLI arg, so it never lands in
//! shell history or a process listing)

use api::{auth::password, config::AppConfig};
use sea_orm::{ActiveModelTrait, Database, Set};
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let email = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: create_platform_admin <email>"))?;
    let plaintext_password = std::env::var("PLATFORM_ADMIN_PASSWORD")
        .map_err(|_| anyhow::anyhow!("PLATFORM_ADMIN_PASSWORD env var must be set"))?;
    if plaintext_password.len() < 8 {
        anyhow::bail!("password must be at least 8 characters");
    }

    let config = AppConfig::from_env()?;
    let admin_db = Database::connect(&config.database_url_admin).await?;

    let password_hash = password::hash_password(&plaintext_password)?;
    let am = entity::platform_admin::ActiveModel {
        id: Set(Uuid::new_v4()),
        email: Set(email.clone()),
        password_hash: Set(password_hash),
        created_at: Set(chrono::Utc::now().into()),
    };
    am.insert(&admin_db).await?;

    println!("Platform admin created: {email}");
    Ok(())
}
