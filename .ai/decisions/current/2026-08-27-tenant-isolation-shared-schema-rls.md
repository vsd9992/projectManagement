# Decision: Multi-tenant isolation via shared schema + PostgreSQL row-level security

**Decision**: All tenants share one database and schema. Every tenant-scoped table carries a `tenant_id` column, and PostgreSQL row-level security (RLS) policies enforce that a query only sees rows matching the tenant bound to the current transaction. The application sets tenant context per-request via `SET LOCAL app.tenant_id = '...'` inside the request's transaction (not on the pooled connection itself, to avoid leaking tenant context between requests that reuse the same pooled connection). The application's normal DB role does not have `BYPASSRLS`; a separate, explicitly elevated role is used for platform-admin/cross-tenant operations.

**Basis**: Chosen over schema-per-tenant and database-per-tenant during the tenant-isolation stack discussion, following the locked backend stack (`.ai/decisions/current/2026-08-27-technology-stack-backend-frontend.md`).

**Why**: At this stage (small team, expected tenant count in the dozens-to-low-hundreds early on), shared-schema + RLS is the cheapest to run and simplest to migrate (one schema, one migration path via `sea-orm-migration`), while still enforcing isolation at the database level rather than relying on every query remembering a `WHERE tenant_id = ?` clause — which fits the project's top priority of full traceability/correctness better than application-level-only filtering. Schema-per-tenant and database-per-tenant were rejected as the *default* because their migration and connection-pooling overhead multiplies per tenant, which is unnecessary machinery at this stage. Critically, choosing RLS now does not foreclose offering a dedicated database to one specific large/compliance-sensitive client later — because every table is already partitioned by `tenant_id`, migrating that one tenant to its own database is a data-migration task, not a schema redesign.

**Consequences/constraints**:
- Every tenant-scoped table needs a `tenant_id` column and a corresponding RLS policy from the first migration onward — this must be part of the base schema, not retrofitted.
- Axum middleware must resolve the authenticated user's tenant and issue `SET LOCAL app.tenant_id = ...` at the start of each request's DB transaction, before any tenant-scoped query runs.
- Two DB roles are needed: the normal application role (RLS-enforced) and a separate elevated role (`BYPASSRLS`) reserved for platform-admin/cross-tenant tooling — never the same role.
- This resolves risk #4 in `.ai/project/risks.md` (multi-tenant data isolation undecided).
