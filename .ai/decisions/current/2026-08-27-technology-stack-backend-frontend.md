# Decision: Technology stack — Rust/Axum/SeaORM+SQLx/PostgreSQL backend, React/TypeScript frontend

**Decision**:
- **Backend**: Rust for both development and production (no prototype-in-another-language-then-rewrite). Web framework: **Axum**. Database access: **SeaORM** as the primary entity/CRUD layer for the core domain entities (with its own migration tooling), dropping to raw **SQLx** (compile-time-checked SQL) for correctness-critical or complex logic — billing/RA-bill calculations, audit-log queries, WBS dependency-graph recursive CTEs. Database: **PostgreSQL**.
- **Frontend**: **React + TypeScript**, locked with no open questions. **Node.js** is used only as the frontend build/dev tooling runtime (e.g. Vite) — not as a separate server-side layer (no BFF, no SSR framework) unless a future decision explicitly changes this.
- **Dropped from consideration**: SurrealDB (originally proposed for unspecified "lightweight things"). Rejected because running a second database breaks cross-store transactional consistency, which conflicts with the system's top priority (full transparency/traceability — see `.ai/project/requirements.md`), and no concrete use case was identified that PostgreSQL (JSONB, `pg_trgm`, recursive CTEs) couldn't already cover.

**Basis**: Stack discussion following the approved system-design baseline (see `.ai/project/project-plan.md`). Backend language/DB and frontend framework proposed by the product owner; Axum/SeaORM/SQLx combination recommended and agreed after weighing tradeoffs (Rust's compile-time rigor vs. iteration speed; SeaORM's CRUD velocity vs. SQLx's query precision).

**Why**: Rust's type system directly supports the project's top priority — full traceability and correctness in the audit trail and billing engine. Axum is the mature, `tower`-based mainstream Rust web framework, which lets tenant-context, auth, and logging compose as middleware. SeaORM covers the ~19-entity CRUD-heavy domain (Tenant, Project, WBS, BOQ, Vendor, Invoice, etc. — see `.ai/project/architecture.md`) without hand-writing SQL for every entity; SQLx is reserved for the specific places where ORM abstraction would work against precision (money calculations, audit queries, graph traversal).

**Consequences/constraints**:
- Migrations are managed via `sea-orm-migration`.
- Typed API contracts for the React frontend still need a mechanism — leaning toward `utoipa` (OpenAPI generation from Axum handlers) so the frontend can generate a typed client, but this is not yet decided (tracked as a remaining stack item).
- Multi-tenant data isolation mechanism (Postgres row-level security via `SET LOCAL` session variables vs. schema-per-tenant) is still open — see `.ai/project/risks.md` risk #4. This is the next stack item to resolve, since it affects both SeaORM/SQLx query patterns and the Axum middleware design.
- `AGENTS.md`'s canonical-commands section remains a placeholder until the repository is actually scaffolded with this stack.
