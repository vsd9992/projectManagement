use axum::{
    extract::{Path, State},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::Deserialize;
use std::collections::HashSet;
use uuid::Uuid;

use crate::{
    audit,
    auth::session::AuthenticatedClientUser,
    db::set_tenant,
    error::{map_txn_err, AppError},
    routes::design::DesignAssetWithRevisions,
    state::AppState,
};

/// Lists only the projects belonging to this client's own `client_id` — RLS
/// alone would return every project in the tenant, so this filter is what
/// actually keeps a client scoped to their own data (see
/// .ai/decisions/current/2026-08-27-auth-session-based-single-login.md).
pub async fn list_my_projects(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
) -> Result<Json<Vec<entity::project::Model>>, AppError> {
    let tenant_id = client.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::project::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let items = entity::prelude::Project::find()
                    .filter(entity::project::Column::ClientId.eq(client.client_id))
                    .all(txn)
                    .await?;
                Ok(items)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

pub async fn list_project_design_assets(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<DesignAssetWithRevisions>>, AppError> {
    let tenant_id = client.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<DesignAssetWithRevisions>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let project = entity::prelude::Project::find_by_id(project_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if project.client_id != client.client_id {
                    return Err(AppError::NotFound);
                }

                let assets = entity::prelude::DesignAsset::find()
                    .filter(entity::design_asset::Column::ProjectId.eq(project_id))
                    .all(txn)
                    .await?;

                let mut result = Vec::with_capacity(assets.len());
                for asset in assets {
                    let revisions = entity::prelude::DesignRevision::find()
                        .filter(entity::design_revision::Column::DesignAssetId.eq(asset.id))
                        .all(txn)
                        .await?;
                    result.push(DesignAssetWithRevisions { asset, revisions });
                }
                Ok(result)
            })
        })
        .await
        .map_err(map_txn_err)?;
    Ok(Json(items))
}

#[derive(Deserialize)]
pub struct DecisionRequest {
    pub notes: Option<String>,
}

