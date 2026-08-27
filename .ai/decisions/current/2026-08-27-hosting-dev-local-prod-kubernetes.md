# Decision: Local dev server for development, Kubernetes for production (provider TBD)

**Decision**:
- **Development**: a local Debian-based dev server on the LAN (reachable at 192.168.1.4 via SSH). Credentials are supplied out-of-band via local files on the operator's machine and must never be committed to this repository, printed in output/logs, or written into any `.ai/` document.
- **Production**: Kubernetes, on either **Linode** or **E2E Networks** — final provider not yet chosen. This does not block MVP development, which runs against the local dev server; it needs to be resolved before an actual production deployment, not before Execution starts.

**Basis**: Final item of the technology-stack discussion, following the locked backend/frontend/tenant-isolation/auth/API-contract decisions.

**Why**: Using an existing local dev server avoids provisioning cloud infrastructure before there's anything to deploy. Kubernetes as the production target was specified directly; Linode and E2E Networks are both credible candidates — E2E is India-based (relevant given the India-first billing/regional profile already locked in `.ai/decisions/current/2026-08-27-generic-billing-engine-india-first-profile.md`), while Linode (Akamai) also has a Mumbai region. That tradeoff is real but not urgent, so it's left open rather than forced now.

**Consequences/constraints**:
- The Rust backend must be containerized (Docker image) for the Kubernetes production target — this affects the eventual CI/build setup, not the application code itself.
- Whichever managed Postgres or in-cluster Postgres setup is chosen for production must support **transaction-level connection pooling** (not statement-level), since the RLS design (`.ai/decisions/current/2026-08-27-tenant-isolation-shared-schema-rls.md`) relies on `SET LOCAL` being scoped per transaction. This must be verified explicitly when the production DB hosting is finalized.
- No SSH credentials, IPs beyond the dev server's LAN address, kubeconfig files, or cloud provider API keys are to be stored in this repository at any point — they belong in a local, untracked secrets mechanism when the time comes (e.g. a `.env` file covered by `.gitignore`, or a secrets manager), never committed.
- Detailed Kubernetes deployment architecture (manifests/Helm charts, ingress, DB hosting within/outside the cluster) is deferred until closer to actual deployment — not needed to unblock Execution.
