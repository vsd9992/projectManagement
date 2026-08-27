# Decision: Client Portal authentication was built in M2, not deferred to M5

**Decision**: The `client_users` table and the Client Portal session/auth mechanism (extending `sessions` to carry either an internal-user or client-user principal) were implemented as part of M2, even though the original roadmap grouped "Client Portal" under M5 ("Billing & Client Portal").

**Basis**: M2's own verification criterion — "a design revision can be submitted, and a client-role user can approve/reject it" — requires a genuine external actor making that decision. Discovered while implementing M2 design approval.

**Why**: The alternative was to fake the client's approval through an internal user acting "on behalf of" the client (e.g. a sales rep marking approval after an email/verbal sign-off). That would have undermined the project's top priority — full transparency and traceability — for the exact interaction where it matters most: a client-facing approval decision. The auth architecture (`.ai/decisions/current/2026-08-27-auth-session-based-single-login.md`) was already designed to support both principal types through one session mechanism specifically so this wouldn't require a parallel auth system later; M2 simply exercised that design earlier than the roadmap assumed.

**Consequences/constraints**:
- `audit_log` was extended with `actor_client_user_id` (alongside `actor_user_id`) so client decisions are attributed as precisely as internal-user actions — see the `audit_log_single_actor` CHECK constraint in `m20260827_000002_add_sales_design_workstream`.
- Every client-facing handler must filter by `client_id` explicitly (in addition to RLS's tenant scoping), since RLS alone would let a client see every project in the tenant, not just their own — verified directly in M2 testing (a second client under the same tenant got a 404, not a data leak, when probing the first client's project and revision).
- M5 is now scoped to milestone billing and extending the already-built portal with invoice visibility, not building Client Portal login from scratch — `roadmap.md` M5 updated accordingly.
- The client invite/onboarding flow is still manual for now (an internal user sets an initial password directly via `POST /clients/:id/users`, not an email-invite token flow) — that remains a known simplification, not addressed by this decision.
