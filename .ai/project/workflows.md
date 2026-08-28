modified: 2026-08-29

# Workflows

## Project Lifecycle (typical shape, not a mandated pipeline)
Lead → Quotation/BOQ → Design → Client Approval → Procurement/Production Planning → Manufacturing and/or Site Execution → Installation → Snagging/Punch-list → Handover → Billing milestones (throughout) → Warranty/AMC.

**This is not strictly sequential.** Design, procurement, and site execution can run in parallel, and scope can be extended or reduced mid-project. The workflow engine models this as a dependency graph across workstream tasks (see `architecture.md` § Project Model), not a single linear stage-gate pipeline. A task in one workstream may declare an explicit dependency on a task in another (e.g. an Install task depends on a specific PO being delivered); absent such a link, workstreams progress independently.

## Change Order / Scope Change Flow
1. A scope change (extension or reduction) is raised as a Change Order, referencing the specific WBS/BOQ line items it affects. A Change Order can also request enabling a new workstream on the project (e.g. adding Site Execution to a project that started design-and-manufacturing-only) — the only way to expand a project's enabled-workstream set once created, since workstream membership is enforced at the API layer (`architecture.md`).
2. The Change Order captures the cost and schedule impact (before → after) and routes for approval.
3. **The client must formally approve the Change Order before it is binding.** Internal PM sign-off alone is not sufficient — this is a business rule, not just a notification step.
4. On approval, the Change Order re-baselines the affected BOQ lines (always) and can spawn new schedule tasks for added scope if the Change Order explicitly requested them (`add_schedule_tasks` — Phase 3; not automatically derived from BOQ line changes), and the full before/after trail is retained in the Audit Log (see `architecture.md`). A newly spawned schedule task can carry a dependency on an existing task in the same project.
5. Downstream workstreams (design, procurement, site) pick up new/changed WBS items without disturbing already-completed work, because dependencies are explicit links rather than a global stage pointer.

## Approval Workflows (general pattern)
Each of these exists as its own bespoke implementation today, **not** a shared generic configurable chain — the originally-envisioned generic Approval Workflow entity remains unbuilt (deliberately, per `risks.md` risk #8, to avoid over-engineering a half-generic version):
- **Design approval**: client approves drawings/specs before they're released to procurement/manufacturing/site.
- **Change Order approval**: client approval required (see above) before the change is binding.
- **Purchase Order approval** (Phase 3): internal approval only (no vendor-facing step, since procurement is internal-facing only — see `requirements.md`) — a PO starts `pending_approval` and must be approved (gated by the same `delivery` role that creates it, since no distinct approver role exists) before it can be marked delivered.

Exact approver chains beyond "client must approve Change Orders and Design, internal delivery-role approval gates a PO" (e.g. multi-level internal sign-off thresholds, tenant-configurable chains) are not yet defined and should be resolved when the generic Approval Workflow entity is actually built, not assumed.

## Scenario Walkthroughs (used to sanity-check the model — see `project-plan.md` Success Criteria)
- **Pure furniture order** (no site work): Project enables only Manufacturing + Procurement workstreams. BOQ is effectively BOM-driven. Design workstream is optional/light (spec sheet, not full drawings).
- **Pure civil job**: Project enables only Site Execution (+ light Design for approved drawings). RFI/punch-list/daily-log entities apply directly; billing uses the progressive RA-bill-style method (`billing_method: "progressive"` on the project — implemented Phase 3) rather than milestone billing.
- **Turnkey interior with a mid-project scope change** (e.g. client adds a room): a Change Order is raised against the existing BOQ, requires client approval, re-baselines the BOQ, and can spawn new schedule tasks for the added scope (Phase 3) if explicitly requested in the Change Order. There is still no WBS/Scope-Item entity as such — schedule tasks are the closest implemented analog — and schedule-graph re-baselining is opt-in per Change Order, not an automatic derivation from every BOQ change.

## MVP Workflow Depth
Per `roadmap.md`, the MVP (Turnkey Interiors) implements the full flow above. Manufacturing and Site Execution workstreams run at simplified/basic depth within MVP; their full depth (shop-floor/QC/dispatch for Manufacturing; CPM scheduling and formal RFI/submittal/transmittal routing for Site Execution/Civil) is built out later as standalone verticals.
