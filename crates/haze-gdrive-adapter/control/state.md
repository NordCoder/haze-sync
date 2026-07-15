# Control State

component: gdrive-adapter
repository: NordCoder/haze-sync
branch: component/gdrive-adapter
status: PROMPT_READY
repository_access_verified: yes
control_ref_source: component/gdrive-adapter
default_branch_control_is_active: no

active_prompt: crates/haze-gdrive-adapter/control/prompt.md
active_report: crates/haze-gdrive-adapter/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: gdrive-adapter — W1 GDA-GDA-P1 Config Mode Normalization

wave: W1
phase: GDA-GDA-P1-CONFIG-MODE-NORMALIZATION
implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: NOT_RUN
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN
known_failed_checks: []

accepted_baseline:
- component_local_product_sha: 06a7051a7e14c1da45de8cf96a78658b59cb823e
- synchronized_sha: f9a2da6eb9ac6f59b1ec18ae4d85eb51964f3cbe
- exact_main_ancestor: c1e69a664388b0cba028170e8398b9088218957d
- architecture_report_blob: 14c427880e1201d851cdc9ee04b9cd0e83334de4

required_surface:
- authoritative HAZE_GDRIVE_MODE
- fail-closed legacy HAZE_GDRIVE_DRY_RUN compatibility
- explicit mode capability matrix
- safe redacted config/status output
- exhaustive mode and contradiction tests

protected_scope:
- one active GDrive phase only
- no OAuth/provider client
- no HTTP/Storage integration
- no scheduler or long-running loop
- no status API or Deployment wiring
- no sibling component or workflow changes

next_gate:
- SELF_ACCEPT plus exact-SHA green CI -> focused GDrive clean review
- implementation or CI defect -> fixer loop
