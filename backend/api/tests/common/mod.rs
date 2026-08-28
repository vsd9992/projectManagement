use api::{build_app, state::AppState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use sea_orm::Database;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

pub struct TestApp {
    router: Router,
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
        router: build_app(AppState { app_db, admin_db }),
    }
}

pub struct TestResponse {
    pub status: StatusCode,
    pub json: Value,
    pub cookie: Option<String>,
}

impl TestApp {
    pub async fn call(
        &self,
        method: &str,
        path: &str,
        cookie: Option<&str>,
        body: Value,
    ) -> TestResponse {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json");
        if let Some(c) = cookie {
            builder = builder.header("cookie", format!("session_token={c}"));
        }
        let req = builder.body(Body::from(body.to_string())).unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let cookie = resp
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .and_then(|kv| kv.strip_prefix("session_token="))
            .map(|s| s.to_string());
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

pub async fn create_project(
    app: &TestApp,
    cookie: &str,
    bu_id: Uuid,
    client_id: Uuid,
    name: &str,
) -> TestResponse {
    app.call(
        "POST",
        "/projects",
        Some(cookie),
        json!({
            "name": name,
            "business_unit_id": bu_id,
            "client_id": client_id,
            "workstreams": ["design"],
        }),
    )
    .await
}
