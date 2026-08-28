use api::{auth::password, build_app, state::AppState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

pub struct TestApp {
    router: Router,
    /// Kept for test-only setup that has no HTTP path by design — seeding a
    /// platform admin (there is deliberately no self-service signup for
    /// that account type; see backend/api/src/bin/create_platform_admin.rs).
    admin_db: DatabaseConnection,
}

/// Connects to the dedicated test database (never the dev-server's main
/// `project_management` DB) and builds the real app router — tests exercise
/// exactly the same code path as production, just with test data.
pub async fn spawn_app() -> TestApp {
    let app_url = std::env::var("TEST_DATABASE_URL_APP")
        .expect("TEST_DATABASE_URL_APP must be set to run integration tests");
    let admin_url = std::env::var("TEST_DATABASE_URL_ADMIN")
        .expect("TEST_DATABASE_URL_ADMIN must be set to run integration tests");
    let app_db = Database::connect(&app_url)
        .await
        .expect("connect TEST_DATABASE_URL_APP");
    let admin_db = Database::connect(&admin_url)
        .await
        .expect("connect TEST_DATABASE_URL_ADMIN");
    TestApp {
        router: build_app(AppState {
            app_db,
            admin_db: admin_db.clone(),
        }),
        admin_db,
    }
}

pub struct TestResponse {
    pub status: StatusCode,
    pub json: Value,
    /// The `session_token` cookie, if this response set one.
    pub cookie: Option<String>,
    raw_set_cookies: Vec<String>,
}

impl TestResponse {
    /// Looks up any Set-Cookie by name — needed for `platform_session_token`,
    /// which uses a different cookie name than the tenant `session_token`
    /// (deliberately, so the two session types can never be confused).
    pub fn cookie_named(&self, name: &str) -> Option<String> {
        self.raw_set_cookies.iter().find_map(|v| {
            v.split(';')
                .next()
                .and_then(|kv| kv.strip_prefix(&format!("{name}=")))
                .map(|s| s.to_string())
        })
    }
}

impl TestApp {
    pub async fn call(
        &self,
        method: &str,
        path: &str,
        cookie: Option<&str>,
        body: Value,
    ) -> TestResponse {
        self.call_with_cookie_header(
            method,
            path,
            cookie.map(|c| format!("session_token={c}")).as_deref(),
            body,
        )
        .await
    }

    /// Same as `call`, but takes a fully-formed `Cookie` header value —
    /// needed for the platform-admin cookie, which has a different name.
    pub async fn call_with_cookie_header(
        &self,
        method: &str,
        path: &str,
        cookie_header: Option<&str>,
        body: Value,
    ) -> TestResponse {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json");
        if let Some(c) = cookie_header {
            builder = builder.header("cookie", c.to_string());
        }
        let req = builder.body(Body::from(body.to_string())).unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let raw_set_cookies: Vec<String> = resp
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        let cookie = raw_set_cookies.iter().find_map(|v| {
            v.split(';')
                .next()
                .and_then(|kv| kv.strip_prefix("session_token="))
                .map(|s| s.to_string())
        });
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        TestResponse {
            status,
            json,
            cookie,
            raw_set_cookies,
        }
    }
}

pub fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@test.example", Uuid::new_v4())
}

