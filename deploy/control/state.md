# Control State

component: deployment
branch: component/deployment
status: PROMPT_READY
active_prompt: deploy/control/prompt.md
active_report: deploy/control/report.md
active_agent_role: architect-reviewer
assigned_chat_name: deployment — W1 DEP-P5A Migration Ownership Review

wave: W1
phase: DEP-P5A-MIGRATION-OWNERSHIP-REVIEW
implementation_status: SYNC_ACCEPTED
clean_review_status: CLEAN_ACCEPT_EXISTING_DEPLOYMENT
ci_status: CI_GREEN_POST_SYNC
architect_status: NOT_STARTED_POLICY_REVIEW
known_failed_checks: []

synchronized_baseline:
- exact_main_sha: c1e69a664388b0cba028170e8398b9088218957d
- post_sync_sha: 54e0b8b84e06e7475dc99ea25b22ddd248bb98c2
- pre_sync_report_blob: ad4156e26fae85bfa1049e71ea9738640b92a715
- ci_run_id: 29400618638
- ci_run_number: 1954
- ci_conclusion: success

policy_candidate:
- execution_owner: operator-run manual SQLx command
- schema_owner: Storage
- operational_sequence_owner: Deployment
- automatic_startup_migration: forbidden/unaccepted
- all_writers_quiesced: required
- postgres_object_store_same_window: required
- rollback: operator-approved restore, not automatic down/reset/drop
- secrets_and_backups_tracked: forbidden

protected_scope:
- architecture review only
- no DEP-P5A implementation
- no service/config/runbook edits
- no migrations executed
- no secrets or sibling branch changes

next_gate:
- ARCHITECT_ACCEPT -> DEP-P5A implementation slot
- policy ambiguity -> focused policy decision/fix slot
