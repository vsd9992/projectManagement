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
    billing,
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

// ---- Milestones ----

#[derive(Deserialize)]
pub struct CreateMilestoneRequest {
    pub title: String,
}

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

#[derive(Deserialize)]
pub struct CreateInvoiceRequest {
    pub milestone_id: Uuid,
    pub base_amount: Decimal,
    pub retention_percent: Decimal,
}

/// Raises a milestone-based invoice against a *completed* milestone, running
/// the tenant's regional tax profile (India: GST 18% + GST TDS 2%, both on
/// the taxable base_amount) plus the given retention percentage. See
/// api::billing for the calculation and why mobilization-advance recovery
/// isn't included yet.
pub async fn create_invoice(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<Json<entity::invoice::Model>, AppError> {
    if req.base_amount <= Decimal::ZERO {
        return Err(AppError::BadRequest("base_amount must be positive".into()));
    }
    if req.retention_percent < Decimal::ZERO || req.retention_percent > Decimal::from(100) {
        return Err(AppError::BadRequest(
            "retention_percent must be between 0 and 100".into(),
        ));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let milestone_id = req.milestone_id;
    let base_amount = req.base_amount;
    let retention_percent = req.retention_percent;

    let model = state
        .app_db
        .transaction::<_, entity::invoice::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

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
                        "milestone_id": milestone_id,
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
