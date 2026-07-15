# Control State

component: storage
repository: NordCoder/haze-sync
branch: component/storage
status: PROMPT_READY
repository_access_verified: yes
migrated_repository_lookup_required: no
control_ref_source: component/storage
default_branch_control_is_active: no

active_prompt: crates/haze-sync-storage/control/prompt.md
active_report: crates/haze-sync-storage/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: storage — W1 STOR-GDA-P11 GDrive Durable State

wave: W1
phase: STOR-GDA-P1-DURABLE-STATE
implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN
ci_status: NOT_RUN
known_failed_checks: []

accepted_storage_baseline:
- code_bearing_sha: 66b6a1f554aae1d1b774cc88560d46dd140c7a54
- prior_ci_run_id: 29185466870
- prior_ci_run_number: 1833
- prior_ci_conclusion: success
- exact_main_ancestor: c1e69a664388b0cba028170e8398b9088218957d
- branch_head_before_slot: 30b04154523165900be568875fff6e1a69216022

architecture_input:
- report_blob: 14c427880e1201d851cdc9ee04b9cd0e83334de4
- persistence_owner: Storage
- delivery: Server/API-mediated
- direct_adapter_db_access: forbidden
- transaction_owner: later Server caller
- first_phase: STOR-GDA-P1-DURABLE-STATE

required_surface:
- versioned adapter-scoped GDrive runtime state
- Drive cursor and Core export checkpoint
- mapping/echo/delete-candidate/operation compare-and-commit
- minimum contiguous migration
- unit and real PostgreSQL evidence
- redacted passive Storage contracts

protected_scope:
- no API/Server/GDrive/Deployment product changes
- no provider/OAuth/scheduling work
- no Core or delete-unlock policy
- no real secrets or workflow changes

next_gate:
- SELF_ACCEPT plus exact-SHA DB-green CI -> focused Storage clean/DB review
- implementation or CI defect -> focused Storage fixer
