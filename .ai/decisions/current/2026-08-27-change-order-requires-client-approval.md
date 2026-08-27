# Decision: Change Orders require formal client approval before they are binding

**Decision**: A scope change (Change Order/Variation) is not binding on approval by the internal Project Manager alone — the client must formally approve it before it takes effect and re-baselines cost/schedule.

**Basis**: Direct answer given when resolving the plan's open questions: "client need to formally approve scope change before it's binding" (chosen over a PM-only-sign-off-with-client-notified alternative).

**Why**: Scope changes affect cost and schedule commitments the client is ultimately paying for; a PM-only approval creates dispute risk and undermines the full-transparency/traceability goal that is the top product priority. Formal client approval keeps the audit trail defensible (mirrors the "legally defensible schedule" value seen with CPM-based tools in the market research).

**Consequences/constraints**: The Approval Workflow entity must support a client-approval step specifically for Change Orders (see `.ai/project/workflows.md` § Change Order / Scope Change Flow and § Approval Workflows). A Change Order's effects (BOQ/schedule re-baseline) must not be applied until that approval is recorded. Exact multi-level internal approval thresholds (if any) below the client step are not yet defined.
