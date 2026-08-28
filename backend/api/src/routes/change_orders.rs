use axum::{
    extract::{Path, State},
    Json,
};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audit,
    auth::session::AuthenticatedUser,
    authz,
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ChangeLineItemInput {
    /// None = a newly added line (scope extension). Some = this line item in
    /// the base quotation is being modified or removed (scope reduction/change).
    pub original_line_item_id: Option<Uuid>,
    #[serde(default)]
    pub removed: bool,
    pub description: String,
    pub quantity: Decimal,
    pub unit: String,
    pub unit_rate: Decimal,
}

#[derive(Deserialize)]
pub struct CreateChangeOrderRequest {
    pub base_quotation_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub line_items: Vec<ChangeLineItemInput>,
    /// Workstream(s) this Change Order requests enabling on the project
    /// alongside (or instead of) BOQ changes — the only way to add a
    /// workstream to an already-existing project, since project_workstream
    /// membership is enforced at the API layer for workstream-specific
    /// entities (see .ai/decisions/current/
    /// 2026-08-28-workstream-enforcement-and-expansion.md). Takes effect
    /// only once the client approves this change order.
    #[serde(default)]
    pub add_workstreams: Vec<entity::workstream_type::WorkstreamType>,
}

#[derive(Serialize)]
pub struct ChangeOrderResponse {
    #[serde(flatten)]
    pub change_order: entity::change_order::Model,
    pub line_items: Vec<entity::change_order_line_item::Model>,
    pub add_workstreams: Vec<entity::change_order_workstream::Model>,
}

/// Proposes a Change Order against a project's currently approved quotation.
/// It is not binding until the client approves it via the Client Portal
/// (.ai/decisions/current/2026-08-27-change-order-requires-client-approval.md)
/// — this endpoint only records the proposal and its computed cost impact.
pub async fn create_change_order(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateChangeOrderRequest>,
) -> Result<Json<ChangeOrderResponse>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    if req.line_items.is_empty() && req.add_workstreams.is_empty() {
        return Err(AppError::BadRequest(
            "at least one line item or workstream addition is required".into(),
        ));
    }
    let tenant_id = user.tenant_id;
    let change_order_id = Uuid::new_v4();
    let title = req.title.clone();
    let description = req.description.clone();
    let base_quotation_id = req.base_quotation_id;
    let add_workstreams = req.add_workstreams.clone();

    let (change_order, line_items, workstreams) = state
        .app_db
        .transaction::<_, (entity::change_order::Model, Vec<entity::change_order_line_item::Model>, Vec<entity::change_order_workstream::Model>), AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("sales_design"),
                )
                .await?;

                let existing_workstreams: std::collections::HashSet<_> =
                    entity::prelude::ProjectWorkstream::find()
                        .filter(entity::project_workstream::Column::ProjectId.eq(project_id))
                        .all(txn)
                        .await?
                        .into_iter()
                        .map(|w| w.workstream_type)
                        .collect();
                for wt in &add_workstreams {
                    if existing_workstreams.contains(wt) {
                        return Err(AppError::BadRequest(format!(
                            "workstream {:?} is already enabled on this project",
                            wt
                        )));
                    }
                }

                let base_quotation = entity::prelude::Quotation::find_by_id(base_quotation_id)
                    .one(txn)
                    .await?
                    .ok_or_else(|| AppError::BadRequest("base_quotation_id not found".into()))?;
                if base_quotation.project_id != project_id {
                    return Err(AppError::BadRequest(
                        "base_quotation_id does not belong to this project".into(),
                    ));
                }
                if base_quotation.status != "approved" {
                    return Err(AppError::BadRequest(
                        "a Change Order can only be raised against an approved quotation".into(),
                    ));
                }

                // First pass: validate every line item and compute the total
                // cost impact, without inserting anything yet — the line
                // items' change_order_id FK needs the parent row to exist
                // first (inserted below).
                let mut cost_impact = Decimal::ZERO;
                for li in &req.line_items {
                    let amount = li.quantity * li.unit_rate;
                    let delta = match li.original_line_item_id {
                        None => amount,
                        Some(orig_id) => {
                            let orig = entity::prelude::QuotationLineItem::find_by_id(orig_id)
                                .one(txn)
                                .await?
                                .ok_or_else(|| {
                                    AppError::BadRequest(
                                        "original_line_item_id not found".into(),
                                    )
                                })?;
                            if orig.quotation_id != base_quotation_id {
                                return Err(AppError::BadRequest(
                                    "original_line_item_id does not belong to base_quotation_id"
                                        .into(),
                                ));
                            }
                            if li.removed {
                                -orig.amount
                            } else {
                                amount - orig.amount
                            }
                        }
                    };
                    cost_impact += delta;
                }

                let co_am = entity::change_order::ActiveModel {
                    id: Set(change_order_id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    base_quotation_id: Set(base_quotation_id),
                    new_quotation_id: Set(None),
                    title: Set(title.clone()),
                    description: Set(description.clone()),
                    status: Set("pending_client_approval".to_string()),
                    cost_impact: Set(cost_impact),
                    requested_by: Set(user.user_id),
                    created_at: Set(chrono::Utc::now().into()),
                    decided_by: Set(None),
                    decided_at: Set(None),
                    decision_notes: Set(None),
                };
                let change_order = co_am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "change_order",
                    change_order_id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({
                        "project_id": project_id,
                        "base_quotation_id": base_quotation_id,
                        "title": title,
                        "cost_impact": cost_impact,
                        "line_item_count": req.line_items.len(),
                        "add_workstreams": add_workstreams,
                    })),
                )
                .await?;

                // Second pass: now that the parent row exists, insert the
                // line items themselves.
                let mut line_items = Vec::with_capacity(req.line_items.len());
                for li in &req.line_items {
                    let amount = li.quantity * li.unit_rate;
                    let id = Uuid::new_v4();
                    let am = entity::change_order_line_item::ActiveModel {
                        id: Set(id),
                        tenant_id: Set(tenant_id),
                        change_order_id: Set(change_order_id),
                        original_line_item_id: Set(li.original_line_item_id),
                        removed: Set(li.removed),
                        description: Set(li.description.clone()),
                        quantity: Set(li.quantity),
                        unit: Set(li.unit.clone()),
                        unit_rate: Set(li.unit_rate),
                        amount: Set(amount),
                        created_at: Set(chrono::Utc::now().into()),
                    };
                    let model = am.insert(txn).await?;
                    line_items.push(model);
                }

                let mut workstreams = Vec::with_capacity(add_workstreams.len());
                for wt in &add_workstreams {
                    let id = Uuid::new_v4();
                    let am = entity::change_order_workstream::ActiveModel {
                        id: Set(id),
                        tenant_id: Set(tenant_id),
                        change_order_id: Set(change_order_id),
                        workstream_type: Set(wt.clone()),
                        created_at: Set(chrono::Utc::now().into()),
                    };
                    let model = am.insert(txn).await?;
                    audit::record(
                        txn,
                        tenant_id,
                        "change_order_workstream",
                        id,
                        "create",
                        audit::Actor::User(user.user_id),
                        None,
                        Some(serde_json::json!({ "change_order_id": change_order_id, "workstream_type": wt })),
                    )
                    .await?;
                    workstreams.push(model);
                }

                Ok((change_order, line_items, workstreams))
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(ChangeOrderResponse {
        change_order,
        line_items,
        add_workstreams: workstreams,
    }))
}

