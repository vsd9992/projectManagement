modified: 2026-08-27

# Requirements

## Functional — Tenancy & Access
- A Tenant is the paying account; it owns subscription, global config (approval chains, billing/tax rule profile, custom fields).
- A Tenant can have multiple Business Units (branch/factory/division); users and projects belong to a Business Unit; tenant owners get roll-up visibility across all business units.
- Users hold role(s) governing permissions. Day-one role groups: Sales & Design, Delivery (Procurement/Factory/Site), a minimal Finance role, and an external Client role.
- A Client is an external party scoped to their own project(s) only, via the Client Portal, with read/approve-only access (no internal system access).

## Functional — Project & Workstreams
- A Project must support enabling any combination of four workstreams: Design, Manufacturing/Production, Procurement, Site Execution — not a fixed project-type enum. (A pure civil project enables Site Execution [+ light Design]; a pure manufacturing project enables Manufacturing [+ Procurement]; a turnkey interiors project enables all four.)
- Each workstream instance has its own status/state machine and can progress independently and concurrently with the others in the same project.
- Cross-workstream dependencies must be expressible as explicit links between tasks (e.g. "Install task depends on PO #4 delivered"), not assumed from a global stage order.

## Functional — Scope, Estimation & Change
- A WBS (Work Breakdown Structure) provides hierarchical scope decomposition; each item is tagged to a workstream.
- A BOQ (Bill of Quantities/Estimation) holds versioned line items (qty, rate, cost, margin) linked to WBS items.
- A Change Order (Variation) is required for any scope extension or reduction. It must link to the specific BOQ/WBS items it modifies, and **requires formal client approval before it is binding** — a PM-only sign-off is not sufficient. Every change order re-baselines budget and schedule and is fully audited (who requested, who approved, before/after cost and schedule impact).

## Functional — Design, Manufacturing, Procurement, Site Execution
- Design Assets (drawings/specs/renders) must be versioned with full revision history and an approval status, with a client approval loop.
- Manufacturing: BOM linked to a design spec, Work Orders derived from BOM + BOQ, Inventory/Material tracking (stock levels, movements). MVP depth: simplified "production task" tracking only — full shop-floor/QC/dispatch is deferred (see `roadmap.md`).
- Procurement: Purchase Orders and Vendor master data, linked to BOQ/BOM line items. **MVP is internal-facing only** — no vendor-facing portal (vendors do not get login access; PO status is updated by internal staff).
- Site Execution: Site Tasks, Daily Logs, RFIs, Punch List items. MVP depth: task dependencies and a simple RFI/query log — full CPM scheduling and formal submittal/transmittal routing are deferred.

## Functional — Billing
- Billing engine must be generic: support milestone-based, progressive (RA-bill-style), and lump-sum methods, with tax/statutory rules abstracted behind a pluggable Region Rule Profile.
- India is the first concrete profile: GST works-contract SAC codes, TDS, RA billing with mobilization-advance recovery, retention deductions.
- **MVP Finance scope is minimal**: raise and track milestone invoices, mark paid, track retention — explicitly not a full general ledger/AP-AR/statutory-filing module.

## Functional — Traceability & Communication
- Every state change on every core entity (Project, WBS, BOQ, Change Order, Design Asset, Work Order, PO, Site Task, Invoice, Approval) must produce an audit log entry: who, when, what, before → after.
- Every document (drawing, spec, contract) must carry a full version lineage, not just a "latest" pointer.
- Comments/notifications must be threadable against any core entity, to close the coordination-fragmentation gap (currently WhatsApp/Excel/email).
- Dashboards are filtered views over the same underlying data, not separate reporting systems — internal roles see full detail scoped to their business unit/workstream; the Client Portal is a curated subset (design approvals, progress, milestone invoices) of the client's own project.

## Non-Functional
- **Configurability over code**: approval chains, billing/tax rule profile, and workstream labels must be tenant-level configuration, not require a redeploy — this is a hard lesson from the market research (composable IFS Cloud vs. rigid SAP).
- **Extensibility**: new workstream types (e.g. a deeper Civil-specific workstream) must be addable without redesigning the core Project entity.
- **Usability**: the system must remain intuitive and agile despite its breadth — this is a product constraint, evaluated at UI/UX design time, not just a preference.
- Multi-tenant data isolation strategy is a stack-level decision, deferred to the architecture/stack discussion (see `risks.md`).

## Business Rules
- A Change Order is not binding until the client has formally approved it (not merely notified).
- No vendor/subcontractor has portal access in MVP; all procurement communication and status updates are internal-facing.
- Finance-role users in MVP can raise/track invoices and mark payments/retention, but cannot access a general ledger or AP/AR (that module does not exist yet).

## Acceptance Requirements
See `project-plan.md` → Success Criteria, and `roadmap.md` → per-milestone verification criteria.
