modified: 2026-08-29

# Architecture

## Tenancy & Organization Model
- **Tenant** — the paying customer account. Owns subscription, global settings, tenant-level configuration (approval chains, tax/billing rule profile, custom fields).
- **Business Unit** (branch / factory / division) — sub-organization under a Tenant. Users and Projects belong to a Business Unit; Tenant owners get roll-up visibility across all Business Units.
- **User & Role** — users belong to a Tenant, are assigned to one or more Business Units, and hold role(s) governing permissions.
- **Client** — external party (company or individual) linked to one or more Projects. Lightweight CRM only — not a full contacts/marketing module. Scoped read/approve-only access via the Client Portal to their own project(s).

## Project Model — Composable, Not a Rigid Type
Instead of a fixed `project_type` enum, a **Project** has one or more **Workstreams** enabled from a fixed catalog:
- **Design** — drawings, specs, 3D renders, revisions, client approval loop
- **Manufacturing/Production** — BOM, work orders, shop floor, QC, dispatch
- **Procurement** — vendor POs, material sourcing, cost tracking
- **Site Execution** — daily logs, installation, punch lists, RFIs, subcontractor coordination

A pure civil project enables only Site Execution (+ light Design); a pure manufacturing project enables only Manufacturing (+ Procurement); a turnkey interiors project enables all four. This is the core mechanism satisfying "mixed / case-by-case" business lines without special-casing project types throughout the system — new workstream types can be added later (e.g. a deeper Civil-specific workstream) without redesigning the Project entity itself.

Each workstream instance has its own status/state machine and progresses independently and concurrently; cross-workstream dependencies are explicit links (e.g. "Install task depends on Procurement PO #4 delivered"), not an assumed global stage order.

A project's enabled-workstream set is enforced, not just descriptive: the API rejects creating a workstream-specific entity (a design asset, production task, purchase order, or site task/daily log/punch list item/site query) against a project that hasn't enabled that workstream. The only way to add a workstream to an already-existing project is via Change Order client approval — see `.ai/decisions/current/2026-08-28-workstream-enforcement-and-expansion.md`.

## Core Domain Entities

