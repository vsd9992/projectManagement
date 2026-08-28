# Project Index

**Project**: Multi-tenant PM SaaS for furniture manufacturing, turnkey interiors, and civil/architectural projects.
**Phase**: Execution (Phase 2), MVP = Turnkey Interiors.
**Milestone**: M1–M5 complete; RBAC/business-unit-scoping hardening pass also complete (inserted before M6 after a risk review). M6 (MVP scenario verification) not started.
**Active tasks**: None.

## Documents
- `project/project-plan.md` — what/why/scope/success criteria
- `project/requirements.md` — functional & non-functional requirements
- `project/architecture.md` — entities, tenancy model, workstream model
- `project/workflows.md` — project lifecycle, change-order flow, approvals, scenario walkthroughs
- `project/roadmap.md` — phases, milestones, status
- `project/risks.md` — active cross-cutting risks
- `decisions/current/` — 14 active decisions recorded (below); 1 archived (superseded)

## Decision pointers (current)
- `multi-tenant-saas-product` — SaaS from day one, not internal tool
- `composable-workstream-project-model` — Project = composable Workstreams, not a rigid type enum
- `generic-billing-engine-india-first-profile` — generic billing/tax engine, India is first profile
- `mvp-scope-turnkey-interiors-first` — MVP = Turnkey Interiors, full depth, before other verticals
- `change-order-requires-client-approval` — Change Orders not binding without formal client approval
- `mvp-finance-and-vendor-access-scope` — minimal Finance role in MVP; procurement internal-facing only
- `technology-stack-backend-frontend` — Rust/Axum/SeaORM+SQLx/PostgreSQL backend, React/TypeScript frontend; SurrealDB dropped
- `tenant-isolation-shared-schema-rls` — shared schema + Postgres row-level security for multi-tenant isolation
- `auth-session-based-single-login` — server-side sessions + Argon2, single global login, unified auth for internal + Client Portal
- `api-contract-utoipa-orval-react-query` — utoipa OpenAPI spec + orval-generated client/React Query hooks
- `hosting-dev-local-prod-kubernetes` — local dev server; production Kubernetes on Linode or E2E (TBD)
- `client-portal-auth-pulled-into-m2` — Client Portal login/auth built in M2 (not deferred to M5) since design approval needs a real client actor
- `rbac-business-unit-scoping-implemented` — role + business-unit-membership enforcement across all internal endpoints; supersedes archived `no-rbac-enforcement-yet`

## Risk pointers
See `project/risks.md` — 7 active risks (breadth-vs-depth, workstream over-engineering, billing engine India-correctness [GST/TDS/retention verified, mobilization advance still missing], traceability retrofit cost, UX-vs-entity-breadth, no admin/owner role, no automated regression suite for M1–M5). Multi-tenant isolation and RBAC/business-unit-scoping risks resolved.

## History
- 2026-08-27 — Baseline system design (entities, tenancy, workflow, MVP scope) established through planning discussion; six foundational decisions recorded; AGENTS.md/.ai/ structure bootstrapped.
- 2026-08-27 — Backend/frontend technology stack locked (Rust/Axum/SeaORM+SQLx/PostgreSQL, React/TypeScript); remaining stack sub-decisions to follow one at a time.
- 2026-08-27 — Multi-tenant isolation strategy locked (shared schema + PostgreSQL RLS).
- 2026-08-27 — Auth/authorization mechanism locked (server-side sessions, single global login, unified Client Portal auth).
- 2026-08-27 — API contract generation locked (utoipa + orval); React Query adopted as frontend data-fetching library.
- 2026-08-27 — Hosting locked (local dev server for development; Kubernetes on Linode or E2E for production, provider TBD). Full technology-stack baseline complete.
- 2026-08-27 — M1 (Foundation) built and verified against a live PostgreSQL instance on the dev server: tenancy/BU/project/workstream/audit_log all confirmed working, including RLS tenant isolation at the DB level. One bug found and fixed (audit-log FK ordering during signup).
- 2026-08-27 — M2 (Sales & Design workstream) built and verified: lead-to-project conversion, versioned quotations, versioned design revisions with real client-portal approve/reject, and cross-client isolation within a tenant — all confirmed against the live database, not just compiled.
- 2026-08-28 — M3 (Change Orders) built and verified: quotation approve/reject (deferred from M2), Change Orders that add/modify/remove BOQ lines with a computed cost impact, client-approval-required re-baselining into a new quotation version, rejection leaving the baseline untouched, and audit_log showing explicit before/after for both outcomes. Schedule re-baselining explicitly deferred — no Schedule entity exists yet.
- 2026-08-28 — M4 (Delivery workstreams) built and verified: vendors/POs tracked to delivered, site tasks with explicit dependency links, punch list raise/close, plus production tasks/daily logs/site queries — all internal-facing, all confirmed against the live database with audit_log counts matching exactly.
- 2026-08-28 — M5 (Billing & Client Portal) built and verified: milestone-gated invoices via a pluggable IndiaGstProfile (GST 18% + GST TDS 2%), verified against independently hand-computed figures; mark-paid, client visibility, and cross-client isolation all confirmed. Recorded that no RBAC enforcement exists yet (consistent gap since M2, not Finance-specific) as an active risk.
- 2026-08-28 — Pre-M6 risk/decision review, at the user's request, before proceeding to M6. Surfaced that "no RBAC" was actually two gaps (role AND business-unit membership unchecked), plus two smaller undiscovered gaps (no automated tests, no admin session-revocation despite that being the session-auth decision's own rationale). User chose to fix RBAC now rather than after M6, bundling the smaller gaps in. Implemented `api::authz` enforcement across every internal endpoint, `POST /business-units/:id/roles` (role assignment — didn't exist), `POST /users` (second internal user per tenant — didn't exist, so "separate teams" was previously untestable), `POST /users/:id/revoke-sessions`, and a new integration-test suite (`backend/api/tests/authz.rs`) run against a dedicated test database. Verified against both the new test suite and the existing M1–M5 dev-server data (pre-existing user correctly locked out, then restored via retroactive role assignment).
