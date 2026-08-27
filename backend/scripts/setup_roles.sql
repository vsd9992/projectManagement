-- One-time, per-environment setup for the two DB roles the RLS design
-- requires (see .ai/decisions/current/2026-08-27-tenant-isolation-shared-schema-rls.md).
-- Run manually as a Postgres superuser. Not part of versioned migrations,
-- since it involves role/password management that shouldn't live in
-- migration history. Replace the placeholder passwords before running, and
-- set the matching values in your local .env (never commit real passwords).

CREATE ROLE app_user WITH LOGIN PASSWORD 'CHANGE_ME_APP_USER' NOBYPASSRLS;
CREATE ROLE app_admin WITH LOGIN PASSWORD 'CHANGE_ME_APP_ADMIN' BYPASSRLS;

-- Adjust the database name if it differs from project_management.
GRANT ALL PRIVILEGES ON DATABASE project_management TO app_user;
GRANT ALL PRIVILEGES ON DATABASE project_management TO app_admin;

-- Run once connected to the target database (\c project_management) so these
-- apply to the right schema:
GRANT ALL ON ALL TABLES IN SCHEMA public TO app_user;
GRANT ALL ON ALL TABLES IN SCHEMA public TO app_admin;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO app_user;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO app_admin;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO app_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO app_admin;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO app_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO app_admin;
