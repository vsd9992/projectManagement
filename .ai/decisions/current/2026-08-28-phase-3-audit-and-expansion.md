# Decision: Phase 3 scope — audit/test backfill plus four pulled-forward baseline pieces

**Decision**: Phase 3 (Testing & Bug Fixing) covers, per direct user approval during scoping:
1. A systematic audit of the implementation against `requirements.md`/`architecture.md`, fixing real bugs found.
2. Backfilling automated regression tests for the M1–M5 business logic that had none (closes `risks.md` risk #6).
3. A second billing method: progressive/RA-bill-style, alongside the existing milestone-based one.
4. A generalized Schedule Task + Dependency graph spanning all four workstreams, with planned/actual dates, forward date-shift propagation, and in-app notifications — replacing `site_task_dependency` entirely (not additive).
5. A PO internal approval step (pulled in after the audit surfaced its absence).
6. A tenant-level configuration API, narrowed to `region_profile` + `workstream_labels` — **not** a generic configurable-approval-chain engine, which would be a comparably-sized separate undertaking.

**Basis**: Direct user request ("let's discuss Phase 3 scope"), refined through several rounds of `AskUserQuestion` clarification during planning — see the full implementation plan for the exact question/answer trail. Full design plan: `C:\Users\vikas.VISHNU\.claude\plans\lexical-hatching-dijkstra.md` at time of writing (a local Claude Code plan file, not part of this repo — see this decision record and the `roadmap.md` Phase 3 entries for the durable record instead).

**Why**: `architecture.md`/`workflows.md` already described the progressive billing method and the Schedule/Dependency graph as intended baseline — they were simply never built (explicitly deferred at each milestone that touched adjacent code, per `roadmap.md`'s own M3/M4/M5/M6 notes). Building them now closes real requirements/implementation mismatches rather than adding net-new product scope. The PO approval step and tenant-config API were genuinely new findings from the audit pass, pulled in by explicit user choice rather than assumed.

## Sub-decisions (money-math and architecture choices confirmed with the user)

- **RA-bill cumulative math**: each progressive invoice's `certified_value_to_date` is Finance's re-entered running total. This bill's taxable amount = current certified value minus the **maximum** `certified_value_to_date` of any prior progressive invoice on the project (not a sum — each row already stores a cumulative figure, matching how real RA bills restate the running total each time).
- **Schedule graph fully replaces `site_task_dependency`** (not additive): every `site_task` gets an auto-created, permanently-linked `schedule_task` row; the old dependency table/endpoints are removed. Production tasks, purchase orders, and design revisions do **not** get auto-created schedule tasks — a PM can optionally link a standalone schedule task via a nullable FK. This asymmetry was a deliberate scoping choice (only `site_task_dependency` was named for replacement), flagged to the user rather than silently generalized further.
- **Date-shift propagation** is a basic conditional forward-pass (a dependent absorbs a delta if it has slack, and does not propagate further through it) — explicitly **not** full CPM/critical-path scheduling, which `roadmap.md` defers to a standalone future Civil vertical.
- **Notification delivery**: in-app only (a DB record + `GET /notifications`), no email/SMS — no such integration exists in this app yet, and building one wasn't asked for.
- **Notification audience**: internal team only (everyone with a role in the project's business unit, plus the tenant admin) — not the client. Matches how the Client Portal is otherwise a curated read/approve surface, never raw internal schedule data.
- **Notification trigger**: exactly the set of tasks the date-shift propagation actually shifted, filtered to tasks not yet started (`status != 'done' AND actual_start_date IS NULL`) — avoids noise on work already underway or finished.

## Consequences/constraints

- `require_any_business_unit_role` (new in `api::authz`) is weaker than the project-scoped checks elsewhere — it confirms role membership *somewhere* in the tenant, not in one specific business unit, because vendors and clients have no `business_unit_id` of their own. A known limitation, not a bug, given the entities' current shape.
- Configurable approval chains remain unbuilt after this pass — flagged as a fresh, explicit gap (not silently dropped) for a future deliberate decision, since building even a narrow version now would risk exactly the over-engineering `risks.md` risk #2 warns against.
- This decision record is added to incrementally as each stage of the plan lands, rather than written once at the end — later edits append verification detail per stage, following the same discipline used throughout M1–M6.
