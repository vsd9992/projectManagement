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
    error::{map_txn_err, AppError, ErrorResponse},
    state::AppState,
};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateVendorRequest {
    pub name: String,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/vendors",
    tag = "procurement",
    request_body = CreateVendorRequest,
    responses(
        (status = 200, description = "Vendor created", body = VendorModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_vendor(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateVendorRequest>,
) -> Result<Json<VendorModel>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let name = req.name.clone();

    let model = state
        .app_db
        .transaction::<_, VendorModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_any_business_unit_role(txn, user, Some("delivery")).await?;
                let am = entity::vendor::ActiveModel {
                    id: Set(id),
                    tenant_id: Set(tenant_id),
                    name: Set(name.clone()),
                    contact_email: Set(req.contact_email.clone()),
                    contact_phone: Set(req.contact_phone.clone()),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let model = am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "vendor",
                    id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "name": name })),
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
    path = "/api/vendors",
    tag = "procurement",
    responses(
        (status = 200, description = "List vendors", body = Vec<VendorModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn list_vendors(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<VendorModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<VendorModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_any_business_unit_role(txn, user, None).await?;
                Ok(entity::prelude::Vendor::find().all(txn).await?)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct PoLineItemInput {
    pub quotation_line_item_id: Option<Uuid>,
    pub description: String,
    pub quantity: Decimal,
    pub unit: String,
    pub unit_rate: Decimal,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreatePurchaseOrderRequest {
    pub vendor_id: Uuid,
    pub title: String,
    pub line_items: Vec<PoLineItemInput>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PurchaseOrderResponse {
    #[serde(flatten)]
    pub purchase_order: PurchaseOrderModel,
    pub line_items: Vec<PurchaseOrderLineItemModel>,
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/purchase-orders",
    tag = "procurement",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreatePurchaseOrderRequest,
    responses(
        (status = 200, description = "Purchase order created (pending_approval)", body = PurchaseOrderResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn create_purchase_order(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreatePurchaseOrderRequest>,
) -> Result<Json<PurchaseOrderResponse>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    if req.line_items.is_empty() {
        return Err(AppError::BadRequest(
            "at least one line item is required".into(),
        ));
    }
    let tenant_id = user.tenant_id;
    let po_id = Uuid::new_v4();
    let title = req.title.clone();
    let vendor_id = req.vendor_id;

    let (purchase_order, line_items) = state
        .app_db
        .transaction::<_, (PurchaseOrderModel, Vec<PurchaseOrderLineItemModel>), AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                authz::require_project_workstream(
                    txn,
                    project_id,
                    entity::workstream_type::WorkstreamType::Procurement,
                )
                .await?;

                if entity::prelude::Vendor::find_by_id(vendor_id).one(txn).await?.is_none() {
                    return Err(AppError::BadRequest("vendor_id not found".into()));
                }

                let po_am = entity::purchase_order::ActiveModel {
                    id: Set(po_id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    vendor_id: Set(vendor_id),
                    title: Set(title.clone()),
                    status: Set("pending_approval".to_string()),
                    created_by: Set(user.user_id),
                    created_at: Set(chrono::Utc::now().into()),
                    delivered_at: Set(None),
                    decided_by: Set(None),
                    decided_at: Set(None),
                    decision_notes: Set(None),
                };
                let purchase_order = po_am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "purchase_order",
                    po_id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "project_id": project_id, "vendor_id": vendor_id, "title": title })),
                )
                .await?;

                let mut line_items = Vec::with_capacity(req.line_items.len());
                for li in &req.line_items {
                    let id = Uuid::new_v4();
                    let amount = li.quantity * li.unit_rate;
                    let am = entity::purchase_order_line_item::ActiveModel {
                        id: Set(id),
                        tenant_id: Set(tenant_id),
                        purchase_order_id: Set(po_id),
                        quotation_line_item_id: Set(li.quotation_line_item_id),
                        description: Set(li.description.clone()),
                        quantity: Set(li.quantity),
                        unit: Set(li.unit.clone()),
                        unit_rate: Set(li.unit_rate),
                        amount: Set(amount),
                        created_at: Set(chrono::Utc::now().into()),
                    };
                    let model = am.insert(txn).await?;
                    audit::record(
                        txn,
                        tenant_id,
                        "purchase_order_line_item",
                        id,
                        "create",
                        audit::Actor::User(user.user_id),
                        None,
                        Some(serde_json::json!({ "purchase_order_id": po_id, "description": li.description, "amount": amount })),
                    )
                    .await?;
                    line_items.push(model);
                }

                Ok((purchase_order, line_items))
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(PurchaseOrderResponse {
        purchase_order,
        line_items,
    }))
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/purchase-orders",
    tag = "procurement",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses(
        (status = 200, description = "List purchase orders", body = Vec<PurchaseOrderModel>),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
    )
)]
pub async fn list_purchase_orders(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<PurchaseOrderModel>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<PurchaseOrderModel>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("delivery"),
                )
                .await?;
                let items = entity::prelude::PurchaseOrder::find()
                    .filter(entity::purchase_order::Column::ProjectId.eq(project_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct PoDecisionRequest {
    pub notes: Option<String>,
}

async fn decide_purchase_order(
    state: &AppState,
    user: AuthenticatedUser,
    po_id: Uuid,
    approve: bool,
    notes: Option<String>,
) -> Result<PurchaseOrderModel, AppError> {
    let tenant_id = user.tenant_id;
    state
        .app_db
        .transaction::<_, PurchaseOrderModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let po = entity::prelude::PurchaseOrder::find_by_id(po_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    po.project_id,
                    Some("delivery"),
                )
                .await?;
                if po.status != "pending_approval" {
                    return Err(AppError::BadRequest(format!(
                        "purchase order is already {}",
                        po.status
                    )));
                }
                let before = serde_json::json!({ "status": po.status });
                let new_status = if approve { "open" } else { "rejected" };
                let mut am: entity::purchase_order::ActiveModel = po.into();
                am.status = Set(new_status.to_string());
                am.decided_by = Set(Some(user.user_id));
                am.decided_at = Set(Some(chrono::Utc::now().into()));
                am.decision_notes = Set(notes.clone());
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "purchase_order",
                    po_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": new_status, "notes": notes })),
                )
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)
}

/// Internal approval step before a PO is sent to a vendor — closes the
/// gap `architecture.md` listing POs in the approval chain even though no
/// approval step existed. No distinct "approver" role exists in the
/// current catalog, so this is gated by the same `delivery` role that
/// creates a PO — matching how this app doesn't otherwise separate
/// creator/approver roles anywhere internal.
#[utoipa::path(
    post,
    path = "/api/purchase-orders/{id}/approve",
    tag = "procurement",
    params(("id" = Uuid, Path, description = "Purchase order id")),
    request_body = PoDecisionRequest,
    responses(
        (status = 200, description = "Purchase order approved (now open)", body = PurchaseOrderModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn approve_purchase_order(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(po_id): Path<Uuid>,
    Json(req): Json<PoDecisionRequest>,
) -> Result<Json<PurchaseOrderModel>, AppError> {
    let model = decide_purchase_order(&state, user, po_id, true, req.notes).await?;
    Ok(Json(model))
}

#[utoipa::path(
    post,
    path = "/api/purchase-orders/{id}/reject",
    tag = "procurement",
    params(("id" = Uuid, Path, description = "Purchase order id")),
    request_body = PoDecisionRequest,
    responses(
        (status = 200, description = "Purchase order rejected (terminal)", body = PurchaseOrderModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn reject_purchase_order(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(po_id): Path<Uuid>,
    Json(req): Json<PoDecisionRequest>,
) -> Result<Json<PurchaseOrderModel>, AppError> {
    let model = decide_purchase_order(&state, user, po_id, false, req.notes).await?;
    Ok(Json(model))
}

#[utoipa::path(
    post,
    path = "/api/purchase-orders/{id}/deliver",
    tag = "procurement",
    params(("id" = Uuid, Path, description = "Purchase order id")),
    responses(
        (status = 200, description = "Purchase order marked delivered", body = PurchaseOrderModel),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unauthorized", body = ErrorResponse),
        (status = 403, description = "forbidden", body = ErrorResponse),
        (status = 404, description = "not found", body = ErrorResponse),
    )
)]
pub async fn mark_purchase_order_delivered(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(po_id): Path<Uuid>,
) -> Result<Json<PurchaseOrderModel>, AppError> {
    let tenant_id = user.tenant_id;
    let model = state
        .app_db
        .transaction::<_, PurchaseOrderModel, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let po = entity::prelude::PurchaseOrder::find_by_id(po_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    po.project_id,
                    Some("delivery"),
                )
                .await?;
                if po.status != "open" {
                    return Err(AppError::BadRequest(format!(
                        "purchase order is already {}",
                        po.status
                    )));
                }
                let before = serde_json::json!({ "status": po.status });
                let mut am: entity::purchase_order::ActiveModel = po.into();
                am.status = Set("delivered".to_string());
                am.delivered_at = Set(Some(chrono::Utc::now().into()));
                let updated = am.update(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "purchase_order",
                    po_id,
                    "update",
                    audit::Actor::User(user.user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": "delivered" })),
                )
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(model))
}

use entity::purchase_order::Model as PurchaseOrderModel;
use entity::purchase_order_line_item::Model as PurchaseOrderLineItemModel;
use entity::vendor::Model as VendorModel;
