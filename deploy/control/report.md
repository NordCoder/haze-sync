REPORT_TYPE:
FIX

STATUS:
FIX_COMPLETE

AGENT:
role: fixer-worker
agent_execution_id: W1-FIX-DEP-P5A-DOCUMENTATION-ALIGNMENT-20260715
chat_name: deployment — W1 DEP-P5A Documentation Alignment Fix

COMPONENT:
name: deployment
path: deploy
branch: component/deployment
control_prompt_path: deploy/control/prompt.md
control_report_path: deploy/control/report.md

WAVE:
id: W1
phase_id: FIX-DEP-P5A-DOCUMENTATION-ALIGNMENT

SUMMARY:
Removed the remaining historical documentation contradictions after DEP-P5A. The host-directory runbook now distinguishes the safe base Compose no-bind/disabled default from the accepted opt-in Worktree override, records the exact container target /var/lib/haze-sync/worktree, preserves UID 10001 and non-world-writable permission requirements, and keeps read-only mount and write-capable mode as independent explicit operator gates. The migration/backup/restore runbook now treats active Worktree writers as part of the stop/quiesce boundary and includes Worktree data in the coordinated PostgreSQL/object-store/Worktree recovery set whenever it may contain authoritative or unreplicated content. Migration ownership remains human/operator manual SQLx execution; Storage owns schema contents; Deployment owns sequencing. No Compose behavior, env defaults, product files, migrations, workflows, startup hooks, services, secrets, host mutations, or destructive automation changed.

CHANGED_FILES:
- deploy/docs/host-directory-layout.md
- deploy/docs/migrations-backup-restore.md
- deploy/control/report.md

CONTRADICTIONS_REMOVED:
- Removed statements that a Worktree Compose bind is wholly future or forbidden.
- Scoped no-bind language to the base Compose file.
- Documented the accepted opt-in override and exact target /var/lib/haze-sync/worktree.
- Documented read-only default and explicit independent gates for writable access and write-capable mode.
- Replaced future-only Worktree writer language with current conditional writer/quiescence guidance.
- Added Worktree data to the same coordinated recovery window when authoritative or unreplicated content may exist.
- Removed the old claim that this runbook does not enable or account for Worktree runtime in any form; automatic enablement remains explicitly forbidden.

POLICY_PRESERVATION:
- migration_execution_owner: human/operator manual SQLx
- schema_owner: Storage
- operational_sequence_owner: Deployment
- startup_migrations: forbidden
- writer_quiescence_before_backup_migration_restore: required
- rollback: operator-approved coordinated restore only
- automatic_down_reset_drop_delete_cleanup_fallback: forbidden
- base_compose_worktree_bind: absent
- opt_in_override_default_bind: read-only
- opt_in_override_default_mode: disabled
- container_worktree_target: /var/lib/haze-sync/worktree
- container_uid: 10001

VALIDATION:
- Read active state and focused fixer prompt.
- Verified host-directory scope states base no-bind/disabled and opt-in read-only override.
- Verified host-directory Worktree section uses exact target /var/lib/haze-sync/worktree and preserves UID 10001, path separation, backup, and explicit write gate requirements.
- Verified migration runbook lists Server, Worktree runtime, adapters, CLI write commands, and client/plugin writes as writers requiring stop or explicit quiescence.
- Verified coordinated recovery set includes PostgreSQL, object store, and Worktree when required by authority/replication state.
- Verified manual SQLx ownership and no startup migration/automatic rollback rules remain explicit.
- Verified no real secret, credential, dump, archive, vault content, or machine-specific real host path was introduced.
- Component CI run 29406615806, run number 1961, completed successfully on exact documentation-bearing SHA d14b5f04177eae13d03b386b6c73da66921835e3.
- cargo fmt, cargo check, cargo test, cargo clippy, and diagnostics finalization completed successfully.

CHECKS_NOT_RUN:
- Docker Compose rendering and Caddy validation were not run because the connector-only environment provides no shell/Docker/Caddy channel and protected behavior was not changed.
- Target-host permission, backup, migration, restore, and Worktree runtime smoke checks were not run because no authorized host or operator environment was in scope.

CODE_BEARING_SHA:
d14b5f04177eae13d03b386b6c73da66921835e3

CI:
workflow: Component CI
run_id: 29406615806
run_number: 1961
conclusion: success
exact_sha: d14b5f04177eae13d03b386b6c73da66921835e3

SAFETY:
secrets_committed: no
real_host_mutation: no
product_changes: no
compose_behavior_changes: no
env_key_or_default_changes: no
migration_or_workflow_changes: no
automatic_write_or_bidirectional_rollout: no
destructive_automation: no

BLOCKERS:
- none for the focused documentation alignment fix

NEXT_RECOMMENDED_AGENT:
focused DEP-P5A functional/security reviewer

FINAL_VERDICT:
FIX_COMPLETE. The two required Deployment runbooks now agree with the accepted opt-in Worktree architecture and migration/recovery ownership policy. Exact documentation-bearing CI is green. This report does not claim CLEAN_ACCEPT or merge readiness.

PUSHED:
yes