| Entity | Purpose |
|---|---|
| Tenant, BusinessUnit, User, Role | Multi-tenant org & access model |
| Client, Lead/Enquiry | Lightweight CRM feeding into Projects |
| Project | Central record; links Client + BusinessUnit; has enabled Workstreams |
| WBS / Scope Item | Hierarchical scope breakdown, each item tagged to a workstream |
| BOQ / Estimation | Versioned line items (qty, rate, cost, margin), linked to WBS |
| Change Order / Variation | Scope change with approval trail (requires client approval), re-baselines BOQ/schedule |
| Design Asset / Drawing | Versioned documents with full revision history + approval status |
| BOM | Bill of materials for manufactured/custom items, linked to a design spec |
| Work Order | Factory production instruction derived from BOM + BOQ |
| Inventory / Material | Stock levels, raw materials, finished goods, movements |
| Purchase Order, Vendor | Procurement, linked to BOQ/BOM line items (internal-facing only in MVP) |
| Site Task, Daily Log, RFI, Punch List Item | Site execution records |
| Schedule Task, Dependency, Milestone | **Implemented (Phase 3)**: `schedule_tasks`/`schedule_task_dependencies` span all four workstreams, with planned/actual dates and basic forward-pass date-shift propagation on a dependency chain (deliberately not full CPM/critical-path scheduling). `Milestone` remains a separate, minimal entity that only exists to gate milestone-based billing — it isn't part of the schedule graph itself. |
| Invoice / Billing Event | **Milestone and progressive (RA-bill-style) implemented (Phase 3 added progressive)**; lump-sum is not. Driven by a pluggable Tax/Region Rule Profile — India implements GST + GST TDS + retention. Mobilization-advance recovery is not implemented (see `risks.md` risk #3). |
| Approval Workflow | **Not actually generic** — Design approval, Change Order approval, and (Phase 3) PO approval are each bespoke, hardcoded logic, not a shared configurable chain. Confirmed and deliberately left unbuilt during Phase 3 (see `risks.md` risk #8) rather than built half-generically. |
| Document/Attachment | Not implemented as a generic entity — only `design_asset`/`design_revision` has versioned-document behavior, scoped to Design. |
| Audit Log | Every state change on every entity above recorded (who/when/what/before→after) |
| Comment/Notification | **Partially implemented (Phase 3)**: in-app `notifications` exist for one specific case — schedule-task date shifts, addressed to the project's internal team. Not the generic "threaded communication tied to any entity" this row originally described; that remains unbuilt. |

## Cross-Cutting Principles
- **Traceability by construction**: audit logging and document versioning are core platform services every entity plugs into, not per-module bolt-ons.
- **Transparency by role**: dashboards are filtered views over the same underlying data — internal roles see full detail scoped to their business unit/workstream; the Client Portal is a curated read/approve subset (design approvals, progress, milestone invoices) of their own project only.
- **Tenant-level configurability**: billing/tax rule profile and workstream labels are configuration via `GET/POST /tenant-settings` (Phase 3), not code — so onboarding a new tenant or region doesn't require a redeploy. Approval chains are **not** configurable yet (see `risks.md` risk #8) — the original composable-per-tenant ambition (per the IFS Cloud vs. SAP lesson in the market research) is only partially realized.
- **Generic billing engine**: billing method (milestone / progressive RA-style / lump-sum) and tax rules are abstracted behind a Region Rule Profile; India is the first concrete profile, built to be one of several, not hardcoded into the core.

## Technology Stack
- **Backend**: Rust, Axum (web framework), SeaORM (primary entity/CRUD layer) + SQLx (raw compile-time-checked SQL for billing calculations, audit-log queries, dependency-graph CTEs), PostgreSQL.
- **Frontend**: React + TypeScript. Node.js is build tooling only, not a server-side layer.
- See `.ai/decisions/current/2026-08-27-technology-stack-backend-frontend.md` for the full rationale and what was explicitly dropped (SurrealDB).

## Multi-Tenant Isolation
Shared database/schema; every tenant-scoped table has a `tenant_id` column enforced by PostgreSQL row-level security. Tenant context is set per-request via `SET LOCAL` inside the request's transaction. See `.ai/decisions/current/2026-08-27-tenant-isolation-shared-schema-rls.md` for full rationale and constraints (two DB roles required: RLS-enforced app role, and a separate `BYPASSRLS` role for platform-admin/cross-tenant tooling).

## Authentication & Authorization
Server-side sessions (opaque token, httpOnly Secure cookie, stored in PostgreSQL), Argon2 password hashing, single global login (tenant resolved from the account, not a subdomain). The Client Portal uses the same session mechanism as internal users, scoped to project access instead of a business-unit role. See `.ai/decisions/current/2026-08-27-auth-session-based-single-login.md` for full rationale.

## API Contract & Frontend Data Layer
**Implemented.** Axum generates an OpenAPI spec via `utoipa` (`backend/api/src/openapi.rs`, browsable at `/api/docs`, raw at `/api/openapi.json`) covering all ~65 endpoints and their request/response schemas. The React frontend (`frontend/`, Vite + React 19 + TypeScript, scaffolded and wired up) consumes it via `orval`, generating a typed fetch client and React Query hooks per endpoint into `frontend/src/api/generated/` (committed to git, regenerated via `npm run generate:api`) — React Query is the frontend's server-state library. Every backend route lives under an `/api` prefix (`Router::new().nest("/api", routes::router())`) so dev (Vite proxy) and a future production reverse proxy/ingress share the same path structure. See `.ai/decisions/current/2026-08-27-api-contract-utoipa-orval-react-query.md` for the original decision and `AGENTS.md`'s "Frontend" section for the dev workflow and utoipa/SeaORM interaction gotchas hit while building this.

## Hosting & Deployment
Development runs against a local Debian dev server (LAN-only; credentials handled out-of-band, never committed). Production target is Kubernetes on either Linode or E2E Networks — provider not yet finalized, not a blocker for Execution. See `.ai/decisions/current/2026-08-27-hosting-dev-local-prod-kubernetes.md`.

## Known Open Architecture Decisions
- Production Kubernetes provider: Linode vs. E2E Networks — to resolve before actual production deployment, not before Execution.
- Concrete state-machine/workflow-engine implementation for concurrent workstreams and dependency graphs — to design during Execution.
- Document storage/versioning backend — to design during Execution.
- Concrete state-machine/workflow-engine implementation for concurrent workstreams and dependency graphs.
- Document storage/versioning backend.
