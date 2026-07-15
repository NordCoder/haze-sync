# Control State

component: deployment
branch: component/deployment
status: PROMPT_READY
active_prompt: deploy/control/prompt.md
active_report: deploy/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: deployment — W1 DEP-P5A Worktree Runtime Fan-In

wave: W1
phase: DEP-P5A-WORKTREE-RUNTIME-CONFIG-FAN-IN
implementation_status: NOT_STARTED_PRODUCT
clean_review_status: NOT_STARTED
ci_status: NOT_RUN_PRODUCT
architect_status: ARCHITECT_ACCEPT
known_failed_checks: []

synchronized_baseline:
- exact_main_sha: c1e69a664388b0cba028170e8398b9088218957d
- post_sync_sha: 54e0b8b84e06e7475dc99ea25b22ddd248bb98c2
- pre_sync_ci_run_id: 29400618638
- pre_sync_ci_run_number: 1954
- pre_sync_ci_conclusion: success

migration_policy:
- architecture_report_blob: 737a3395945d94479c19b27d771384b57c267d06
- status: ARCHITECT_ACCEPT
- execution_owner: human/operator manual SQLx
- schema_owner: Storage
- operational_sequence_owner: Deployment
- startup_migrations: forbidden
- writer_quiescence: required
- rollback: operator-approved coordinated restore

implementation_architecture:
- base compose remains disabled and has no Worktree bind
- one opt-in Worktree compose override
- operator-supplied host path required
- target path /var/lib/haze-sync/worktree
- bind read-only by default
- adapter mode disabled by default
- no automatic write-capable or bidirectional rollout
- no migration runner or startup side effect

protected_scope:
- deploy override/config/docs only
- placeholder-only .env.example changes
- no product component changes
- no real secrets or host mutations
- no migrations/workflows/remote automation

next_gate:
- SELF_ACCEPT plus green exact-SHA CI -> focused DEP-P5A functional/security review
- implementation defect -> focused Deployment fixer
