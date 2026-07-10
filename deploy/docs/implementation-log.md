# Implementation Log: deployment

## Entries

### 2026-07-10 — W1/DEP-P6 reverse proxy, TLS, and public access boundary

Agent:
Implementation Worker

Branch:
component/deployment

Prompt:
deploy/control/prompt.md

Report:
deploy/control/report.md

Commit(s):

- 63209c593096cefcec06e03721743d8bf84aa825 — Add secure Caddy reverse proxy example
- 4ac5c746acb8b6f579bb113d2ce1f6eccda6a54f — Document TLS and public access boundary
- cfbbca690c17fe2db2d6cfd111f4733dc9767cf3 — Add reverse proxy boundary to deployment contract
- 23052f6d15a23b551c5648540495e6fa9127824c — Link server compose to public access runbook
- 29b54a77c9f08fc8e9ff4b0ade63248ce804e512 — Record Caddy public access boundary decision

Summary:
Implemented DEP-P6 with one placeholder-only Caddy example and a public-access runbook. The topology terminates public TLS at Caddy and proxies to the Server over `127.0.0.1:8080`; blocks `/health` and `/ready` publicly while using loopback `/health` for active checks; preserves Server-owned bearer authentication and role checks; aligns the proxy request-body limit with the accepted `52,428,800`-byte Server/API contract; and documents DNS, certificate storage, firewall/port, validation, smoke-check, and secrecy boundaries. Updated the Deployment contract, decisions, and Server Compose runbook to describe the example honestly without claiming a running proxy, certificate provisioning, DNS automation, firewall mutation, or production readiness.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run clean-code review and external CI. Validate the Caddyfile with Caddy 2.10 or newer. A later authorized rollout must replace the placeholder hostname, configure DNS/firewall/certificate storage, and start/reload Caddy outside this connector-only phase. Sanitized proxy logging/retention and service-manager wiring remain deferred.

---

### 2026-07-10 — W1/DEP-P5 object-store, worktree, and host directory provisioning

Agent:
Implementation Worker

Branch:
component/deployment

Prompt:
deploy/control/prompt.md

Report:
deploy/control/report.md

Commit(s):

- 8649a2163fe9cf12303e231cfcdbe8848f6043d9 — Document host directory layout and permissions
- f5db306f80d8ad6a9546641a48a595d5409e9ab4 — Align local env placeholders with current deployment docs
- 43b945864c1df5cbf99a865502685dfe5b194a78 — Refresh deployment contract current surface
- 47e8fe8a6fdcd33f2a5fe319fc43ce7225400774 — Record host path separation decision
- 89b3a249f673ec54bea9560e76a7bbd960588ac0 — Link local compose to host directory guidance
- ad71cf0c425a4b43f0dfe60666ab286cdd256cc8 — Link server compose to host path guidance

Summary:
Implemented DEP-P5 as documentation and placeholder alignment only. Added `deploy/docs/host-directory-layout.md` with production-style object-store, worktree, config, secret, log, backup, and runtime-temp path classes; ownership and mode expectations; Server config-key mapping; container UID 10001 considerations; backup classification; never-commit rules; provisioning checklist; and read-only validation examples. Updated `.env.example` with the existing Compose HTTP host-port placeholder and corrected current Server/Compose comments while preserving relative local Server path defaults. Refreshed the deployment contract and decisions to reflect the current PostgreSQL-plus-Server local scaffold and the accepted path separation model. Updated local/server Compose runbooks to link the new host layout while explicitly keeping named volumes, disabled Worktree runtime, and no host bind mounts.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run clean-code review and external CI. Validate host ownership/mode checks on an actual target only in a later authorized operations phase. Future bind mounts must coordinate container UID/GID behavior and accepted Worktree authority/write semantics. Secret generation/rotation, log retention, backup automation, cleanup, and actual host provisioning remain deferred.

---

### 2026-07-09 — W1/DEP-P4 database migrations, backup, and restore runbook

Agent:
Implementation Worker

Branch:
component/deployment

Prompt:
deploy/control/prompt.md

Report:
deploy/control/report.md

Commit(s):

- 4b22b5f3de7a6d43b1cf73d60591c87addaf069d — Document migration backup restore runbook
- 1d443092db3d7896c4c1a62eea529e2354da22aa — Record manual migration procedure decision
- a6c1e28344f61d5ae32e52d110231c8894c921e2 — Link server runbook to migration backup restore procedure

