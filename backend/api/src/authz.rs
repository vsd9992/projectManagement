use sea_orm::{ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::{auth::session::AuthenticatedUser, error::AppError};

/// Confirms `user` belongs to `business_unit_id` — optionally requiring a
/// specific role, or (when `required_role` is `None`) any role at all, i.e.
/// plain membership. A tenant admin always passes, regardless of
/// membership — they have authority across the whole tenant (see
/// .ai/decisions/current/2026-08-28-tenant-admin-and-platform-manager.md).
pub async fn require_business_unit_role(
    txn: &DatabaseTransaction,
    user: AuthenticatedUser,
    business_unit_id: Uuid,
    required_role: Option<&str>,
) -> Result<(), AppError> {
    if user.is_tenant_admin {
        return Ok(());
    }
    let mut query = entity::prelude::UserBusinessUnitRole::find()
        .filter(entity::user_business_unit_role::Column::UserId.eq(user.user_id))
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

/// Looks up `project_id`'s business unit, then confirms `user` has
/// (optionally role-scoped) membership in it — or is a tenant admin. The
/// one-extra-lookup cost is deliberate: see the RBAC decision record for why
/// this is a Rust-level check rather than an RLS policy (avoids touching the
/// Client Portal's already-verified access path, which is scoped by
/// client_id, not role).
pub async fn require_project_business_unit_role(
    txn: &DatabaseTransaction,
    user: AuthenticatedUser,
    project_id: Uuid,
    required_role: Option<&str>,
) -> Result<(), AppError> {
    let project = entity::prelude::Project::find_by_id(project_id)
        .one(txn)
        .await?
        .ok_or(AppError::NotFound)?;
    require_business_unit_role(txn, user, project.business_unit_id, required_role).await
}

/// Confirms `project_id` has `workstream` enabled. This is the guardrail
/// that makes a project's enabled-workstream set (architecture.md's "one or
/// more Workstreams") actually restrict what can be recorded against it,
/// not just describe it (see .ai/project/risks.md risk #7 and
/// .ai/decisions/current/2026-08-28-workstream-enforcement-and-expansion.md).
/// Call this alongside `require_project_business_unit_role` at the *root*
/// creation endpoint for each workstream-specific entity family
/// (design_asset, production_task, purchase_order, site_task/daily_log/
/// punch_list_item/site_query) — not on reads/lists, and not on children of
/// an already-gated root (e.g. design_revision, whose parent design_asset
/// couldn't exist without this check already having passed, and no
/// endpoint ever removes a workstream once enabled).
pub async fn require_project_workstream(
    txn: &DatabaseTransaction,
    project_id: Uuid,
    workstream: entity::workstream_type::WorkstreamType,
) -> Result<(), AppError> {
    let exists = entity::prelude::ProjectWorkstream::find()
        .filter(entity::project_workstream::Column::ProjectId.eq(project_id))
        .filter(entity::project_workstream::Column::WorkstreamType.eq(workstream.clone()))
        .one(txn)
        .await?
        .is_some();
    if !exists {
        return Err(AppError::BadRequest(format!(
            "project does not have the '{}' workstream enabled",
            workstream_label(&workstream)
        )));
    }
    Ok(())
}

/// For tenant-wide entities with no `business_unit_id` of their own
/// (vendors, clients) — confirms `user` holds `required_role` in *some*
/// business unit in the tenant, or is a tenant admin. When `required_role`
/// is `None`, any role (plain membership) passes. Weaker than the
/// project-scoped checks above (any holder of the role tenant-wide passes,
/// not just one BU's), which is a real limitation given the entity itself
/// isn't BU-scoped — see .ai/decisions/current/2026-08-28-phase-3-audit-and-expansion.md.
pub async fn require_any_business_unit_role(
    txn: &DatabaseTransaction,
    user: AuthenticatedUser,
    required_role: Option<&str>,
) -> Result<(), AppError> {
    if user.is_tenant_admin {
        return Ok(());
    }
    let ids = accessible_business_units(txn, user, required_role).await?;
    if ids.is_empty() {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

fn workstream_label(w: &entity::workstream_type::WorkstreamType) -> &'static str {
    use entity::workstream_type::WorkstreamType::*;
    match w {
        Design => "design",
        Manufacturing => "manufacturing",
        Procurement => "procurement",
        SiteExecution => "site_execution",
    }
}

/// Fails unless `user` is their tenant's admin.
pub fn require_tenant_admin(user: AuthenticatedUser) -> Result<(), AppError> {
    if user.is_tenant_admin {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

/// Returns the business_unit_ids `user` can act on (optionally scoped to a
/// specific role) — for scoping list endpoints to "my team's data" instead
/// of the whole tenant. A tenant admin gets every business unit in the
/// tenant, which is the "tenant owners get roll-up visibility across all
/// business units" requirement from the original design finally landing.
pub async fn accessible_business_units(
    txn: &DatabaseTransaction,
    user: AuthenticatedUser,
    required_role: Option<&str>,
) -> Result<Vec<Uuid>, AppError> {
    use sea_orm::QuerySelect;

    if user.is_tenant_admin {
        let ids: Vec<Uuid> = entity::prelude::BusinessUnit::find()
            .select_only()
            .column(entity::business_unit::Column::Id)
            .into_tuple()
            .all(txn)
            .await?;
        return Ok(ids);
    }

    let mut query = entity::prelude::UserBusinessUnitRole::find()
        .filter(entity::user_business_unit_role::Column::UserId.eq(user.user_id));
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
