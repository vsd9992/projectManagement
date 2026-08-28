# Project Index

**Project**: Multi-tenant PM SaaS for furniture manufacturing, turnkey interiors, and civil/architectural projects.
**Phase**: Execution (Phase 2), MVP = Turnkey Interiors.
**Milestone**: M1, M2, and M3 complete and verified against a live database on the dev server. M4 (Delivery workstreams) not started.
**Active tasks**: None.

## Documents
- `project/project-plan.md` — what/why/scope/success criteria
- `project/requirements.md` — functional & non-functional requirements
- `project/architecture.md` — entities, tenancy model, workstream model
- `project/workflows.md` — project lifecycle, change-order flow, approvals, scenario walkthroughs
- `project/roadmap.md` — phases, milestones, status
- `project/risks.md` — active cross-cutting risks
- `decisions/current/` — 6 foundational decisions (below)

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

## Risk pointers
See `project/risks.md` — 5 active risks (breadth-vs-depth, workstream over-engineering, billing engine India-correctness, traceability retrofit cost, UX-vs-entity-breadth). Multi-tenant isolation risk resolved via `tenant-isolation-shared-schema-rls`.

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
