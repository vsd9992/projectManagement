# Project Index

**Project**: Multi-tenant PM SaaS for furniture manufacturing, turnkey interiors, and civil/architectural projects.
**Phase**: Planning & Evaluation — baseline established; backend/frontend stack, tenant isolation, and auth locked (Rust/Axum/SeaORM+SQLx/PostgreSQL with shared-schema RLS, session-based auth, React/TypeScript); remaining stack items (API contract generation, hosting) being resolved one at a time.
**Milestone**: None started — Execution has not begun.
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

## Risk pointers
See `project/risks.md` — 5 active risks (breadth-vs-depth, workstream over-engineering, billing engine India-correctness, traceability retrofit cost, UX-vs-entity-breadth). Multi-tenant isolation risk resolved via `tenant-isolation-shared-schema-rls`.

## History
- 2026-08-27 — Baseline system design (entities, tenancy, workflow, MVP scope) established through planning discussion; six foundational decisions recorded; AGENTS.md/.ai/ structure bootstrapped.
- 2026-08-27 — Backend/frontend technology stack locked (Rust/Axum/SeaORM+SQLx/PostgreSQL, React/TypeScript); remaining stack sub-decisions to follow one at a time.
- 2026-08-27 — Multi-tenant isolation strategy locked (shared schema + PostgreSQL RLS).
- 2026-08-27 — Auth/authorization mechanism locked (server-side sessions, single global login, unified Client Portal auth).
