use axum::{
    extract::{Path, State},
    Json,
};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    audit,
    auth::session::AuthenticatedUser,
    authz, billing,
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

// ---- Milestones ----

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateMilestoneRequest {
    pub title: String,
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/milestones",
    tag = "billing",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreateMilestoneRequest,
    responses(
        (status = 200, description = "Milestone created", body = entity::milestone::Model),
        (status = 400, description = "bad request", body = crate::error::ErrorResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
    )
)]
pub async fn create_milestone(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateMilestoneRequest>,
) -> Result<Json<entity::milestone::Model>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let title = req.title.clone();

    let model = state
        .app_db
        .transaction::<_, entity::milestone::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(txn, user, project_id, None)
                    .await?;
                let am = entity::milestone::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    title: Set(title.clone()),
                    status: Set("pending".to_string()),
                    created_by: Set(user.user_id),
                    created_at: Set(chrono::Utc::now().into()),
                    completed_at: Set(None),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "milestone",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "project_id": project_id, "title": title })),
                )
                .await?;
                Ok(model)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/milestones",
    tag = "billing",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "List milestones", body = Vec<entity::milestone::Model>),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
    )
)]
pub async fn list_milestones(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::milestone::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::milestone::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(txn, user, project_id, None)
                    .await?;
                let items = entity::prelude::Milestone::find()
                    .filter(entity::milestone::Column::ProjectId.eq(project_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[utoipa::path(
    post,
    path = "/api/milestones/{id}/complete",
    tag = "billing",
    params(("id" = Uuid, Path, description = "Milestone id")),
    responses(
        (status = 200, description = "Milestone marked completed", body = entity::milestone::Model),
        (status = 400, description = "bad request", body = crate::error::ErrorResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn complete_milestone(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(milestone_id): Path<Uuid>,
) -> Result<Json<entity::milestone::Model>, AppError> {
    let tenant_id = user.tenant_id;
    let model = state
        .app_db
        .transaction::<_, entity::milestone::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let m = entity::prelude::Milestone::find_by_id(milestone_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(txn, user, m.project_id, None)
                    .await?;
                if m.status == "completed" {
                    return Err(AppError::BadRequest("milestone is already completed".into()));
                }
                let before = serde_json::json!({ "status": m.status });
                let mut am: entity::milestone::ActiveModel = m.into();
                am.status = Set("completed".to_string());
                am.completed_at = Set(Some(chrono::Utc::now().into()));
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "milestone",
                    milestone_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": "completed" })),
                )
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

// ---- Invoices ----

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateInvoiceRequest {
    /// "milestone" (default) or "progressive" — see .ai/decisions/current/
    /// 2026-08-28-phase-3-audit-and-expansion.md.
    #[serde(default = "default_billing_method")]
    pub billing_method: String,
    /// Required (and only meaningful) for "milestone".
    pub milestone_id: Option<Uuid>,
    /// Required for "milestone" — the taxable amount for this bill.
    /// Ignored for "progressive", which derives its own base_amount from
    /// certified_value_to_date minus the project's prior progressive bills.
    pub base_amount: Option<Decimal>,
    /// Required for "progressive" — the *cumulative* certified value as of
    /// this bill (Finance re-enters the running total each time, matching
    /// how real RA bills restate it), not a per-period delta.
    pub certified_value_to_date: Option<Decimal>,
    pub retention_percent: Decimal,
}

fn default_billing_method() -> String {
    "milestone".to_string()
}

/// Raises an invoice using either of the two implemented billing methods,
/// running the tenant's regional tax profile (India: GST 18% + GST TDS 2%,
/// both on the taxable base_amount) plus the given retention percentage —
/// `billing::calculate_invoice` is shared, method-agnostic math. See
/// api::billing for the calculation and why mobilization-advance recovery
/// isn't included yet.
#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/invoices",
    tag = "billing",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreateInvoiceRequest,
    responses(
        (status = 200, description = "Invoice raised", body = entity::invoice::Model),
        (status = 400, description = "bad request", body = crate::error::ErrorResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn create_invoice(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<Json<entity::invoice::Model>, AppError> {
    if req.retention_percent < Decimal::ZERO || req.retention_percent > Decimal::from(100) {
        return Err(AppError::BadRequest(
            "retention_percent must be between 0 and 100".into(),
        ));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let billing_method = req.billing_method.clone();
    let retention_percent = req.retention_percent;

    let model = state
        .app_db
        .transaction::<_, entity::invoice::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("finance"),
                )
                .await?;

                let (milestone_id, certified_value_to_date, base_amount) = match billing_method
                    .as_str()
                {
                    "milestone" => {
                        let milestone_id = req.milestone_id.ok_or_else(|| {
                            AppError::BadRequest("milestone_id is required for milestone billing".into())
                        })?;
                        let base_amount = req.base_amount.ok_or_else(|| {
                            AppError::BadRequest("base_amount is required for milestone billing".into())
                        })?;
                        if base_amount <= Decimal::ZERO {
                            return Err(AppError::BadRequest("base_amount must be positive".into()));
                        }

                        let milestone = entity::prelude::Milestone::find_by_id(milestone_id)
                            .one(txn)
                            .await?
                            .ok_or_else(|| AppError::BadRequest("milestone_id not found".into()))?;
                        if milestone.project_id != project_id {
                            return Err(AppError::BadRequest(
                                "milestone_id does not belong to this project".into(),
                            ));
                        }
                        if milestone.status != "completed" {
                            return Err(AppError::BadRequest(
                                "an invoice can only be raised against a completed milestone".into(),
                            ));
                        }
                        if entity::prelude::Invoice::find()
                            .filter(entity::invoice::Column::MilestoneId.eq(milestone_id))
                            .one(txn)
                            .await?
                            .is_some()
                        {
                            return Err(AppError::BadRequest(
                                "an invoice has already been raised against this milestone".into(),
                            ));
                        }
                        (Some(milestone_id), None, base_amount)
                    }
                    "progressive" => {
                        let certified = req.certified_value_to_date.ok_or_else(|| {
                            AppError::BadRequest(
                                "certified_value_to_date is required for progressive billing".into(),
                            )
                        })?;
                        let project = entity::prelude::Project::find_by_id(project_id)
                            .one(txn)
                            .await?
                            .ok_or(AppError::NotFound)?;
                        if project.billing_method != "progressive" {
                            return Err(AppError::BadRequest(
                                "project is not configured for progressive billing".into(),
                            ));
                        }
                        let prior_bills = entity::prelude::Invoice::find()
                            .filter(entity::invoice::Column::ProjectId.eq(project_id))
                            .filter(entity::invoice::Column::BillingMethod.eq("progressive"))
                            .all(txn)
                            .await?;
                        let prior = prior_bills
                            .iter()
                            .filter_map(|i| i.certified_value_to_date)
                            .max()
                            .unwrap_or(Decimal::ZERO);
                        if certified <= prior {
                            return Err(AppError::BadRequest(
                                "certified_value_to_date must exceed the previous cumulative value".into(),
                            ));
                        }
                        (None, Some(certified), certified - prior)
                    }
                    _ => {
                        return Err(AppError::BadRequest(
                            "billing_method must be 'milestone' or 'progressive'".into(),
                        ))
                    }
                };

                let tenant = entity::prelude::Tenant::find_by_id(tenant_id)
                    .one(txn)
                    .await?
                    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("tenant not found")))?;
                let profile = billing::profile_for(&tenant.region_profile);
                let calc = billing::calculate_invoice(&*profile, base_amount, retention_percent);

                let am = entity::invoice::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    milestone_id: Set(milestone_id),
                    billing_method: Set(billing_method.clone()),
                    certified_value_to_date: Set(certified_value_to_date),
                    base_amount: Set(base_amount),
                    retention_percent: Set(retention_percent),
                    gst_amount: Set(calc.gst_amount),
                    gst_tds_amount: Set(calc.gst_tds_amount),
                    retention_amount: Set(calc.retention_amount),
                    gross_amount: Set(calc.gross_amount),
                    net_payable: Set(calc.net_payable),
                    status: Set("raised".to_string()),
                    raised_by: Set(user.user_id),
                    created_at: Set(chrono::Utc::now().into()),
                    paid_at: Set(None),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "invoice",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({
                        "project_id": project_id,
                        "billing_method": billing_method,
                        "milestone_id": milestone_id,
                        "certified_value_to_date": certified_value_to_date,
                        "base_amount": base_amount,
                        "gross_amount": calc.gross_amount,
                        "net_payable": calc.net_payable,
                    })),
                )
                .await?;
                Ok(model)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/invoices",
    tag = "billing",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "List invoices", body = Vec<entity::invoice::Model>),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
    )
)]
pub async fn list_invoices(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::invoice::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::invoice::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("finance"),
                )
                .await?;
                let items = entity::prelude::Invoice::find()
                    .filter(entity::invoice::Column::ProjectId.eq(project_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[utoipa::path(
    post,
    path = "/api/invoices/{id}/mark-paid",
    tag = "billing",
    params(("id" = Uuid, Path, description = "Invoice id")),
    responses(
        (status = 200, description = "Invoice marked paid", body = entity::invoice::Model),
        (status = 400, description = "bad request", body = crate::error::ErrorResponse),
        (status = 401, description = "unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn mark_invoice_paid(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(invoice_id): Path<Uuid>,
) -> Result<Json<entity::invoice::Model>, AppError> {
    let tenant_id = user.tenant_id;
    let model = state
        .app_db
        .transaction::<_, entity::invoice::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let invoice = entity::prelude::Invoice::find_by_id(invoice_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    invoice.project_id,
                    Some("finance"),
                )
                .await?;
                if invoice.status == "paid" {
                    return Err(AppError::BadRequest("invoice is already paid".into()));
                }
                let before = serde_json::json!({ "status": invoice.status });
                let mut am: entity::invoice::ActiveModel = invoice.into();
                am.status = Set("paid".to_string());
                am.paid_at = Set(Some(chrono::Utc::now().into()));
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "invoice",
                    invoice_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": "paid" })),
                )
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}