pub async fn list_change_orders(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::change_order::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::change_order::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("sales_design"),
                )
                .await?;
                let items = entity::prelude::ChangeOrder::find()
                    .filter(entity::change_order::Column::ProjectId.eq(project_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

pub async fn get_change_order(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(change_order_id): Path<Uuid>,
) -> Result<Json<ChangeOrderResponse>, AppError> {
    let tenant_id = user.tenant_id;
    let result = state
        .app_db
        .transaction::<_, Option<(entity::change_order::Model, Vec<entity::change_order_line_item::Model>, Vec<entity::change_order_workstream::Model>)>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let Some(change_order) = entity::prelude::ChangeOrder::find_by_id(change_order_id).one(txn).await? else {
                    return Ok(None);
                };
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    change_order.project_id,
                    Some("sales_design"),
                )
                .await?;
                let line_items = entity::prelude::ChangeOrderLineItem::find()
                    .filter(entity::change_order_line_item::Column::ChangeOrderId.eq(change_order_id))
                    .all(txn)
                    .await?;
                let workstreams = entity::prelude::ChangeOrderWorkstream::find()
                    .filter(entity::change_order_workstream::Column::ChangeOrderId.eq(change_order_id))
                    .all(txn)
                    .await?;
                Ok(Some((change_order, line_items, workstreams)))
            })
        })
        .await
        .map_err(map_txn_err)?;

    let (change_order, line_items, workstreams) = result.ok_or(AppError::NotFound)?;
    Ok(Json(ChangeOrderResponse {
        change_order,
        line_items,
        add_workstreams: workstreams,
    }))
}
