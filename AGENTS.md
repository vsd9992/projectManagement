# AGENTS.md — Operating Rules

## Project
Multi-tenant Project Management SaaS for furniture manufacturers, turnkey interior fit-outs, and civil/architectural projects. See `.ai/00-project-index.md` for current state.

## Context loading
- Read `.ai/00-project-index.md` when orienting or restoring project context.
- Start with the minimum context needed for the current task; escalate only when uncertainty requires it.
- Do not reread unchanged files already reliably read since the last context compaction unless they may have changed.
- After context compaction or loss of conversational context, re-read this file and `.ai/00-project-index.md` before continuing.

## Working rules
- Work one task at a time. Do not re-plan the whole project unless required by the current phase or explicitly requested.
- Do not change scope, requirements, product behaviour, or architecture without approval.
- Do not refactor unrelated code.
- Verify file-backed facts before claiming them as current truth; inspect relevant source/config/schema before claiming how the implementation currently works.
- Never claim a test/build/check passed unless it was actually run successfully.
- Keep repository searches, logs, test output, diffs, directory listings, and command output bounded. Avoid generated files, dependency/vendor directories, build output, binaries, lockfiles, and large logs unless directly relevant.
- Protect secrets, credentials, API keys, production data, deployment configuration, DNS, SSL, payments, account/session/token files, and similar sensitive resources unless explicitly authorized.
- Surface discrepancies between documented intent and actual implementation rather than silently changing either side.
- Maintain project documentation per the lifecycle rules below. Preserve durable AI memory only when losing it would cause meaningful rework or repeated mistakes.

## Two truth models
For **what the system should do** (intended truth), in order: (1) current explicit instruction, (2) approved baseline in `.ai/project/`, (3) current records in `.ai/decisions/current/`.
For **what the system currently does** (implementation truth), in order: (1) current source/config/schema, (2) tests, (3) observed runtime behaviour.
If intended and implementation truth disagree, treat it as a DEVIATION — surface it, don't silently resolve it in either direction.

## Lifecycle phases
1. **Planning & Evaluation** — brainstorm, evaluate feasibility, define scope/requirements/architecture/risks/roadmap, produce an approved baseline. (Repo discovery first for existing-project work.)
2. **Execution** — follow the approved baseline, one task at a time. Update baseline docs only when an approved change makes them incorrect — not just because implementation happened.
3. **Testing & Bug Fixing** — validate against requirements/architecture/verification criteria; fix bugs without changing the baseline unless testing reveals the baseline itself needs an approved change.

## Documents (see `.ai/00-project-index.md` for what currently exists)
LIVE (change freely during normal work): `.ai/00-project-index.md`, roadmap status, active tasks, risks.
BASELINE (change only on an approved decision; update the `modified:` date on edit): `project-plan.md`, `requirements.md`, `architecture.md`, `workflows.md`, current decisions.

- Decisions: `.ai/decisions/current/YYYY-MM-DD-description.md` for decisions costly to forget or reverse. Move to `.ai/decisions/archive/` when superseded, adding a one-line historical pointer to the index if the decision has lasting value.
- Tasks: `.ai/tasks/active/YYYY-MM-DD-description.md` only when losing task reasoning would cause meaningful rework (spans sessions, survives a context compaction, a second approach fails, a non-obvious discovery emerges, an unresolved blocker appears). Move to `.ai/tasks/archive/` when done.
- Verification: task-level verification lives in the task file; milestone-level lives in `roadmap.md`; phase/release-level gets its own file in `.ai/verification/` only when it has genuine lasting value. Never mark something verified without actual supporting checks.

## Canonical commands
Backend lives in `backend/` (Cargo workspace: `migration`, `entity`, `api`). Postgres is not available locally in this dev environment — build/compile checks run locally, but running the server, migrations, or tests requires the dev server (`devMachine` over SSH; see `.ai/decisions/current/2026-08-27-hosting-dev-local-prod-kubernetes.md`).

- Build (works locally): `cd backend && cargo build --workspace`
- Run migrations (on devMachine, `DATABASE_URL` = the `app_migrator` superuser connection string from `.env`): `cargo run --bin migration -- up`
- Run the server (on devMachine): `cargo run --bin api` (reads `DATABASE_URL_APP`/`DATABASE_URL_ADMIN`/`BIND_ADDR` from `.env`)
- Run tests (on devMachine, against the dedicated `project_management_test` database — never the dev fixture data in `project_management`): `TEST_DATABASE_URL_APP=... TEST_DATABASE_URL_ADMIN=... cargo test -p api`
- After any migration change: re-run `GRANT ALL ON ALL TABLES/SEQUENCES IN SCHEMA public TO app_user, app_admin;` (see `backend/scripts/setup_roles.sql`) — new tables aren't visible to the app roles until granted. Run this against **each** database individually (`psql -d <dbname> -c '...' -c '...'`) — passing multiple `-d` flags in one `psql` invocation does not run commands against multiple databases; the last `-d` silently wins for the whole invocation.
- Bootstrap the first platform admin (on devMachine; no HTTP signup exists for this tier deliberately): `PLATFORM_ADMIN_PASSWORD=... cargo run --bin create_platform_admin -- <email>`.
- After `tar`/`scp`-deploying edited source to devMachine, a rebuild can silently no-op: `tar` preserves each file's original local mtime, which can end up older than the mtime Cargo's fingerprint recorded from a *previous* build on devMachine, so Cargo sees the source as "unchanged" and skips recompiling it even though the content differs — `cargo build` then exits 0 having rebuilt nothing real. If a fix doesn't show up in live behavior after a redeploy, don't trust the build's exit code — `touch` the edited file(s) on devMachine before rebuilding, and confirm `stat -c '%y' target/debug/api api/src/routes/<file>.rs` shows the binary genuinely newer than the source before restarting.

No frontend scaffold yet.
