use sea_orm::{ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::error::AppError;

/// Confirms `user_id` belongs to `business_unit_id` — optionally requiring a
/// specific role, or (when `required_role` is `None`) any role at all, i.e.
/// plain membership. This is the enforcement `.ai/decisions/current/2026-08-28-no-rbac-enforcement-yet.md`
/// flagged as missing: business-unit membership, not just role, since a
/// tenant can have multiple branches with separate teams.
pub async fn require_business_unit_role(
    txn: &DatabaseTransaction,
    user_id: Uuid,
    business_unit_id: Uuid,
    required_role: Option<&str>,
) -> Result<(), AppError> {
    let mut query = entity::prelude::UserBusinessUnitRole::find()
        .filter(entity::user_business_unit_role::Column::UserId.eq(user_id))
        .filter(entity::user_business_unit_role::Column::BusinessUnitId.eq(business_unit_id));
    if let Some(role) = required_role {
        query = query.filter(entity::user_business_unit_role::Column::Role.eq(role));
    }
    let exists = query.one(txn).await?.is_some();
    if !exists {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Looks up `project_id`'s business unit, then confirms `user_id` has
/// (optionally role-scoped) membership in it. The one-extra-lookup cost is
/// deliberate — see the RBAC decision record for why this is a Rust-level
/// check rather than an RLS policy (avoids touching the Client Portal's
/// already-verified access path, which is scoped by client_id, not role).
pub async fn require_project_business_unit_role(
    txn: &DatabaseTransaction,
    user_id: Uuid,
    project_id: Uuid,
    required_role: Option<&str>,
) -> Result<(), AppError> {
    let project = entity::prelude::Project::find_by_id(project_id)
        .one(txn)
        .await?
        .ok_or(AppError::NotFound)?;
    require_business_unit_role(txn, user_id, project.business_unit_id, required_role).await
}

/// Returns the business_unit_ids `user_id` belongs to (optionally scoped to
/// a specific role) — for scoping list endpoints to "my team's data"
/// instead of the whole tenant. Note: this does not implement the "tenant
/// owners get roll-up visibility across all business units" requirement
/// from the original design — there is no distinct owner/admin role in the
/// catalog yet, so every user is scoped to their own memberships only. That
/// gap is tracked, not silently dropped.
pub async fn accessible_business_units(
    txn: &DatabaseTransaction,
    user_id: Uuid,
    required_role: Option<&str>,
) -> Result<Vec<Uuid>, AppError> {
    use sea_orm::QuerySelect;
    let mut query = entity::prelude::UserBusinessUnitRole::find()
        .filter(entity::user_business_unit_role::Column::UserId.eq(user_id));
    if let Some(role) = required_role {
        query = query.filter(entity::user_business_unit_role::Column::Role.eq(role));
    }
    let ids: Vec<Uuid> = query
        .select_only()
        .column(entity::user_business_unit_role::Column::BusinessUnitId)
        .into_tuple()
        .all(txn)
        .await?;
    Ok(ids)
}