async fn decide_revision(
    state: &AppState,
    client: AuthenticatedClientUser,
    revision_id: Uuid,
    approve: bool,
    notes: Option<String>,
) -> Result<entity::design_revision::Model, AppError> {
    let tenant_id = client.tenant_id;

    state
        .app_db
        .transaction::<_, entity::design_revision::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let revision = entity::prelude::DesignRevision::find_by_id(revision_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let asset = entity::prelude::DesignAsset::find_by_id(revision.design_asset_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let project = entity::prelude::Project::find_by_id(asset.project_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if project.client_id != client.client_id {
                    return Err(AppError::NotFound);
                }
                if revision.status != "submitted" {
                    return Err(AppError::BadRequest(format!(
                        "revision is already {}",
                        revision.status
                    )));
                }

                let before = serde_json::json!({ "status": revision.status });
                let new_status = if approve { "approved" } else { "rejected" };

                let mut am: entity::design_revision::ActiveModel = revision.into();
                am.status = Set(new_status.to_string());
                am.decided_by = Set(Some(client.client_user_id));
                am.decided_at = Set(Some(chrono::Utc::now().into()));
                am.decision_notes = Set(notes.clone());
                let updated = am.update(txn).await?;

                audit::record(
                    txn,
                    tenant_id,
                    "design_revision",
                    revision_id,
                    "update",
                    audit::Actor::ClientUser(client.client_user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": new_status, "decided_by_client_user": client.client_user_id, "notes": notes })),
                )
                .await?;

                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)
}

pub async fn approve_design_revision(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(revision_id): Path<Uuid>,
    Json(req): Json<DecisionRequest>,
) -> Result<Json<entity::design_revision::Model>, AppError> {
    let model = decide_revision(&state, client, revision_id, true, req.notes).await?;
    Ok(Json(model))
}

pub async fn reject_design_revision(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(revision_id): Path<Uuid>,
    Json(req): Json<DecisionRequest>,
) -> Result<Json<entity::design_revision::Model>, AppError> {
    let model = decide_revision(&state, client, revision_id, false, req.notes).await?;
    Ok(Json(model))
}

/// Marks every other non-terminal quotation for `project_id` (other than
/// `keep_id`) as superseded, so at most one quotation per project is ever
/// "approved" — the invariant Change Orders rely on for `base_quotation_id`.
async fn supersede_other_quotations(
    txn: &sea_orm::DatabaseTransaction,
    tenant_id: Uuid,
    project_id: Uuid,
    keep_id: Uuid,
    actor: audit::Actor,
) -> Result<(), AppError> {
    let others = entity::prelude::Quotation::find()
        .filter(entity::quotation::Column::ProjectId.eq(project_id))
        .filter(entity::quotation::Column::Id.ne(keep_id))
        .filter(entity::quotation::Column::Status.is_in(["draft", "sent", "approved"]))
        .all(txn)
        .await?;
    for q in others {
        let before = serde_json::json!({ "status": q.status });
        let id = q.id;
        let mut am: entity::quotation::ActiveModel = q.into();
        am.status = Set("superseded".to_string());
        am.update(txn).await?;
        audit::record(
            txn,
            tenant_id,
            "quotation",
            id,
            "update",
            actor,
            Some(before),
            Some(serde_json::json!({ "status": "superseded" })),
        )
        .await?;
    }
    Ok(())
}

async fn decide_quotation(
    state: &AppState,
    client: AuthenticatedClientUser,
    quotation_id: Uuid,
    approve: bool,
    notes: Option<String>,
) -> Result<entity::quotation::Model, AppError> {
    let tenant_id = client.tenant_id;

    state
        .app_db
        .transaction::<_, entity::quotation::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let quotation = entity::prelude::Quotation::find_by_id(quotation_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let project = entity::prelude::Project::find_by_id(quotation.project_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if project.client_id != client.client_id {
                    return Err(AppError::NotFound);
                }
                if quotation.status != "draft" && quotation.status != "sent" {
                    return Err(AppError::BadRequest(format!(
                        "quotation is already {}",
                        quotation.status
                    )));
                }

                let before = serde_json::json!({ "status": quotation.status });
                let new_status = if approve { "approved" } else { "rejected" };
                let project_id = quotation.project_id;

                let mut am: entity::quotation::ActiveModel = quotation.into();
                am.status = Set(new_status.to_string());
                let updated = am.update(txn).await?;

                audit::record(
                    txn,
                    tenant_id,
                    "quotation",
                    quotation_id,
                    "update",
                    audit::Actor::ClientUser(client.client_user_id),
                    Some(before),
                    Some(serde_json::json!({ "status": new_status, "notes": notes })),
                )
                .await?;

                if approve {
                    supersede_other_quotations(
                        txn,
                        tenant_id,
                        project_id,
                        quotation_id,
                        audit::Actor::ClientUser(client.client_user_id),
                    )
                    .await?;
                }

                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)
}

pub async fn approve_quotation(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(quotation_id): Path<Uuid>,
    Json(req): Json<DecisionRequest>,
) -> Result<Json<entity::quotation::Model>, AppError> {
    let model = decide_quotation(&state, client, quotation_id, true, req.notes).await?;
    Ok(Json(model))
}

pub async fn reject_quotation(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(quotation_id): Path<Uuid>,
    Json(req): Json<DecisionRequest>,
) -> Result<Json<entity::quotation::Model>, AppError> {
    let model = decide_quotation(&state, client, quotation_id, false, req.notes).await?;
    Ok(Json(model))
}

pub async fn list_my_change_orders(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::change_order::Model>>, AppError> {
    let tenant_id = client.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::change_order::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let project = entity::prelude::Project::find_by_id(project_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if project.client_id != client.client_id {
                    return Err(AppError::NotFound);
                }
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

async fn decide_change_order(
    state: &AppState,
    client: AuthenticatedClientUser,
    change_order_id: Uuid,
    approve: bool,
    notes: Option<String>,
) -> Result<entity::change_order::Model, AppError> {
    let tenant_id = client.tenant_id;

    state
        .app_db
        .transaction::<_, entity::change_order::Model, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;

                let change_order = entity::prelude::ChangeOrder::find_by_id(change_order_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let project = entity::prelude::Project::find_by_id(change_order.project_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if project.client_id != client.client_id {
                    return Err(AppError::NotFound);
                }
                if change_order.status != "pending_client_approval" {
                    return Err(AppError::BadRequest(format!(
                        "change order is already {}",
                        change_order.status
                    )));
                }

                let before = serde_json::json!({
                    "status": change_order.status,
                    "cost_impact": change_order.cost_impact,
                });
                let new_status = if approve { "approved" } else { "rejected" };

                let mut new_quotation_id: Option<Uuid> = None;

                if approve {
                    new_quotation_id = Some(
                        apply_change_order(txn, tenant_id, &change_order).await?,
                    );
                }

                let co_id = change_order.id;
                let mut am: entity::change_order::ActiveModel = change_order.into();
                am.status = Set(new_status.to_string());
                am.new_quotation_id = Set(new_quotation_id);
                am.decided_by = Set(Some(client.client_user_id));
                am.decided_at = Set(Some(chrono::Utc::now().into()));
                am.decision_notes = Set(notes.clone());
                let updated = am.update(txn).await?;

                audit::record(
                    txn,
                    tenant_id,
                    "change_order",
                    co_id,
                    "update",
                    audit::Actor::ClientUser(client.client_user_id),
                    Some(before),
                    Some(serde_json::json!({
                        "status": new_status,
                        "new_quotation_id": new_quotation_id,
                        "notes": notes,
                    })),
                )
                .await?;

                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)
}

/// Executes an approved Change Order: builds the re-baselined quotation
/// (carry-over + modifications + removals + additions), inserts it as the
/// next version, and supersedes the prior baseline. Returns the new
/// quotation's id. Only called once the change order's status has already
/// been confirmed "pending_client_approval" by the caller.
async fn apply_change_order(
    txn: &sea_orm::DatabaseTransaction,
    tenant_id: Uuid,
    change_order: &entity::change_order::Model,
) -> Result<Uuid, AppError> {
    let co_line_items = entity::prelude::ChangeOrderLineItem::find()
        .filter(entity::change_order_line_item::Column::ChangeOrderId.eq(change_order.id))
        .all(txn)
        .await?;
    let base_line_items = entity::prelude::QuotationLineItem::find()
        .filter(entity::quotation_line_item::Column::QuotationId.eq(change_order.base_quotation_id))
        .all(txn)
        .await?;

    let touched_ids: HashSet<Uuid> = co_line_items
        .iter()
        .filter_map(|li| li.original_line_item_id)
        .collect();

    let next_version = entity::prelude::Quotation::find()
        .filter(entity::quotation::Column::ProjectId.eq(change_order.project_id))
        .order_by_desc(entity::quotation::Column::Version)
        .one(txn)
        .await?
        .map(|q| q.version + 1)
        .unwrap_or(1);

    let new_quotation_id = Uuid::new_v4();
    let quotation_am = entity::quotation::ActiveModel {
        id: Set(new_quotation_id),
        tenant_id: Set(tenant_id),
        project_id: Set(change_order.project_id),
        version: Set(next_version),
        status: Set("approved".to_string()),
        created_by: Set(change_order.requested_by),
        created_at: Set(chrono::Utc::now().into()),
    };
    quotation_am.insert(txn).await?;
    audit::record(
        txn,
        tenant_id,
        "quotation",
        new_quotation_id,
        "create",
        audit::Actor::System,
        None,
        Some(serde_json::json!({
            "project_id": change_order.project_id,
            "version": next_version,
            "re_baselined_from_change_order": change_order.id,
        })),
    )
    .await?;

    // Carry over base-quotation lines that this change order didn't touch.
    for base in base_line_items.iter().filter(|b| !touched_ids.contains(&b.id)) {
        insert_baseline_line(
            txn,
            tenant_id,
            new_quotation_id,
            &base.description,
            base.quantity,
            &base.unit,
            base.unit_rate,
            base.amount,
        )
        .await?;
    }

    // Apply modifications and additions (skip removals entirely).
    for li in co_line_items.iter().filter(|li| !li.removed) {
        insert_baseline_line(
            txn,
            tenant_id,
            new_quotation_id,
            &li.description,
            li.quantity,
            &li.unit,
            li.unit_rate,
            li.amount,
        )
        .await?;
    }

    supersede_other_quotations(
        txn,
        tenant_id,
        change_order.project_id,
        new_quotation_id,
        audit::Actor::System,
    )
    .await?;

    Ok(new_quotation_id)
}

#[allow(clippy::too_many_arguments)]
async fn insert_baseline_line(
    txn: &sea_orm::DatabaseTransaction,
    tenant_id: Uuid,
    quotation_id: Uuid,
    description: &str,
    quantity: rust_decimal::Decimal,
    unit: &str,
    unit_rate: rust_decimal::Decimal,
    amount: rust_decimal::Decimal,
) -> Result<(), AppError> {
    let id = Uuid::new_v4();
    let am = entity::quotation_line_item::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        quotation_id: Set(quotation_id),
        description: Set(description.to_string()),
        quantity: Set(quantity),
        unit: Set(unit.to_string()),
        unit_rate: Set(unit_rate),
        amount: Set(amount),
        created_at: Set(chrono::Utc::now().into()),
    };
    am.insert(txn).await?;
    audit::record(
        txn,
        tenant_id,
        "quotation_line_item",
        id,
        "create",
        audit::Actor::System,
        None,
        Some(serde_json::json!({ "quotation_id": quotation_id, "description": description, "amount": amount })),
    )
    .await?;
    Ok(())
}

pub async fn approve_change_order(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(change_order_id): Path<Uuid>,
    Json(req): Json<DecisionRequest>,
) -> Result<Json<entity::change_order::Model>, AppError> {
    let model = decide_change_order(&state, client, change_order_id, true, req.notes).await?;
    Ok(Json(model))
}

/// Read-only invoice visibility for the client — raising/marking-paid stays
/// an internal Finance action (.ai/project/requirements.md).
pub async fn list_project_invoices(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<entity::invoice::Model>>, AppError> {
    let tenant_id = client.tenant_id;
    let items = state
        .app_db
        .transaction::<_, Vec<entity::invoice::Model>, AppError>(|txn| {
            Box::pin(async move {
                set_tenant(txn, tenant_id).await?;
                let project = entity::prelude::Project::find_by_id(project_id)
                    .one(txn)
                    .await?
                    .ok_or(AppError::NotFound)?;
                if project.client_id != client.client_id {
                    return Err(AppError::NotFound);
                }
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

pub async fn reject_change_order(
    State(state): State<AppState>,
    client: AuthenticatedClientUser,
    Path(change_order_id): Path<Uuid>,
    Json(req): Json<DecisionRequest>,
) -> Result<Json<entity::change_order::Model>, AppError> {
    let model = decide_change_order(&state, client, change_order_id, false, req.notes).await?;
    Ok(Json(model))
}
