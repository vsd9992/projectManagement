# Decision: Build as a multi-tenant SaaS product, not a single-company internal tool

**Decision**: The system is architected from the start as a multi-tenant SaaS product, even if the first real usage is the founder's own operations.

**Basis**: Explicit product-scope decision made during system-design discussion, choosing "Multi-tenant SaaS product" over "internal tool" and "internal first, SaaS later."

**Why**: The intent is to eventually sell this to other furniture/interior/civil firms. Retrofitting multi-tenancy onto a single-tenant internal tool later is a substantial architectural rewrite (tenant isolation, subscription/billing, admin console); building it in from the start avoids that cost.

**Consequences/constraints**: Every core entity (Project, User, Client, etc.) must carry tenant scoping from day one. Tenant/Business Unit hierarchy (see `2026-08-27-mvp-scope-turnkey-interiors-first.md` and `architecture.md`) is foundational, not an add-on. Data isolation strategy is still an open architecture decision (see `.ai/project/risks.md`), to be settled during the stack discussion.
