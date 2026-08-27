modified: 2026-08-27

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
| Schedule Task, Dependency, Milestone | Concurrent-capable scheduling; milestones drive billing |
| Invoice / Billing Event | Generic engine — milestone, progressive (RA-bill-style), or lump-sum, driven by a pluggable Tax/Region Rule Profile (India profile implements GST SAC codes, TDS, mobilization-advance recovery, retention) |
| Approval Workflow | Generic configurable approval chain, reused by Design approval, Change Orders (client-approval-required), and POs |
| Document/Attachment | Generic versioned file entity attachable to any of the above |
| Audit Log | Every state change on every entity above recorded (who/when/what/before→after) |
| Comment/Notification | Threaded communication tied to any entity — addresses coordination fragmentation |

## Cross-Cutting Principles
- **Traceability by construction**: audit logging and document versioning are core platform services every entity plugs into, not per-module bolt-ons.
- **Transparency by role**: dashboards are filtered views over the same underlying data — internal roles see full detail scoped to their business unit/workstream; the Client Portal is a curated read/approve subset (design approvals, progress, milestone invoices) of their own project only.
- **Tenant-level configurability**: approval chains, billing/tax rule profile, and workstream labels are configuration, not code — so onboarding a new tenant or region doesn't require a redeploy (composable, per the IFS Cloud vs. SAP lesson in the market research).
- **Generic billing engine**: billing method (milestone / progressive RA-style / lump-sum) and tax rules are abstracted behind a Region Rule Profile; India is the first concrete profile, built to be one of several, not hardcoded into the core.

## Technology Stack
- **Backend**: Rust, Axum (web framework), SeaORM (primary entity/CRUD layer) + SQLx (raw compile-time-checked SQL for billing calculations, audit-log queries, dependency-graph CTEs), PostgreSQL.
- **Frontend**: React + TypeScript. Node.js is build tooling only, not a server-side layer.
- See `.ai/decisions/current/2026-08-27-technology-stack-backend-frontend.md` for the full rationale and what was explicitly dropped (SurrealDB).

## Known Open Architecture Decisions (remaining stack items)
- Multi-tenant data isolation strategy (Postgres row-level security via session variables vs. schema-per-tenant vs. database-per-tenant) — leaning RLS, not yet locked.
- Typed API contract generation from Axum to the React frontend (leaning `utoipa` for OpenAPI generation) — not yet locked.
- Authentication/authorization mechanism — not yet discussed.
- Hosting/deployment target — not yet discussed.
- Concrete state-machine/workflow-engine implementation for concurrent workstreams and dependency graphs.
- Document storage/versioning backend.
