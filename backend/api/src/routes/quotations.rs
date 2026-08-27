use axum::{
    extract::{Path, State},
    Json,
};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audit,
    auth::session::AuthenticatedUser,
    db::set_tenant,
    error::{map_txn_err, AppError},
    state::AppState,
};

#[derive(Deserialize)]
pub struct LineItemInput {
    pub description: String,
    pub quantity: Decimal,
    pub unit: String,
    pub unit_rate: Decimal,
}

#[derive(Deserialize)]
pub struct CreateQuotationRequest {
    pub line_items: Vec<LineItemInput>,
}

#[derive(Serialize)]
pub struct QuotationResponse {
    #[serde(flatten)]
    pub quotation: entity::quotation::Model,
    pub line_items: Vec<entity::quotation_line_item::Model>,
}

/// Creates a new versioned Quotation for a project — each call adds the next
/// version number for that project rather than mutating a prior one, so the
/// full quotation history stays intact.
pub async fn create_quotation(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateQuotationRequest>,
) -> Result<Json<QuotationResponse>, AppError> {
    if req.line_items.is_empty() {
        return Err(AppError::BadRequest(
            "at least one line item is required".into(),
        ));
    }
    let tenant_id = user.tenant_id;
    let quotation_id = Uuid::new_v4();

    let (quotation, line_items) = state
        .app_db
        .transaction::<_, (entity::quotation::Model, Vec<entity::quotation_line_item::Model>), AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                if entity::prelude::Project::find_by_id(project_id).one(txn).await?.is_none() {
                    return Err(AppError::NotFound);
                }

                let next_version = entity::prelude::Quotation::find()
                    .filter(entity::quotation::Column::ProjectId.eq(project_id))
                    .order_by_desc(entity::quotation::Column::Version)
                    .one(txn)
                    .await?
                    .map(|q| q.version + 1)
                    .unwrap_or(1);

                let quotation_am = entity::quotation::ActiveModel {
                    id: Set(quotation_id),
                    tenant_id: Set(tenant_id),
                    project_id: Set(project_id),
                    version: Set(next_version),
                    status: Set("draft".to_string()),
                    created_by: Set(user.user_id),
                    created_at: Set(chrono::Utc::now().into()),
                };
                let quotation = quotation_am.insert(txn).await?;
                audit::record(
                    txn,
                    tenant_id,
                    "quotation",
                    quotation_id,
                    "create",
                    audit::Actor::User(user.user_id),
                    None,
                    Some(serde_json::json!({ "project_id": project_id, "version": next_version })),
                )
                .await?;

                let mut line_items = Vec::with_capacity(req.line_items.len());
                for li in &req.line_items {
                    let li_id = Uuid::new_v4();
                    let amount = li.quantity * li.unit_rate;
                    let am = entity::quotation_line_item::ActiveModel {
                        id: Set(li_id),
                        tenant_id: Set(tenant_id),
                        quotation_id: Set(quotation_id),
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
                        "quotation_line_item",
                        li_id,
                        "create",
                        audit::Actor::User(user.user_id),
                        None,
                        Some(serde_json::json!({ "quotation_id": quotation_id, "description": li.description, "amount": amount })),
                    )
                    .await?;
                    line_items.push(model);
                }

                Ok((quotation, line_items))
            })
        })
        .await
        .map_err(map_txn_err)?;

    Ok(Json(QuotationResponse {
        quotation,
        line_items,
    }))
}

pub async fn list_quotations(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::quotation::Model>>, AppError> {
    let tenant_id = user.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::quotation::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let items = entity::prelude::Quotation::find()
                    .filter(entity::quotation::Column::ProjectId.eq(project_id))
                    .order_by_desc(entity::quotation::Column::Version)
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

pub async fn get_quotation(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(quotation_id): Path<Uuid>,
) -> Result<Json<QuotationResponse>, AppError> {
    let tenant_id = user.tenant_id;
    let result = state
        .app_db
        .transaction::<_, Option<(entity::quotation::Model, Vec<entity::quotation_line_item::Model>)>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let Some(quotation) = entity::prelude::Quotation::find_by_id(quotation_id).one(txn).await? else {
                    return Ok(None);
                };
                let line_items = entity::prelude::QuotationLineItem::find()
                    .filter(entity::quotation_line_item::Column::QuotationId.eq(quotation_id))
                    .all(txn)
                    .await?;
                Ok(Some((quotation, line_items)))
            })
        })
        .await
        .map_err(map_txn_err)?;

    let (quotation, line_items) = result.ok_or(AppError::NotFound)?;
    Ok(Json(QuotationResponse {
        quotation,
        line_items,
    }))
}
