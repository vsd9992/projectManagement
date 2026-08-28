mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;

/// Covers the RBAC gap fixed in .ai/decisions/current/2026-08-28-no-rbac-enforcement-yet.md:
/// business-unit membership + role must actually be checked, not just
/// assumed from "role data exists somewhere."
#[tokio::test]
async fn lead_creation_requires_sales_design_role() {
    let app = spawn_app().await;
    let (owner_cookie, _tenant_id, owner_id) = signup(&app, "leadtest-owner").await;
    let bu_id = create_business_unit(&app, &owner_cookie, "HQ").await;
    let client_id = create_client(&app, &owner_cookie, "Acme").await;

    // Wrong role: delivery, not sales_design.
    assign_role(&app, &owner_cookie, bu_id, owner_id, "delivery").await;

    let resp = app
        .call(
            "POST",
            "/leads",
            Some(&owner_cookie),
            json!({ "business_unit_id": bu_id, "client_id": client_id, "title": "New enquiry" }),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "delivery role should not be able to create a lead: {:?}",
        resp.json
    );

    // Now grant the right role — same call should succeed.
    assign_role(&app, &owner_cookie, bu_id, owner_id, "sales_design").await;
    let resp = app
        .call(
            "POST",
            "/leads",
            Some(&owner_cookie),
            json!({ "business_unit_id": bu_id, "client_id": client_id, "title": "New enquiry" }),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "sales_design role should be able to create a lead: {:?}",
        resp.json
    );
}

