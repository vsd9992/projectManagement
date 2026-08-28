mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;

#[tokio::test]
async fn site_task_creation_links_schedule_task_and_syncs_status() {
    let (app, _owner, cookie, project_id) = setup_project("schedlink").await;

    let task = app
        .call(
            "POST",
            &format!("/projects/{project_id}/site-tasks"),
            Some(&cookie),
            json!({ "title": "Electrical rough-in" }),
        )
        .await;
    assert_eq!(task.status, StatusCode::OK, "{:?}", task.json);
    let site_task_id = task.json["id"].as_str().unwrap().to_string();
    let schedule_task_id = task.json["schedule_task_id"].as_str().unwrap().to_string();
    assert_ne!(site_task_id, schedule_task_id);

    let sched_list = app
        .call("GET", &format!("/projects/{project_id}/schedule-tasks"), Some(&cookie), json!({}))
        .await;
    assert_eq!(sched_list.status, StatusCode::OK);
    let matching = sched_list
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"].as_str().unwrap() == schedule_task_id)
        .expect("linked schedule_task should be listed");
    assert_eq!(matching["site_task_id"].as_str().unwrap(), site_task_id);
    assert_eq!(matching["status"], "not_started");

    // Status sync: updating the site task's status also updates the linked
    // schedule_task's status.
    let updated = app
        .call(
            "POST",
            &format!("/site-tasks/{site_task_id}/status"),
            Some(&cookie),
            json!({ "status": "done" }),
        )
        .await;
    assert_eq!(updated.status, StatusCode::OK, "{:?}", updated.json);

    let sched_list2 = app
        .call("GET", &format!("/projects/{project_id}/schedule-tasks"), Some(&cookie), json!({}))
        .await;
    let matching2 = sched_list2
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"].as_str().unwrap() == schedule_task_id)
        .unwrap();
    assert_eq!(matching2["status"], "done");
}

#[tokio::test]
async fn standalone_schedule_task_create_and_status_lifecycle() {
    let (app, _owner, cookie, project_id) = setup_project("schedstandalone").await;

    let task = app
        .call(
            "POST",
            &format!("/projects/{project_id}/schedule-tasks"),
            Some(&cookie),
            json!({ "title": "Procure steel", "workstream_type": "procurement" }),
        )
        .await;
    assert_eq!(task.status, StatusCode::OK, "{:?}", task.json);
    assert_eq!(task.json["status"], "not_started");
    assert_eq!(task.json["workstream_type"], "procurement");
    let task_id = task.json["id"].as_str().unwrap().to_string();

    let bad_status = app
        .call(
            "POST",
            &format!("/schedule-tasks/{task_id}/status"),
            Some(&cookie),
            json!({ "status": "bogus" }),
        )
        .await;
    assert_eq!(bad_status.status, StatusCode::BAD_REQUEST, "{:?}", bad_status.json);

    let in_progress = app
        .call(
            "POST",
            &format!("/schedule-tasks/{task_id}/status"),
            Some(&cookie),
            json!({ "status": "in_progress" }),
        )
        .await;
    assert_eq!(in_progress.status, StatusCode::OK, "{:?}", in_progress.json);
    assert_eq!(in_progress.json["status"], "in_progress");
}

#[tokio::test]
async fn schedule_task_dates_full_replace_and_check_constraint() {
    let (app, _owner, cookie, project_id) = setup_project("scheddates").await;
    let task = app
        .call(
            "POST",
            &format!("/projects/{project_id}/schedule-tasks"),
            Some(&cookie),
            json!({ "title": "Design draft", "workstream_type": "design" }),
        )
        .await;
    let task_id = task.json["id"].as_str().unwrap().to_string();

    let dates = app
        .call(
            "POST",
            &format!("/schedule-tasks/{task_id}/dates"),
            Some(&cookie),
            json!({ "planned_start_date": "2026-09-01", "planned_end_date": "2026-09-10" }),
        )
        .await;
    assert_eq!(dates.status, StatusCode::OK, "{:?}", dates.json);
    assert_eq!(dates.json["planned_start_date"], "2026-09-01");
    assert_eq!(dates.json["planned_end_date"], "2026-09-10");
    assert_eq!(dates.json["actual_start_date"], serde_json::Value::Null);
}

