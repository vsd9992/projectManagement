# Decision: Typed API contracts via utoipa + orval, with React Query as the frontend data-fetching library

**Decision**: The Axum backend generates an OpenAPI spec using `utoipa` (annotated handlers/structs), also serving interactive API docs (Swagger UI) from the running app. The React frontend consumes that spec via `orval`, which generates typed client functions **and** React Query (TanStack Query) hooks per endpoint — not just types. React Query is therefore locked in as the frontend's data-fetching/server-state library.

**Basis**: Direct choice between "types-only" (`openapi-typescript`, no fetching-library commitment) and "full client + hooks" (`orval`, commits to React Query) during the API-contract stack discussion. Full client generation was chosen.

**Why**: The domain has real breadth (~19 core entities, growing across the MVP roadmap's M1–M6 milestones), and hand-written fetch calls per endpoint are exactly the kind of repetitive, drift-prone boilerplate this project's traceability/correctness values argue against. Generating the full client plus React Query hooks removes that boilerplate entirely — a backend contract change breaks the generated client at build time rather than silently drifting. `utoipa` was the uncontested choice for the backend side of this, since it generates the spec directly from the Axum code (no separately-maintained OpenAPI file) and yields free API docs as a side benefit.

**Consequences/constraints**:
- React Query is now the frontend's server-state layer — any future frontend architecture decisions (caching strategy, optimistic updates, real-time updates) should build on it rather than introduce a competing pattern.
- The OpenAPI spec becomes a build artifact generated from Axum handlers; the frontend's generated client must be regenerated whenever the backend API surface changes (this should become part of the eventual dev workflow/CI, not a manual step someone forgets).
- Every Axum handler intended for frontend consumption needs `utoipa` annotations (`#[utoipa::path(...)]`, `#[derive(ToSchema)]`) — this is a discipline requirement from the first endpoint onward, not something to retrofit later.
