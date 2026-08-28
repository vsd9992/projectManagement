# Decision: "Role" in requirements.md describes capability scope, not enforced access control — no RBAC yet

**Decision**: Through M1–M5, `user_business_unit_role` rows exist (a user can be assigned "sales_design", "delivery", or "finance" for a business unit) but nothing checks them. Every internal-facing endpoint is gated only by "is this an authenticated tenant user" (`AuthenticatedUser`), not by which role that user holds. This applies uniformly — Sales & Design, Delivery, and Finance actions are all equally unenforced.

**Basis**: Surfaced explicitly while building M5, where `requirements.md` names a "minimal Finance role" for invoice actions. Rather than add role-checking only for Finance (the milestone at hand), the existing pattern from M2–M4 — no role enforcement anywhere — was kept consistent.

**Why**: Requirements.md's Finance-role language ("Finance-role users in MVP can raise/track invoices... but cannot access a general ledger") reads as scoping *what the Finance capability includes*, not mandating that access to it be technically restricted by role in the MVP. Adding RBAC enforcement for exactly one workstream while leaving Sales & Design, Delivery, and Procurement all unrestricted would be an inconsistent, arbitrary line — worse than either enforcing it everywhere or nowhere. Since no milestone's verification criteria have required role-gated access, and building real RBAC (role assignment endpoints, permission checks per action, business-unit scoping of those checks) is a genuine cross-cutting feature in its own right, it was deferred wholesale rather than half-built.

**Consequences/constraints**:
- Any authenticated internal user in a tenant can currently perform any internal action (create quotations, approve nothing client-side, raise invoices, mark POs delivered, etc.) — this is a real gap before production use, not a rounding error, and must be closed before this system handles real customers.
- `user_business_unit_role` data is already being collected (business units, users, roles can all be created) — when RBAC is built, it plugs into existing data rather than needing a new assignment mechanism.
- If a future milestone needs role-gated access to satisfy its own verification criteria, build enforcement generally at that point (a shared Axum extractor/middleware checking role membership), not as a one-off for that milestone.