#[tokio::test]
async fn schedule_task_dependency_self_and_cross_project_rejected() {
    let (app, _owner, cookie, project_id) = setup_project("scheddep").await;
    let t1 = app
        .call(
            "POST",
            &format!("/projects/{project_id}/schedule-tasks"),
            Some(&cookie),
            json!({ "title": "A", "workstream_type": "manufacturing" }),
        )
        .await;
    let t1_id = t1.json["id"].as_str().unwrap().to_string();

    let self_dep = app
        .call(
            "POST",
            &format!("/schedule-tasks/{t1_id}/dependencies"),
            Some(&cookie),
            json!({ "depends_on_task_id": t1_id }),
        )
        .await;
    assert_eq!(self_dep.status, StatusCode::BAD_REQUEST, "{:?}", self_dep.json);

    // A task in a different project.
    let bu2 = create_business_unit(&app, &_owner, "Other BU").await;
    let client2 = create_client(&app, &_owner, "Other Client").await;
    let other_project = create_project(&app, &_owner, bu2, client2, "Other Project").await;
    let project2_id = other_project.json["id"].as_str().unwrap().to_string();
    let t2 = app
        .call(
            "POST",
            &format!("/projects/{project2_id}/schedule-tasks"),
            Some(&_owner),
            json!({ "title": "B", "workstream_type": "manufacturing" }),
        )
        .await;
    assert_eq!(t2.status, StatusCode::OK, "{:?}", t2.json);
    let t2_id = t2.json["id"].as_str().unwrap().to_string();

    let cross = app
        .call(
            "POST",
            &format!("/schedule-tasks/{t1_id}/dependencies"),
            Some(&cookie),
            json!({ "depends_on_task_id": t2_id }),
        )
        .await;
    assert_eq!(cross.status, StatusCode::BAD_REQUEST, "{:?}", cross.json);
}

#[tokio::test]
async fn schedule_task_dependency_cycle_rejected() {
    let (app, _owner, cookie, project_id) = setup_project("schedcycle").await;

    let mut ids = Vec::new();
    for name in ["A", "B", "C"] {
        let t = app
            .call(
                "POST",
                &format!("/projects/{project_id}/schedule-tasks"),
                Some(&cookie),
                json!({ "title": name, "workstream_type": "site_execution" }),
            )
            .await;
        assert_eq!(t.status, StatusCode::OK, "{:?}", t.json);
        ids.push(t.json["id"].as_str().unwrap().to_string());
    }
    let (a, b, c) = (ids[0].clone(), ids[1].clone(), ids[2].clone());

    // A depends on B, B depends on C (a chain).
    let ab = app
        .call(
            "POST",
            &format!("/schedule-tasks/{a}/dependencies"),
            Some(&cookie),
            json!({ "depends_on_task_id": b }),
        )
        .await;
    assert_eq!(ab.status, StatusCode::OK, "{:?}", ab.json);
    let bc = app
        .call(
            "POST",
            &format!("/schedule-tasks/{b}/dependencies"),
            Some(&cookie),
            json!({ "depends_on_task_id": c }),
        )
        .await;
    assert_eq!(bc.status, StatusCode::OK, "{:?}", bc.json);

    // C depending on A would close the cycle A -> B -> C -> A.
    let cycle = app
        .call(
            "POST",
            &format!("/schedule-tasks/{c}/dependencies"),
            Some(&cookie),
            json!({ "depends_on_task_id": a }),
        )
        .await;
    assert_eq!(
        cycle.status,
        StatusCode::BAD_REQUEST,
        "C depending on A should be rejected as a cycle: {:?}",
        cycle.json
    );

    // C depending on a *new*, unrelated task D is not a cycle.
    let d = app
        .call(
            "POST",
            &format!("/projects/{project_id}/schedule-tasks"),
            Some(&cookie),
            json!({ "title": "D", "workstream_type": "site_execution" }),
        )
        .await;
    let d_id = d.json["id"].as_str().unwrap().to_string();
    let cd = app
        .call(
            "POST",
            &format!("/schedule-tasks/{c}/dependencies"),
            Some(&cookie),
            json!({ "depends_on_task_id": d_id }),
        )
        .await;
    assert_eq!(cd.status, StatusCode::OK, "{:?}", cd.json);

    let listed = app
        .call("GET", &format!("/schedule-tasks/{a}/dependencies"), Some(&cookie), json!({}))
        .await;
    assert_eq!(listed.status, StatusCode::OK);
    assert_eq!(listed.json.as_array().unwrap().len(), 1);
}

