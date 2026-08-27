# Decision: MVP builds Turnkey Interiors to full depth first; Furniture Manufacturing and Civil/Architectural are deferred standalone verticals

**Decision**: Given three project verticals and a multi-tenant SaaS ambition, the first build slice is Turnkey Interiors taken to full depth (Lead → Quotation/BOQ → Design → Client Approval → Procurement → Site Execution → Installation → Handover → Billing → Client Portal), not a shallow pass across all three verticals.

**Basis**: Explicit answer to "which should the first build slice prioritize?" — chose "One project type, full depth" over "All three types, shallow" and "Not sure yet." Turnkey Interiors was then chosen as that vertical because it naturally touches Design, light Manufacturing, Procurement, and Site Execution — the best single vehicle to validate the shared composable core (see `2026-08-27-composable-workstream-project-model.md`).

**Why**: Building all three verticals shallowly risks a system that does nothing well and never gets real-world validation of the core data model. Building one vertical deep first proves the composable Workstream model against the hardest real case (a project that spans multiple workstreams at once) before extending to simpler, more self-contained verticals.

**Consequences/constraints**: Full-depth Manufacturing (shop-floor/QC/dispatch/EAM) and full-depth Civil/Architectural (CPM scheduling, formal RFI/submittal/transmittal routing, BIM) are explicitly out of MVP scope — see `.ai/project/roadmap.md` "After MVP." Within Turnkey Interiors MVP, the Manufacturing and Site Execution workstreams are implemented at simplified/basic depth only (see `.ai/project/requirements.md`).
