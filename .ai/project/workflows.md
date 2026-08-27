modified: 2026-08-27

# Workflows

## Project Lifecycle (typical shape, not a mandated pipeline)
Lead → Quotation/BOQ → Design → Client Approval → Procurement/Production Planning → Manufacturing and/or Site Execution → Installation → Snagging/Punch-list → Handover → Billing milestones (throughout) → Warranty/AMC.

**This is not strictly sequential.** Design, procurement, and site execution can run in parallel, and scope can be extended or reduced mid-project. The workflow engine models this as a dependency graph across workstream tasks (see `architecture.md` § Project Model), not a single linear stage-gate pipeline. A task in one workstream may declare an explicit dependency on a task in another (e.g. an Install task depends on a specific PO being delivered); absent such a link, workstreams progress independently.

## Change Order / Scope Change Flow
1. A scope change (extension or reduction) is raised as a Change Order, referencing the specific WBS/BOQ line items it affects.
2. The Change Order captures the cost and schedule impact (before → after) and routes for approval.
3. **The client must formally approve the Change Order before it is binding.** Internal PM sign-off alone is not sufficient — this is a business rule, not just a notification step.
4. On approval, the Change Order re-baselines the affected BOQ lines and schedule tasks, and the full before/after trail is retained in the Audit Log (see `architecture.md`).
5. Downstream workstreams (design, procurement, site) pick up new/changed WBS items without disturbing already-completed work, because dependencies are explicit links rather than a global stage pointer.

## Approval Workflows (general pattern)
Approval Workflow is a generic, configurable chain reused across the system:
- **Design approval**: client approves drawings/specs before they're released to procurement/manufacturing/site.
- **Change Order approval**: client approval required (see above) before the change is binding.
- **Purchase Order approval**: internal approval only in MVP (no vendor-facing step, since procurement is internal-facing only — see `requirements.md`).

Exact approver chains beyond "client must approve Change Orders and Design" (e.g. multi-level internal sign-off thresholds) are not yet defined and should be resolved when the Approval Workflow entity is implemented, not assumed.

## Scenario Walkthroughs (used to sanity-check the model — see `project-plan.md` Success Criteria)
- **Pure furniture order** (no site work): Project enables only Manufacturing + Procurement workstreams. BOQ is effectively BOM-driven. Design workstream is optional/light (spec sheet, not full drawings).
- **Pure civil job**: Project enables only Site Execution (+ light Design for approved drawings). RFI/punch-list/daily-log entities apply directly; billing likely uses the progressive RA-bill-style method rather than milestone billing.
- **Turnkey interior with a mid-project scope change** (e.g. client adds a room): a Change Order is raised against the existing WBS/BOQ, spawns new WBS items and BOQ lines, requires client approval, re-baselines the schedule graph, and every downstream workstream picks up the new items independently.

## MVP Workflow Depth
Per `roadmap.md`, the MVP (Turnkey Interiors) implements the full flow above. Manufacturing and Site Execution workstreams run at simplified/basic depth within MVP; their full depth (shop-floor/QC/dispatch for Manufacturing; CPM scheduling and formal RFI/submittal/transmittal routing for Site Execution/Civil) is built out later as standalone verticals.
