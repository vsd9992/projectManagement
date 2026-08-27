# Decision: Session-based auth, single global login, unified auth surface for internal users and the Client Portal

**Decision**:
- **Mechanism**: server-side sessions, not JWT. An opaque session token is delivered via an httpOnly, Secure cookie; sessions are stored in a table in the same PostgreSQL database (no Redis at this stage).
- **Password hashing**: Argon2 (`argon2` crate).
- **Request flow**: Axum middleware resolves the session cookie → user → tenant_id/role (or, for Client Portal users, project scope) → issues `SET LOCAL app.tenant_id = ...` for that request's transaction, consistent with the RLS design in `.ai/decisions/current/2026-08-27-tenant-isolation-shared-schema-rls.md`. Role/business-unit permission checks are a separate authorization layer on top of RLS, not a replacement for it.
- **Login model**: a single global login page/endpoint; the user's tenant is resolved from their account, not from a subdomain. No wildcard DNS/TLS/per-tenant routing needed for MVP.
- **Client Portal**: authenticates through the same session mechanism as internal users, not a separate auth surface. A client user's session resolves to project-scoped access instead of a business-unit role.

**Basis**: Stack discussion following the locked tenant-isolation strategy. Session-vs-JWT and password hashing were a direct recommendation, accepted; login model and Client Portal auth surface were explicit either/or choices, both resolved in favor of the simpler option.

**Why**:
- Revocation matters more here than JWT's statelessness benefit: this is B2B SaaS handling financial/project data, and instantly killing a departing employee's or a client's access (a `DELETE` on the sessions table) is a real operational need. JWT revocation before expiry requires a blocklist, which reintroduces the same state a session store has anyway.
- There is one Axum API, not a distributed set of independently-verifying services, so JWT's main architectural advantage doesn't apply here.
- An httpOnly cookie is not readable by JavaScript, closing off the XSS-token-theft risk that comes with storing a JWT in `localStorage`.
- A single global login and a unified auth surface for internal + Client Portal users avoid building and securing two auth code paths for a portal that is read/approve-only in MVP; both can be revisited later (subdomain branding, a separate portal domain) without a rearchitecture, since tenant/project scoping is already resolved from the session, not from the URL.

**Consequences/constraints**:
- A `sessions` table (or equivalent) is part of the base schema; session lookups happen on every authenticated request before any tenant-scoped query.
- Session store must be indexed for fast lookup by token and support expiry/cleanup.
- The Client Portal's authorization layer must distinguish "project-scoped access" from "business-unit role access" at the permission-check layer, even though both flow through the same session mechanism.
- If a future need arises for third-party/API access or a mobile app, that will likely need a separate token-based flow (not necessarily JWT) layered alongside sessions, not a replacement for this decision.
