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
pub struct CreateVendorRequest {
    pub name: String,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
}

pub async fn create_vendor(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateVendorRequest>,
) -> Result<Json<entity::vendor::Model>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let tenant_id = user.tenant_id;
    let id = Uuid::new_v4();
    let name = req.name.clone();

    let model = state
        .app_db
        .transaction::<_, entity::vendor::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
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

pub async fn list_vendors(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<entity::vendor::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::vendor::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                Ok(entity::prelude::Vendor::find().all(txn).await?)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[derive(Deserialize)]
pub struct PoLineItemInput {
    pub quotation_line_item_id: Option<Uuid>,
    pub description: String,
    pub quantity: Decimal,
    pub unit: String,
    pub unit_rate: Decimal,
}

#[derive(Deserialize)]
pub struct CreatePurchaseOrderRequest {
    pub vendor_id: Uuid,
    pub title: String,
    pub line_items: Vec<PoLineItemInput>,
}

#[derive(Serialize)]
pub struct PurchaseOrderResponse {
    #[serde(flatten)]
    pub purchase_order: entity::purchase_order::Model,
    pub line_items: Vec<entity::purchase_order_line_item::Model>,
}

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
        .transaction::<_, (entity::purchase_order::Model, Vec<entity::purchase_order_line_item::Model>), AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                authz::require_project_business_unit_role(
                    txn,
                    user,
                    project_id,
                    Some("delivery"),
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
                    status: Set("open".to_string()),
                    created_by: Set(user.user_id),
                    created_at: Set(chrono::Utc::now().into()),
                    delivered_at: Set(None),
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

pub async fn list_purchase_orders(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::purchase_order::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::purchase_order::Model>, AppError>(|txn| {
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

pub async fn mark_purchase_order_delivered(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(po_id): Path<Uuid>,
) -> Result<Json<entity::purchase_order::Model>, AppError> {
    let tenant_id = user.tenant_id;
    let model = state
        .app_db
        .transaction::<_, entity::purchase_order::Model, AppError>(|txn| {
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