/// The key "conditional forward-pass, not blind cascade" assertion: A's
/// actual completion slips 5 days late. B (tightly coupled, no slack)
/// shifts by the same 5 days. C (depends on B, but with plenty of slack
/// relative to B's new end date) does NOT shift, and the cascade stops
/// there rather than propagating further.
#[tokio::test]
async fn schedule_task_date_shift_propagates_conditionally_to_dependents() {
    let (app, _owner, cookie, project_id) = setup_project("schedshift").await;

    async fn make_task(app: &TestApp, cookie: &str, project_id: &str, title: &str) -> String {
        let t = app
            .call(
                "POST",
                &format!("/projects/{project_id}/schedule-tasks"),
                Some(cookie),
                json!({ "title": title, "workstream_type": "site_execution" }),
            )
            .await;
        assert_eq!(t.status, StatusCode::OK, "{:?}", t.json);
        t.json["id"].as_str().unwrap().to_string()
    }

    let a = make_task(&app, &cookie, &project_id, "A").await;
    let b = make_task(&app, &cookie, &project_id, "B").await;
    let c = make_task(&app, &cookie, &project_id, "C").await;

    // B depends on A, C depends on B.
    let dep1 = app
        .call(
            "POST",
            &format!("/schedule-tasks/{b}/dependencies"),
            Some(&cookie),
            json!({ "depends_on_task_id": a }),
        )
        .await;
    assert_eq!(dep1.status, StatusCode::OK, "{:?}", dep1.json);
    let dep2 = app
        .call(
            "POST",
            &format!("/schedule-tasks/{c}/dependencies"),
            Some(&cookie),
            json!({ "depends_on_task_id": b }),
        )
        .await;
    assert_eq!(dep2.status, StatusCode::OK, "{:?}", dep2.json);

    // Initial plans: A ends right before B starts (no slack); C has 10
    // days of slack after B's planned end.
    let set_a = app
        .call(
            "POST",
            &format!("/schedule-tasks/{a}/dates"),
            Some(&cookie),
            json!({ "planned_start_date": "2026-09-01", "planned_end_date": "2026-09-05" }),
        )
        .await;
    assert_eq!(set_a.status, StatusCode::OK, "{:?}", set_a.json);
    assert!(set_a.json["shifted_dependent_task_ids"].as_array().unwrap().is_empty());

    let set_b = app
        .call(
            "POST",
            &format!("/schedule-tasks/{b}/dates"),
            Some(&cookie),
            json!({ "planned_start_date": "2026-09-06", "planned_end_date": "2026-09-10" }),
        )
        .await;
    assert_eq!(set_b.status, StatusCode::OK, "{:?}", set_b.json);

    let set_c = app
        .call(
            "POST",
            &format!("/schedule-tasks/{c}/dates"),
            Some(&cookie),
            json!({ "planned_start_date": "2026-09-20", "planned_end_date": "2026-09-25" }),
        )
        .await;
    assert_eq!(set_c.status, StatusCode::OK, "{:?}", set_c.json);

    // A actually finishes 5 days late (2026-09-10 instead of 2026-09-05).
    let slip = app
        .call(
            "POST",
            &format!("/schedule-tasks/{a}/dates"),
            Some(&cookie),
            json!({
                "planned_start_date": "2026-09-01",
                "planned_end_date": "2026-09-05",
                "actual_start_date": "2026-09-01",
                "actual_end_date": "2026-09-10",
            }),
        )
        .await;
    assert_eq!(slip.status, StatusCode::OK, "{:?}", slip.json);
    let shifted: Vec<&str> = slip.json["shifted_dependent_task_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(shifted, vec![b.as_str()], "only B should shift, not C: {:?}", slip.json);

    let b_after = app
        .call("GET", &format!("/projects/{project_id}/schedule-tasks"), Some(&cookie), json!({}))
        .await;
    let b_task = b_after
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"].as_str().unwrap() == b)
        .unwrap();
    assert_eq!(b_task["planned_start_date"], "2026-09-11", "B should shift +5 days: {:?}", b_task);
    assert_eq!(b_task["planned_end_date"], "2026-09-15");

    let c_task = b_after
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"].as_str().unwrap() == c)
        .unwrap();
    assert_eq!(c_task["planned_start_date"], "2026-09-20", "C has slack and should not shift: {:?}", c_task);
    assert_eq!(c_task["planned_end_date"], "2026-09-25");
}

#[tokio::test]
async fn schedule_task_shift_creates_notifications_for_bu_team_and_tenant_admin() {
    let app = spawn_app().await;
    let (owner_cookie, _tenant_id, _owner_id) = signup(&app, "notiftest-owner").await;
    let bu1 = create_business_unit(&app, &owner_cookie, "BU1").await;
    let bu2 = create_business_unit(&app, &owner_cookie, "BU2").await;
    let client_id = create_client(&app, &owner_cookie, "Acme").await;
    let (teammate1_cookie, teammate1_id) =
        create_and_login_teammate(&app, &owner_cookie, "notiftest-t1").await;
    let (_teammate2_cookie, teammate2_id) =
        create_and_login_teammate(&app, &owner_cookie, "notiftest-t2").await;
    assign_role(&app, &owner_cookie, bu1, teammate1_id, "delivery").await;
    assign_role(&app, &owner_cookie, bu2, teammate2_id, "delivery").await;

    let project = create_project_with_workstreams(
        &app,
        &teammate1_cookie,
        bu1,
        client_id,
        "Notif Test Project",
        &["site_execution"],
    )
    .await;
    assert_eq!(project.status, StatusCode::OK, "{:?}", project.json);
    let project_id = project.json["id"].as_str().unwrap().to_string();

    let a = app
        .call(
            "POST",
            &format!("/projects/{project_id}/schedule-tasks"),
            Some(&teammate1_cookie),
            json!({ "title": "A", "workstream_type": "site_execution" }),
        )
        .await
        .json["id"]
        .as_str()
        .unwrap()
        .to_string();
    let b = app
        .call(
            "POST",
            &format!("/projects/{project_id}/schedule-tasks"),
            Some(&teammate1_cookie),
            json!({ "title": "B", "workstream_type": "site_execution" }),
        )
        .await
        .json["id"]
        .as_str()
        .unwrap()
        .to_string();
    app.call(
        "POST",
        &format!("/schedule-tasks/{b}/dependencies"),
        Some(&teammate1_cookie),
        json!({ "depends_on_task_id": a }),
    )
    .await;
    app.call(
        "POST",
        &format!("/schedule-tasks/{a}/dates"),
        Some(&teammate1_cookie),
        json!({ "planned_start_date": "2026-11-01", "planned_end_date": "2026-11-05" }),
    )
    .await;
    app.call(
        "POST",
        &format!("/schedule-tasks/{b}/dates"),
        Some(&teammate1_cookie),
        json!({ "planned_start_date": "2026-11-06", "planned_end_date": "2026-11-10" }),
    )
    .await;

    let slip = app
        .call(
            "POST",
            &format!("/schedule-tasks/{a}/dates"),
            Some(&teammate1_cookie),
            json!({
                "planned_start_date": "2026-11-01", "planned_end_date": "2026-11-05",
                "actual_start_date": "2026-11-01", "actual_end_date": "2026-11-08",
            }),
        )
        .await;
    assert_eq!(slip.status, StatusCode::OK, "{:?}", slip.json);
    assert!(!slip.json["shifted_dependent_task_ids"].as_array().unwrap().is_empty());

    // BU1 teammate (in the project's business unit) is notified.
    let t1_notifs = app.call("GET", "/notifications", Some(&teammate1_cookie), json!({})).await;
    assert_eq!(t1_notifs.status, StatusCode::OK);
    let t1_list = t1_notifs.json.as_array().unwrap();
    assert_eq!(t1_list.len(), 1, "{:?}", t1_list);
    assert_eq!(t1_list[0]["schedule_task_id"].as_str().unwrap(), b);
    assert_eq!(t1_list[0]["is_read"], false);

    // Tenant admin (owner) is notified too.
    let owner_notifs = app.call("GET", "/notifications", Some(&owner_cookie), json!({})).await;
    assert_eq!(owner_notifs.json.as_array().unwrap().len(), 1);

    // BU2 teammate (no role on this project's BU) is not notified.
    let t2_notifs = app.call("GET", "/notifications", Some(&_teammate2_cookie), json!({})).await;
    assert_eq!(t2_notifs.json.as_array().unwrap().len(), 0);

    // Mark read + unread_only filtering.
    let notif_id = t1_list[0]["id"].as_str().unwrap().to_string();
    let marked = app
        .call("POST", &format!("/notifications/{notif_id}/read"), Some(&teammate1_cookie), json!({}))
        .await;
    assert_eq!(marked.status, StatusCode::OK, "{:?}", marked.json);
    assert_eq!(marked.json["is_read"], true);

    let unread = app
        .call("GET", "/notifications?unread_only=true", Some(&teammate1_cookie), json!({}))
        .await;
    assert_eq!(unread.json.as_array().unwrap().len(), 0, "{:?}", unread.json);
}

#[tokio::test]
async fn already_started_schedule_task_shift_does_not_notify() {
    let (app, _owner, cookie, project_id) = setup_project("notifstarted").await;

    let a = app
        .call(
            "POST",
            &format!("/projects/{project_id}/schedule-tasks"),
            Some(&cookie),
            json!({ "title": "A", "workstream_type": "site_execution" }),
        )
        .await
        .json["id"]
        .as_str()
        .unwrap()
        .to_string();
    let b = app
        .call(
            "POST",
            &format!("/projects/{project_id}/schedule-tasks"),
            Some(&cookie),
            json!({ "title": "B", "workstream_type": "site_execution" }),
        )
        .await
        .json["id"]
        .as_str()
        .unwrap()
        .to_string();
    app.call(
        "POST",
        &format!("/schedule-tasks/{b}/dependencies"),
        Some(&cookie),
        json!({ "depends_on_task_id": a }),
    )
    .await;
    app.call(
        "POST",
        &format!("/schedule-tasks/{a}/dates"),
        Some(&cookie),
        json!({ "planned_start_date": "2026-12-01", "planned_end_date": "2026-12-05" }),
    )
    .await;
    // B has already started (actual_start_date set) — it should still
    // shift (the algorithm doesn't skip shifting), but not notify.
    app.call(
        "POST",
        &format!("/schedule-tasks/{b}/dates"),
        Some(&cookie),
        json!({
            "planned_start_date": "2026-12-06", "planned_end_date": "2026-12-10",
            "actual_start_date": "2026-12-06",
        }),
    )
    .await;

    let slip = app
        .call(
            "POST",
            &format!("/schedule-tasks/{a}/dates"),
            Some(&cookie),
            json!({
                "planned_start_date": "2026-12-01", "planned_end_date": "2026-12-05",
                "actual_start_date": "2026-12-01", "actual_end_date": "2026-12-08",
            }),
        )
        .await;
    assert_eq!(slip.status, StatusCode::OK, "{:?}", slip.json);
    assert_eq!(
        slip.json["shifted_dependent_task_ids"].as_array().unwrap().len(),
        1,
        "B should still shift even though already started: {:?}",
        slip.json
    );

    let notifs = app.call("GET", "/notifications", Some(&cookie), json!({})).await;
    assert_eq!(
        notifs.json.as_array().unwrap().len(),
        0,
        "an already-started task's shift should not notify: {:?}",
        notifs.json
    );
}