#[tokio::test]
async fn purchase_order_creation_requires_delivery_role() {
    let app = spawn_app().await;
    let (owner_cookie, _tenant_id, owner_id) = signup(&app, "potest-owner").await;
    let bu_id = create_business_unit(&app, &owner_cookie, "HQ").await;
    let client_id = create_client(&app, &owner_cookie, "Acme").await;

    // sales_design gives BU membership (so project creation works) but not
    // the delivery role purchase orders require.
    assign_role(&app, &owner_cookie, bu_id, owner_id, "sales_design").await;
    let project = create_project(&app, &owner_cookie, bu_id, client_id, "Test Project").await;
    assert_eq!(project.status, StatusCode::OK, "{:?}", project.json);
    let project_id = project.json["id"].as_str().unwrap();

    let vendor_resp = app
        .call(
            "POST",
            "/vendors",
            Some(&owner_cookie),
            json!({ "name": "Test Vendor" }),
        )
        .await;
    assert_eq!(vendor_resp.status, StatusCode::OK);
    let vendor_id = vendor_resp.json["id"].as_str().unwrap();

    let po_body = json!({
        "vendor_id": vendor_id,
        "title": "Test PO",
        "line_items": [{ "description": "Widgets", "quantity": "1", "unit": "nos", "unit_rate": "100" }],
    });

    let resp = app
        .call(
            "POST",
            &format!("/projects/{project_id}/purchase-orders"),
            Some(&owner_cookie),
            po_body.clone(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "sales_design role should not be able to raise a PO: {:?}",
        resp.json
    );

    assign_role(&app, &owner_cookie, bu_id, owner_id, "delivery").await;
    let resp = app
        .call(
            "POST",
            &format!("/projects/{project_id}/purchase-orders"),
            Some(&owner_cookie),
            po_body,
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "delivery role should be able to raise a PO: {:?}",
        resp.json
    );
}

#[tokio::test]
async fn invoice_creation_requires_finance_role() {
    let app = spawn_app().await;
    let (owner_cookie, _tenant_id, owner_id) = signup(&app, "invtest-owner").await;
    let bu_id = create_business_unit(&app, &owner_cookie, "HQ").await;
    let client_id = create_client(&app, &owner_cookie, "Acme").await;

    // "delivery" gives BU membership (milestones only need membership) but
    // not the finance role invoices require.
    assign_role(&app, &owner_cookie, bu_id, owner_id, "delivery").await;
    let project = create_project(&app, &owner_cookie, bu_id, client_id, "Test Project").await;
    let project_id = project.json["id"].as_str().unwrap();

    let ms_resp = app
        .call(
            "POST",
            &format!("/projects/{project_id}/milestones"),
            Some(&owner_cookie),
            json!({ "title": "Phase 1 complete" }),
        )
        .await;
    assert_eq!(ms_resp.status, StatusCode::OK, "{:?}", ms_resp.json);
    let milestone_id = ms_resp.json["id"].as_str().unwrap();

    let complete_resp = app
        .call(
            "POST",
            &format!("/milestones/{milestone_id}/complete"),
            Some(&owner_cookie),
            json!({}),
        )
        .await;
    assert_eq!(complete_resp.status, StatusCode::OK, "{:?}", complete_resp.json);

    let invoice_body = json!({
        "milestone_id": milestone_id,
        "base_amount": "100000",
        "retention_percent": "5",
    });

    let resp = app
        .call(
            "POST",
            &format!("/projects/{project_id}/invoices"),
            Some(&owner_cookie),
            invoice_body.clone(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "delivery role should not be able to raise an invoice: {:?}",
        resp.json
    );

    assign_role(&app, &owner_cookie, bu_id, owner_id, "finance").await;
    let resp = app
        .call(
            "POST",
            &format!("/projects/{project_id}/invoices"),
            Some(&owner_cookie),
            invoice_body,
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "finance role should be able to raise an invoice: {:?}",
        resp.json
    );
    // Regression check on the India GST/TDS/retention math while we're here.
    assert_eq!(resp.json["gst_amount"], "18000.00");
    assert_eq!(resp.json["gst_tds_amount"], "2000.00");
    assert_eq!(resp.json["retention_amount"], "5000.00");
    assert_eq!(resp.json["net_payable"], "111000.00");
}

/// Confirms the actual point of the RBAC gap: a tenant with two business
/// units (two teams) keeps each team scoped to its own projects, and
/// roll-up-across-everything is not the default.
#[tokio::test]
async fn project_visibility_is_scoped_to_business_unit_membership() {
    let app = spawn_app().await;
    let (owner_cookie, _tenant_id, owner_id) = signup(&app, "scopetest-owner").await;
    let bu1 = create_business_unit(&app, &owner_cookie, "Branch 1").await;
    let bu2 = create_business_unit(&app, &owner_cookie, "Branch 2").await;
    let client_id = create_client(&app, &owner_cookie, "Acme").await;
    assign_role(&app, &owner_cookie, bu1, owner_id, "sales_design").await;

    // A teammate for branch 2, in the same tenant.
    let teammate_email = unique_email("scopetest-teammate");
    let create_resp = app
        .call(
            "POST",
            "/users",
            Some(&owner_cookie),
            json!({ "email": teammate_email, "password": "correcthorsebattery" }),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::OK, "{:?}", create_resp.json);
    let teammate_id: uuid::Uuid = create_resp.json["user_id"].as_str().unwrap().parse().unwrap();

    let login_resp = app
        .call(
            "POST",
            "/auth/login",
            None,
            json!({ "email": teammate_email, "password": "correcthorsebattery" }),
        )
        .await;
    assert_eq!(login_resp.status, StatusCode::OK);
    let teammate_cookie = login_resp.cookie.expect("login did not set a cookie");
    assign_role(&app, &owner_cookie, bu2, teammate_id, "sales_design").await;

    let p1 = create_project(&app, &owner_cookie, bu1, client_id, "Branch 1 Project").await;
    assert_eq!(p1.status, StatusCode::OK, "{:?}", p1.json);
    let p2 = create_project(&app, &teammate_cookie, bu2, client_id, "Branch 2 Project").await;
    assert_eq!(p2.status, StatusCode::OK, "{:?}", p2.json);
    let p2_id = p2.json["id"].as_str().unwrap();

    let owner_list = app.call("GET", "/projects", Some(&owner_cookie), json!({})).await;
    let names: Vec<&str> = owner_list.json.as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"Branch 1 Project"), "owner should see their own branch's project: {names:?}");
    assert!(!names.contains(&"Branch 2 Project"), "owner should NOT see the other branch's project: {names:?}");

    let direct_get = app
        .call("GET", &format!("/projects/{p2_id}"), Some(&owner_cookie), json!({}))
        .await;
    assert_eq!(
        direct_get.status,
        StatusCode::FORBIDDEN,
        "owner should not be able to fetch branch 2's project directly: {:?}",
        direct_get.json
    );
}
