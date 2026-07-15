REPORT_TYPE:
ARCHITECTURE_REVIEW

STATUS:
ARCHITECT_ACCEPT

AGENT:
role: architect-reviewer
chat_name: deployment — W1 DEP-P5A Migration Ownership Review

COMPONENT:
name: deployment
path: deploy
branch: component/deployment
pr: 52

WAVE:
id: W1
phase_id: DEP-P5A-MIGRATION-OWNERSHIP-REVIEW

SUMMARY:
The existing Deployment migration, backup, restore, startup, and secrecy policy is explicit enough to authorize DEP-P5A implementation without inventing automatic migrations, a new migration runner, direct Deployment database ownership, hidden startup side effects, destructive rollback, or a second migration execution owner.

BASELINE_AND_PR:
exact_main_ancestor: c1e69a664388b0cba028170e8398b9088218957d
deployment_post_sync_sha: 54e0b8b84e06e7475dc99ea25b22ddd248bb98c2
pre_sync_report_blob: ad4156e26fae85bfa1049e71ea9738640b92a715
review_slot_head_observed: b3501c8e22dd0fc78e178c7144770cefc2d830cd
component_ci_run_id: 29400618638
component_ci_run_number: 1954
component_ci_conclusion: success
pr_state_observed: open
pr_draft_observed: true
pr_merged_observed: false
merge_readiness_assessed: false

POLICY_DECISION:
migration_execution_owner: human/operator running the documented manual SQLx command
schema_owner: Storage
operational_sequence_owner: Deployment
writer_quiescence_rule: all writers stopped or explicitly quiesced before coordinated backup, migration and restore
backup_consistency_rule: PostgreSQL metadata and object-store content must come from one coordinated recovery window
startup_migration_policy: no implicit migration in Server, Compose or Docker startup
rollback_rule: explicit operator-approved coordinated restore; no automatic down/reset/drop/destructive fallback
secret_handling_rule: credentials, URLs, dumps and archives remain operator-local, untracked and absent from public reports
dep_p5a_authorized: yes

REVIEW_FINDINGS:
1_migration_execution_owner:
- Exactly one execution owner is defined: the human/operator invoking the documented manual `sqlx migrate run --source migrations` command.
- Storage does not execute production migrations.
- Server exposes no accepted automatic startup migration behavior.
- Deployment owns the documented procedure, not a database runtime or schema authority.

2_schema_ownership:
- Storage owns schema design, migration contents, ordering, compatibility requirements, and migration-source correctness.
- Storage does not own production execution timing, writer shutdown, backups, restore approval, or rollout authorization.

3_operational_sequence_ownership:
- Deployment owns the operator procedure:
  stop or explicitly quiesce writers
  -> capture coordinated PostgreSQL and object-store recovery artifacts
  -> run the manual migration command
  -> verify migration command success
  -> start services
  -> verify process health and dependency readiness
  -> obtain operator approval before further rollout
- Deployment must not edit migration contents or Server internals during DEP-P5A.

4_writer_quiescence:
- Server, hosted Worktree runtime, GDrive adapter, Obsidian/plugin write paths, and future writer services are all writers for this policy.
- They must be stopped or explicitly quiesced before backup, migration, or restore and remain so for the coordinated recovery window.
- `/health` and `/ready` are not evidence that writes are stopped.

5_backup_consistency:
- A complete recovery point consists of PostgreSQL metadata, object-store content, repository revision, migration state, timestamp, and recorded writer state from one coordinated operational window.
- Database-only and object-store-only artifacts may be partial diagnostic backups but must not be described as a complete recovery point.
- Mismatched PostgreSQL and object-store windows are forbidden unless an explicit incident plan documents and approves the exception.

6_startup_behavior:
- The current Server Compose service has no migration command or migration runner.
- The current Docker image entrypoint starts only `haze-sync-server` and explicitly states that the image does not run migrations.
- Server, Compose, and Docker startup must remain migration-free unless a later explicit architecture decision changes ownership.
- Starting a process without applying required migrations must not be represented as successful rollout.

7_verification_semantics:
- Migration command success, process health, dependency readiness, application rollout success, and operator approval are distinct gates.
- `/health` proves only that the HTTP process/router responds.
- `/ready` reports sanitized dependency readiness.
- Neither proves backup integrity, provider synchronization, all write paths, migration compatibility beyond the executed verification, or production readiness.

8_rollback_policy:
- Rollback means an explicit operator-approved restore of the coordinated PostgreSQL/object-store backup set under a reviewed recovery plan.
- Automatic down migrations, database reset, database drop, destructive cleanup, and automatic fallback to mismatched artifacts are forbidden.
- Destructive restore flags are allowed only for an explicitly approved empty or replaceable target and do not authorize automatic rollback.

9_secrets_and_artifacts:
- Tracked files and public reports must not contain `DATABASE_URL`, database passwords, bearer tokens, OAuth tokens, token hashes, database dumps, object-store archives, production `.env` files, absolute secret paths, provider payloads, raw vault content, or TLS private keys.
- Credentials and backup artifacts remain operator-local, access-controlled, and untracked.

10_dep_p5a_boundary:
- DEP-P5A may consume the accepted policy for Worktree runtime/config fan-in.
- DEP-P5A must not add automatic migrations, a new migration runner, direct database ownership, hidden startup side effects, destructive rollback, or a second execution owner.
- Deployment may document/configure accepted Server and Worktree runtime surfaces only within Deployment ownership.

DOCUMENTS_READ:
- deploy/docs/component-contract.md
- deploy/docs/migrations-backup-restore.md
- deploy/docs/implementation-plan.md
- deploy/docs/dependency-map.md
- deploy/control/state.md
- deploy/control/prompt.md
- deploy/docker-compose.yml
- deploy/server.Dockerfile

AMBIGUITIES:
blocking_ambiguities_found: none
policy_change_required: no
notes:
- `deploy/docs/implementation-plan.md` contains older current-state wording that understates the already-present Server Compose/package surface. This is documentation drift, but it does not create a migration-ownership ambiguity because the component contract, runbook, control prompt, Compose file, and Dockerfile agree on the operative policy.
- A future documentation-drift pass may update that current-state section without changing the accepted ownership policy.

IMPLEMENTATION_WORKER_BOUNDARIES:
- Preserve the human/operator as the sole migration execution owner.
- Preserve Storage ownership of schema and migration contents.
- Preserve Deployment ownership of operational sequencing and runbooks only.
- Do not modify Server, Storage, Worktree, API, CLI, or provider implementation.
- Do not add migrations to Server, Compose, Docker entrypoints, healthchecks, or startup hooks.
- Do not add a deploy migration runner or hidden database side effect.
- Keep Worktree and other writer services disabled/stopped until the documented migration and rollout gates are satisfied.
- Keep local Compose explicitly local-only and do not present syntax, health, or readiness checks as production readiness.
- Keep secrets, URLs, dumps, archives, and production paths out of tracked files and public reports.
- Do not implement automatic rollback, down migration, reset, drop, cleanup, or mismatched-artifact fallback.

CAN_DEP_P5A_BEGIN:
yes, through a new explicitly scoped implementation-worker control slot after Orchestrator archives this completed review slot

BLOCKERS:
none for migration ownership policy

NEXT_RECOMMENDED_AGENT:
orchestrator

FINAL_VERDICT:
ARCHITECT_ACCEPT. The migration ownership and coordinated recovery policy is sufficiently explicit for DEP-P5A. This review does not implement DEP-P5A, alter runtime/configuration, or establish PR merge readiness.

PUSHED:
yes
