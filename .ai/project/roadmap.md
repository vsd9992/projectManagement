modified: 2026-08-27

# Roadmap

## Phase 1 — Planning & Evaluation
**Status: complete.**
- [x] Market/competitive research reviewed.
- [x] Product/business-line shape, localization approach, tenancy model, MVP vertical decided (see `.ai/decisions/current/`).
- [x] System design baseline written (`project-plan.md`, `requirements.md`, `architecture.md`, `workflows.md`, `risks.md`).
- [x] Backend/frontend framework & language decision: Rust/Axum/SeaORM+SQLx/PostgreSQL, React/TypeScript (see `.ai/decisions/current/2026-08-27-technology-stack-backend-frontend.md`).
- [x] Multi-tenant isolation strategy: shared schema + PostgreSQL row-level security (see `.ai/decisions/current/2026-08-27-tenant-isolation-shared-schema-rls.md`).
- [x] Auth/authorization mechanism: server-side sessions + Argon2, single global login, unified session mechanism for internal users and the Client Portal (see `.ai/decisions/current/2026-08-27-auth-session-based-single-login.md`).
- [x] Typed API contract generation: `utoipa` (Axum → OpenAPI) + `orval` (typed client + React Query hooks) — see `.ai/decisions/current/2026-08-27-api-contract-utoipa-orval-react-query.md`.
- [x] Hosting/deployment target: local dev server for development, Kubernetes (Linode or E2E, TBD) for production — see `.ai/decisions/current/2026-08-27-hosting-dev-local-prod-kubernetes.md`.

**All technology-stack items resolved.**

## Phase 2 — Execution (MVP: Turnkey Interiors, full depth)

- **M1 — Foundation**: tenancy/auth/org model (Tenant, Business Unit, User, Role), Project entity with enabled Workstreams, base entity CRUD + Audit Log wired in from the start (per risk #5 in `risks.md`). **Status: complete and verified.**
  - Implementation: Rust/Axum/SeaORM/SQLx/PostgreSQL workspace (`backend/`) — migration crate (core schema + RLS policies), entity crate, Axum API with session-based auth (signup/login/logout), business unit and client CRUD, and project creation with an arbitrary workstream subset.
  - Verification (run 2026-08-27 against a live PostgreSQL instance on the dev server, not just compiled): signed up a tenant, created 2 business units and a client under it; created a turnkey project with 3 workstreams (design, procurement, site_execution) and a separate pure-manufacturing project with 1 workstream (manufacturing) — confirming the composable workstream model handles both shapes on the same entity. `audit_log` row counts matched every create exactly (2 tenants, 2 users, 2 business units, 1 client, 2 projects, 4 project_workstreams). RLS isolation confirmed at both the API level (a second tenant's session sees zero of the first tenant's business units/projects) and directly at the database level (the `app_user` role sees 0 rows with no tenant context set, and only the correct tenant's rows once `SET LOCAL app.tenant_id` is issued).
  - One real bug found and fixed during verification: the tenant-creation audit-log entry referenced `actor_user_id` before the user row existed in the same transaction, violating the FK constraint — fixed by recording that specific entry with no actor (self-signup has no prior actor for the tenant-creation event).
- **M2 — Sales & Design workstream**: Lead → Quotation (versioned BOQ) → Design (versioned drawings/specs) → Client Approval loop.
  - Verification: a design revision can be submitted, and a client-role user can approve/reject it; approval status and full revision history are visible.
- **M3 — Change Orders**: scope change entity, links to WBS/BOQ, client-approval-required flow, re-baselining.
  - Verification: a Change Order against an approved BOQ requires client approval before it takes effect; once approved, affected BOQ/schedule items reflect the new baseline and the audit trail shows before/after.
- **M4 — Delivery workstreams**: internal-facing Procurement (POs to vendors), simplified Manufacturing "production task" tracking, Site Execution (tasks, daily logs, punch list, basic RFI/query log).
  - Verification: a PO can be raised and tracked to delivered status; a site task can be logged and linked as a dependency of an install task; a punch list item can be raised and closed.
- **M5 — Billing & Client Portal**: milestone-based billing (generic engine, India rule profile), minimal Finance role (raise/track invoice, mark paid, track retention), external Client Portal (design approvals, progress view, invoices).
  - Verification: a milestone invoice can be raised against a completed schedule milestone, marked paid, and is visible to the client in the portal; India GST/retention calculations are correct against a hand-computed example.
- **M6 — MVP verification**: run the three scenario walkthroughs from `workflows.md` end-to-end against the actual implementation (not just on paper); confirm no forced workarounds.

## Phase 3 — Testing & Bug Fixing
Begins after M6. Validate implementation against `requirements.md` and `architecture.md`; fix without changing the baseline unless testing reveals the baseline itself needs an approved change.

## After MVP (explicitly deferred, not scheduled yet)
- Standalone deep Furniture Manufacturing vertical (full BOM/shop-floor/QC/dispatch).
- Standalone deep Civil/Architectural vertical (heavy CPM scheduling, formal RFI/submittal/transmittal routing, BIM integration).
- Full Finance & Admin module (general ledger, AP/AR, statutory filing).
- Vendor-facing portal.
- Multi-region billing profiles beyond India.