/// Signs up a fresh tenant + owner user. Returns (session cookie, tenant_id, user_id).
pub async fn signup(app: &TestApp, prefix: &str) -> (String, Uuid, Uuid) {
    let resp = app
        .call(
            "POST",
            "/auth/signup",
            None,
            json!({
                "tenant_name": format!("{prefix} Co"),
                "email": unique_email(prefix),
                "password": "correcthorsebattery",
            }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "signup failed: {:?}", resp.json);
    let cookie = resp.cookie.expect("signup did not set a session cookie");
    let tenant_id: Uuid = resp.json["tenant_id"].as_str().unwrap().parse().unwrap();
    let user_id: Uuid = resp.json["user_id"].as_str().unwrap().parse().unwrap();
    (cookie, tenant_id, user_id)
}

pub async fn create_business_unit(app: &TestApp, cookie: &str, name: &str) -> Uuid {
    let resp = app
        .call(
            "POST",
            "/business-units",
            Some(cookie),
            json!({ "name": name }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "create BU failed: {:?}", resp.json);
    resp.json["id"].as_str().unwrap().parse().unwrap()
}

pub async fn assign_role(app: &TestApp, cookie: &str, bu_id: Uuid, user_id: Uuid, role: &str) {
    let resp = app
        .call(
            "POST",
            &format!("/business-units/{bu_id}/roles"),
            Some(cookie),
            json!({ "user_id": user_id, "role": role }),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "assign_role({role}) failed: {:?}",
        resp.json
    );
}

pub async fn create_client(app: &TestApp, cookie: &str, name: &str) -> Uuid {
    let resp = app
        .call("POST", "/clients", Some(cookie), json!({ "name": name }))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "create client failed: {:?}", resp.json);
    resp.json["id"].as_str().unwrap().parse().unwrap()
}

/// Creates a teammate (non-admin, by construction — see auth::create_teammate)
/// via `owner_cookie` and logs them in. Returns (cookie, user_id).
pub async fn create_and_login_teammate(app: &TestApp, owner_cookie: &str, prefix: &str) -> (String, Uuid) {
    let email = unique_email(prefix);
    let resp = app
        .call(
            "POST",
            "/users",
            Some(owner_cookie),
            json!({ "email": email, "password": "correcthorsebattery" }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "create teammate failed: {:?}", resp.json);
    let user_id: Uuid = resp.json["user_id"].as_str().unwrap().parse().unwrap();

    let login = app
        .call(
            "POST",
            "/auth/login",
            None,
            json!({ "email": email, "password": "correcthorsebattery" }),
        )
        .await;
    assert_eq!(login.status, StatusCode::OK, "teammate login failed: {:?}", login.json);
    let cookie = login.cookie.expect("login did not set a cookie");
    (cookie, user_id)
}

/// Seeds a platform admin directly in the test database — there is no HTTP
/// path for this by design (see backend/api/src/bin/create_platform_admin.rs),
/// so tests have to go around the API for this one piece of setup.
pub async fn seed_platform_admin(app: &TestApp, email: &str, plaintext_password: &str) {
    let hash = password::hash_password(plaintext_password).expect("hash password");
    let am = entity::platform_admin::ActiveModel {
        id: Set(Uuid::new_v4()),
        email: Set(email.to_string()),
        password_hash: Set(hash),
        created_at: Set(chrono::Utc::now().into()),
    };
    am.insert(&app.admin_db).await.expect("insert platform admin");
}

pub async fn platform_login(app: &TestApp, email: &str, password: &str) -> String {
    let resp = app
        .call(
            "POST",
            "/platform/auth/login",
            None,
            json!({ "email": email, "password": password }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "platform login failed: {:?}", resp.json);
    resp.cookie_named("platform_session_token")
        .expect("platform login did not set a cookie")
}

pub async fn call_as_platform(
    app: &TestApp,
    method: &str,
    path: &str,
    platform_cookie: &str,
    body: Value,
) -> TestResponse {
    app.call_with_cookie_header(
        method,
        path,
        Some(&format!("platform_session_token={platform_cookie}")),
        body,
    )
    .await
}

pub async fn create_project(
    app: &TestApp,
    cookie: &str,
    bu_id: Uuid,
    client_id: Uuid,
    name: &str,
) -> TestResponse {
    // All four workstreams enabled by default so this generic fixture works
    // for any workstream-specific endpoint under test (workstream membership
    // is enforced at the API layer — see .ai/decisions/current/
    // 2026-08-28-workstream-enforcement-and-expansion.md). A test that cares
    // about a restricted workstream set should use
    // create_project_with_workstreams instead.
    create_project_with_workstreams(
        app,
        cookie,
        bu_id,
        client_id,
        name,
        &["design", "manufacturing", "procurement", "site_execution"],
    )
    .await
}

pub async fn create_project_with_workstreams(
    app: &TestApp,
    cookie: &str,
    bu_id: Uuid,
    client_id: Uuid,
    name: &str,
    workstreams: &[&str],
) -> TestResponse {
    app.call(
        "POST",
        "/projects",
        Some(cookie),
        json!({
            "name": name,
            "business_unit_id": bu_id,
            "client_id": client_id,
            "workstreams": workstreams,
        }),
    )
    .await
}

/// Creates a client user via `owner_cookie`, then logs them in through the
/// Client Portal. Returns (session cookie, client_user_id).
pub async fn create_and_login_client_user(
    app: &TestApp,
    owner_cookie: &str,
    client_id: Uuid,
    prefix: &str,
) -> (String, Uuid) {
    let email = unique_email(prefix);
    let resp = app
        .call(
            "POST",
            &format!("/clients/{client_id}/users"),
            Some(owner_cookie),
            json!({ "email": email, "password": "correcthorsebattery" }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "create client user failed: {:?}", resp.json);
    let client_user_id: Uuid = resp.json["id"].as_str().unwrap().parse().unwrap();

    let login = app
        .call(
            "POST",
            "/auth/client-login",
            None,
            json!({ "email": email, "password": "correcthorsebattery" }),
        )
        .await;
    assert_eq!(login.status, StatusCode::OK, "client login failed: {:?}", login.json);
    let cookie = login.cookie.expect("client login did not set a cookie");
    (cookie, client_user_id)
}
