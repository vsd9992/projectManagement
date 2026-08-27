# Decision: Generic billing/tax engine, with India as the first configured regional profile

**Decision**: Billing and statutory tax logic are abstracted behind a pluggable Region Rule Profile. India (GST works-contract SAC codes, TDS, RA billing with mobilization-advance recovery, retention) is implemented as the first concrete profile, not hardcoded into the core engine.

**Basis**: Explicit answer to the localization question — chose "Generic, India as a config" over "India-specific, core" and "Not India-specific."

**Why**: The market research shows localized statutory logic (India's RA billing/GST/TDS) is a genuine competitive moat, but also that it's the exact kind of hardcoded assumption that makes Western software expensive to adapt for other regions. Since this is a multi-tenant SaaS meant to expand beyond one region eventually, the tax/billing engine must be generic even though only India is built out now.

**Consequences/constraints**: The billing engine design must support milestone, progressive (RA-bill-style), and lump-sum methods as configurable strategies, with region-specific tax computation as a swappable component. See `.ai/project/architecture.md` § Cross-Cutting Principles. Risk: if the India profile's specifics aren't modeled precisely now, the abstraction won't actually save rework later — see `.ai/project/risks.md` risk #3.