Summary:
Implemented DEP-P4 as documentation-only Deployment runbook work. Added `deploy/docs/migrations-backup-restore.md` with the current migration execution owner, pre-migration stopped/quiesced requirements, local and production-style PostgreSQL backup command shapes, object-store backup coordination, manual SQLx migration command shape, post-migration health/readiness checks, local and production-style restore order, consistency warnings, and dry-run/checklist verification. Recorded the decision that migrations are currently operator-run manual SQLx commands until a later accepted CLI, Server entry point, or deploy script exists. Linked the server compose runbook to the migration/backup/restore procedure and preserved the rule that compose/server images do not auto-run migrations.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run documentation review and external validation where available. Validate command syntax in a shell/Docker environment before treating the runbook as operationally proven. Future phases should define host directory permissions, production backup path layout, reverse proxy/TLS, provider service rollout, and any accepted migration CLI/server/deploy-script entry point.

---

### 2026-07-09 — W1/DEP-P3 server packaging and service wiring

Agent:
Implementation Worker

Branch:
component/deployment

Prompt:
deploy/control/prompt.md

Report:
deploy/control/report.md

Commit(s):

- 902d3beb36609dc94ffacdd5fb6f31b07c6c4eae — Wire local server service in deployment compose
- 362ae0f57757b3b30c313bf05b8e7ddfa32cce8a — Add server container packaging scaffold
- 6eb4a3925037734cdefaf1fb51aa623e82ef41c4 — Document local server compose workflow
- 87d5f11a3b92affb1af398263bb527441d0c669c — Add server compose runbook

Summary:
Implemented DEP-P3 using Deployment-owned container packaging and local Compose server wiring after the Server startup/config/readiness dependency baseline was accepted. Added `deploy/server.Dockerfile` to build and run `haze-sync-server` without secrets or migration execution. Updated `deploy/docker-compose.yml` to start PostgreSQL plus the server with documented Server env vars, local-only host port binds, a named object-store volume, PostgreSQL health dependency, and `/health` liveness. Updated local compose docs and added `deploy/docs/server-compose.md` documenting packaging decision, environment, volumes, startup, `/health` versus `/ready`, shutdown, destructive local reset, and deferred production behavior.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run `docker compose -f deploy/docker-compose.yml config` and, if Docker is available, `docker compose -f deploy/docker-compose.yml up --build -d postgres server` followed by local `/health` and `/ready` smoke checks. Continue with clean-code review before treating DEP-P3 as merge-ready. Migration execution, host directory permissions, reverse proxy/TLS, Worktree runtime, GDrive service wiring, and production rollout remain deferred.

---

### 2026-07-06 — W1/DEP-P2 local development compose hardening

Agent:
Implementation Worker

Branch:
component/deployment

Prompt:
deploy/control/prompt.md

Report:
deploy/control/report.md

Commit(s):

- 3481399aa70287e72dd2fbe79a6d81d5b2a18a2f — Harden local deployment compose scaffold
- 0f1e15e6bbc4b1033c6111a5e41919b4526b1548 — Align local env placeholders with compose
- 895d5a5f7c8c9c79144c519649a90e173f5ae212 — Document local compose validation

Summary:
Hardened the local-only Docker Compose PostgreSQL scaffold with explicit non-production comments, fixed local bind semantics, configurable local host port, and a slightly stricter PostgreSQL healthcheck start period. Aligned `.env.example` with the compose variables and current server adapter-mode placeholder names without adding real credential material or production URLs. Added `deploy/docs/local-compose.md` with syntax validation, optional local PostgreSQL smoke commands, local-only boundaries, and explicit deferral of object-store/worktree bind mounts until Server/Worktree contracts are ready.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run `docker compose -f deploy/docker-compose.yml config` in an environment with Docker Compose available. Continue to DEP-P3 only after Server startup/config/readiness/migration contracts are accepted.

---

### 2026-07-05 — DEP-P1 component contract and planning normalization

Agent:
Architect

Branch:
component/deployment

Prompt:
User requested continuing component documentation and implementation-plan writing after CLI.

Report:
Conversation summary; no component control report was written because this is an Architect documentation/planning pass, not an implementation-worker execution.

Commit(s):
See branch history after this documentation pass.

Summary:
Replaced scaffold-level Deployment docs with a real component contract, dependency map, phased implementation plan, component decisions, and this planning baseline log entry. The pass documented `deploy` as the operations/runtime placement component for compose, service topology, secret placement, host paths, migration/backup/restore runbooks, reverse proxy/TLS guidance, bootstrap/rollback, and production readiness checks. Current deployment remains local PostgreSQL scaffold only and not production sync deployment.

Status:
ARCHITECT_ACCEPT_PENDING_REVIEW

Follow-ups:
Execute DEP-P2 through DEP-P9 through normal implementation -> clean-code -> CI -> fixer lifecycle when scheduled. Keep deployment artifacts secret-free, local-vs-production status explicit, and coordinate Server/Storage/GDrive/Worktree/CI contracts before adding service wiring or automation.

---

Use this format for future entries:

~~~text
### YYYY-MM-DD — <wave>/<phase>

Agent:
Branch:
Prompt:
Report:
Commit(s):
Summary:
Status:
Follow-ups:
~~~
