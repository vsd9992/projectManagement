mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;

/// Covers the RBAC gap fixed in .ai/decisions/current/2026-08-28-rbac-business-unit-scoping-implemented.md:
/// business-unit membership + role must actually be checked for ordinary
/// (non-admin) users, not just assumed from "role data exists somewhere."
/// Uses a teammate, not the signing-up owner, because the owner is always
/// a tenant admin and tenant admins bypass role checks by design.
#[tokio::test]
async fn lead_creation_requires_sales_design_role() {
    let app = spawn_app().await;
    let (owner_cookie, _tenant_id, _owner_id) = signup(&app, "leadtest-owner").await;
    let bu_id = create_business_unit(&app, &owner_cookie, "HQ").await;
    let client_id = create_client(&app, &owner_cookie, "Acme").await;
    let (teammate_cookie, teammate_id) =
        create_and_login_teammate(&app, &owner_cookie, "leadtest-teammate").await;

    // Wrong role: delivery, not sales_design.
    assign_role(&app, &owner_cookie, bu_id, teammate_id, "delivery").await;

    let resp = app
        .call(
            "POST",
            "/leads",
            Some(&teammate_cookie),
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
    assign_role(&app, &owner_cookie, bu_id, teammate_id, "sales_design").await;
    let resp = app
        .call(
            "POST",
            "/leads",
            Some(&teammate_cookie),
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
    let (owner_cookie, _tenant_id, _owner_id) = signup(&app, "potest-owner").await;
    let bu_id = create_business_unit(&app, &owner_cookie, "HQ").await;
    let client_id = create_client(&app, &owner_cookie, "Acme").await;
    let (teammate_cookie, teammate_id) =
        create_and_login_teammate(&app, &owner_cookie, "potest-teammate").await;

    // sales_design gives BU membership (so project creation works) but not
    // the delivery role purchase orders require.
    assign_role(&app, &owner_cookie, bu_id, teammate_id, "sales_design").await;
    let project = create_project(&app, &teammate_cookie, bu_id, client_id, "Test Project").await;
    assert_eq!(project.status, StatusCode::OK, "{:?}", project.json);
    let project_id = project.json["id"].as_str().unwrap();

    let vendor_resp = app
        .call(
            "POST",
            "/vendors",
            Some(&teammate_cookie),
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
            Some(&teammate_cookie),
            po_body.clone(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "sales_design role should not be able to raise a PO: {:?}",
        resp.json
    );

    assign_role(&app, &owner_cookie, bu_id, teammate_id, "delivery").await;
    let resp = app
        .call(
            "POST",
            &format!("/projects/{project_id}/purchase-orders"),
            Some(&teammate_cookie),
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
    let (owner_cookie, _tenant_id, _owner_id) = signup(&app, "invtest-owner").await;
    let bu_id = create_business_unit(&app, &owner_cookie, "HQ").await;
    let client_id = create_client(&app, &owner_cookie, "Acme").await;
    let (teammate_cookie, teammate_id) =
        create_and_login_teammate(&app, &owner_cookie, "invtest-teammate").await;

    // "delivery" gives BU membership (milestones only need membership) but
    // not the finance role invoices require.
    assign_role(&app, &owner_cookie, bu_id, teammate_id, "delivery").await;
    let project = create_project(&app, &teammate_cookie, bu_id, client_id, "Test Project").await;
    let project_id = project.json["id"].as_str().unwrap();

    let ms_resp = app
        .call(
            "POST",
            &format!("/projects/{project_id}/milestones"),
            Some(&teammate_cookie),
            json!({ "title": "Phase 1 complete" }),
        )
        .await;
    assert_eq!(ms_resp.status, StatusCode::OK, "{:?}", ms_resp.json);
    let milestone_id = ms_resp.json["id"].as_str().unwrap();

    let complete_resp = app
        .call(
            "POST",
            &format!("/milestones/{milestone_id}/complete"),
            Some(&teammate_cookie),
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
            Some(&teammate_cookie),
            invoice_body.clone(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "delivery role should not be able to raise an invoice: {:?}",
        resp.json
    );

    assign_role(&app, &owner_cookie, bu_id, teammate_id, "finance").await;
    let resp = app
        .call(
            "POST",
            &format!("/projects/{project_id}/invoices"),
            Some(&teammate_cookie),
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
/// units (two teams) keeps each ordinary team member scoped to their own
/// projects. The tenant admin (owner), by contrast, gets roll-up visibility
/// across both — that's the "tenant owners get roll-up visibility" behavior
/// tenant-admin status is supposed to provide.
#[tokio::test]
async fn project_visibility_is_scoped_to_business_unit_membership() {
    let app = spawn_app().await;
    let (owner_cookie, _tenant_id, _owner_id) = signup(&app, "scopetest-owner").await;
    let bu1 = create_business_unit(&app, &owner_cookie, "Branch 1").await;
    let bu2 = create_business_unit(&app, &owner_cookie, "Branch 2").await;
    let client_id = create_client(&app, &owner_cookie, "Acme").await;

    let (teammate1_cookie, teammate1_id) =
        create_and_login_teammate(&app, &owner_cookie, "scopetest-t1").await;
    let (teammate2_cookie, teammate2_id) =
        create_and_login_teammate(&app, &owner_cookie, "scopetest-t2").await;
    assign_role(&app, &owner_cookie, bu1, teammate1_id, "sales_design").await;
    assign_role(&app, &owner_cookie, bu2, teammate2_id, "sales_design").await;

    let p1 = create_project(&app, &teammate1_cookie, bu1, client_id, "Branch 1 Project").await;
    assert_eq!(p1.status, StatusCode::OK, "{:?}", p1.json);
    let p2 = create_project(&app, &teammate2_cookie, bu2, client_id, "Branch 2 Project").await;
    assert_eq!(p2.status, StatusCode::OK, "{:?}", p2.json);
    let p2_id = p2.json["id"].as_str().unwrap();

    let t1_list = app.call("GET", "/projects", Some(&teammate1_cookie), json!({})).await;
    let t1_names: Vec<&str> = t1_list.json.as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert!(t1_names.contains(&"Branch 1 Project"), "teammate1 should see their own branch's project: {t1_names:?}");
    assert!(!t1_names.contains(&"Branch 2 Project"), "teammate1 should NOT see the other branch's project: {t1_names:?}");

    let direct_get = app
        .call("GET", &format!("/projects/{p2_id}"), Some(&teammate1_cookie), json!({}))
        .await;
    assert_eq!(
        direct_get.status,
        StatusCode::FORBIDDEN,
        "teammate1 should not be able to fetch branch 2's project directly: {:?}",
        direct_get.json
    );

    // The owner (tenant admin) sees both — roll-up visibility.
    let owner_list = app.call("GET", "/projects", Some(&owner_cookie), json!({})).await;
    let owner_names: Vec<&str> = owner_list.json.as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert!(owner_names.contains(&"Branch 1 Project"), "admin should see branch 1's project: {owner_names:?}");
    assert!(owner_names.contains(&"Branch 2 Project"), "admin should see branch 2's project: {owner_names:?}");
}

/// Org-management actions (create BU, assign roles, add teammates, revoke
/// sessions) are tenant-admin only. Also covers promotion/demotion and the
/// last-admin protection.
#[tokio::test]
async fn org_management_is_tenant_admin_only() {
    let app = spawn_app().await;
    let (owner_cookie, _tenant_id, owner_id) = signup(&app, "admintest-owner").await;
    let (teammate_cookie, teammate_id) =
        create_and_login_teammate(&app, &owner_cookie, "admintest-teammate").await;

    // Non-admin teammate cannot create a business unit.
    let resp = app
        .call("POST", "/business-units", Some(&teammate_cookie), json!({ "name": "Should fail" }))
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{:?}", resp.json);

    // Non-admin teammate cannot invite another teammate.
    let resp = app
        .call(
            "POST",
            "/users",
            Some(&teammate_cookie),
            json!({ "email": unique_email("admintest-blocked"), "password": "correcthorsebattery" }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{:?}", resp.json);

    // Owner promotes the teammate to admin.
    let promote = app
        .call(
            "POST",
            &format!("/users/{teammate_id}/admin"),
            Some(&owner_cookie),
            json!({ "is_tenant_admin": true }),
        )
        .await;
    assert_eq!(promote.status, StatusCode::OK, "{:?}", promote.json);

    // Now the (former) teammate can create a business unit.
    let resp = app
        .call("POST", "/business-units", Some(&teammate_cookie), json!({ "name": "Now allowed" }))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "promoted admin should be able to create a BU: {:?}", resp.json);

    // Demoting the owner now succeeds (the promoted teammate is still an admin).
    let demote_owner = app
        .call(
            "POST",
            &format!("/users/{owner_id}/admin"),
            Some(&teammate_cookie),
            json!({ "is_tenant_admin": false }),
        )
        .await;
    assert_eq!(demote_owner.status, StatusCode::OK, "{:?}", demote_owner.json);

    // But demoting the last remaining admin (the former teammate) is blocked.
    let demote_last = app
        .call(
            "POST",
            &format!("/users/{teammate_id}/admin"),
            Some(&teammate_cookie),
            json!({ "is_tenant_admin": false }),
        )
        .await;
    assert_eq!(
        demote_last.status,
        StatusCode::BAD_REQUEST,
        "should not be able to demote the tenant's only remaining admin: {:?}",
        demote_last.json
    );
}

/// Platform-manager tier: pause/resume/delete a tenant, and a paused/deleted
/// tenant locks out every one of its users at the session level.
#[tokio::test]
async fn platform_admin_can_pause_resume_delete_tenant() {
    let app = spawn_app().await;
    let (owner_cookie, tenant_id, _owner_id) = signup(&app, "platformtest-owner").await;
    let tenant_id = tenant_id.to_string();

    // Confirm the tenant works before touching platform-level controls.
    let pre = app.call("GET", "/projects", Some(&owner_cookie), json!({})).await;
    assert_eq!(pre.status, StatusCode::OK, "{:?}", pre.json);

    let platform_email = unique_email("platform-admin");
    seed_platform_admin(&app, &platform_email, "platformpassword123").await;
    let platform_cookie = platform_login(&app, &platform_email, "platformpassword123").await;

    // Sanity check: this tenant shows up in the platform admin's tenant list.
    let list = call_as_platform(&app, "GET", "/platform/tenants", &platform_cookie, json!({})).await;
    assert_eq!(list.status, StatusCode::OK, "{:?}", list.json);
    assert!(
        list.json
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"].as_str() == Some(tenant_id.as_str())),
        "signed-up tenant should appear in the platform tenant list: {:?}",
        list.json
    );

    // Pause it — the tenant's own session should now be locked out.
    let pause = call_as_platform(
        &app,
        "POST",
        &format!("/platform/tenants/{tenant_id}/pause"),
        &platform_cookie,
        json!({}),
    )
    .await;
    assert_eq!(pause.status, StatusCode::OK, "{:?}", pause.json);

    let during_pause = app.call("GET", "/projects", Some(&owner_cookie), json!({})).await;
    assert_eq!(
        during_pause.status,
        StatusCode::FORBIDDEN,
        "a paused tenant's session should be locked out: {:?}",
        during_pause.json
    );

    // Resume — access comes back.
    let resume = call_as_platform(
        &app,
        "POST",
        &format!("/platform/tenants/{tenant_id}/resume"),
        &platform_cookie,
        json!({}),
    )
    .await;
    assert_eq!(resume.status, StatusCode::OK, "{:?}", resume.json);
    let after_resume = app.call("GET", "/projects", Some(&owner_cookie), json!({})).await;
    assert_eq!(after_resume.status, StatusCode::OK, "{:?}", after_resume.json);

    // Delete — terminal, and resume should no longer work.
    let delete = call_as_platform(
        &app,
        "POST",
        &format!("/platform/tenants/{tenant_id}/delete"),
        &platform_cookie,
        json!({}),
    )
    .await;
    assert_eq!(delete.status, StatusCode::OK, "{:?}", delete.json);

    let after_delete = app.call("GET", "/projects", Some(&owner_cookie), json!({})).await;
    assert_eq!(
        after_delete.status,
        StatusCode::FORBIDDEN,
        "a deleted tenant's session should be locked out: {:?}",
        after_delete.json
    );

    let resume_deleted = call_as_platform(
        &app,
        "POST",
        &format!("/platform/tenants/{tenant_id}/resume"),
        &platform_cookie,
        json!({}),
    )
    .await;
    assert_eq!(
        resume_deleted.status,
        StatusCode::BAD_REQUEST,
        "a deleted tenant should not be resumable: {:?}",
        resume_deleted.json
    );
}
