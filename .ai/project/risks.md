modified: 2026-08-27

# Risks

Active cross-cutting risks only. Remove or move context to a decision record once a risk is resolved or accepted.

1. **Breadth-vs-depth risk**: three verticals (furniture, interiors, civil) is a lot of ground. Mitigated by decision to build Turnkey Interiors to full depth first and defer the other two — but discipline is needed to not let MVP scope creep back toward "a bit of everything."
2. **Workstream abstraction over-engineering**: the composable Workstream model (`architecture.md`) is designed for long-term flexibility across three verticals, but MVP only needs it to flex across four workstream types within one vertical. Risk of building more configurability than MVP actually exercises. Build only what Turnkey Interiors needs; verify the abstraction holds when Furniture/Civil verticals are added later, don't pre-build for them now.
3. **Generic billing/tax engine vs. India-specific correctness**: abstracting billing behind a Region Rule Profile is right for the product's future, but if the India profile's specifics (GST works-contract SAC codes, TDS, mobilization-advance recovery, retention) aren't modeled precisely, the abstraction won't save the rework. Needs careful design once the stack/schema work begins, not just a placeholder interface.
4. **Traceability retrofit cost**: audit logging and document versioning are meant to be core platform services from the first schema (`architecture.md`). If early implementation treats them as an afterthought, retrofitting full traceability later is expensive and may leave gaps in historical data.
5. **"Intuitive and agile" UX ambition vs. entity breadth**: the domain model already has ~15 core entities before any UI exists. Real risk of ending up as complex as the enterprise ERPs this product is meant to differentiate against. UI/UX design must actively guard against this — flagged here so it isn't lost by the time UI work starts.
