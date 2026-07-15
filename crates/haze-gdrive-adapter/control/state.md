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
assigned_chat_name: gdrive-adapter — W1 GDA-GDA-P3 Live Google OAuth

wave: W1
phase: GDA-GDA-P3-LIVE-GOOGLE-OAUTH
implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: NOT_RUN
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

accepted_inputs:
- config_mode_sha: fcc04afd1fe9808656d9bc2effbfff7160efe9fc
- config_clean_report_blob: d9bc16c1a50755ccaecd1b51add231bf52e7e35f
- architecture_report_blob: 14c427880e1201d851cdc9ee04b9cd0e83334de4

required_surface:
- read-only versioned credential-file contract
- Google client and in-memory refresh boundary
- fakeable provider client abstraction
- safe auth/scope/provider categories
- startup preflight and comprehensive redaction
- synthetic fake-based tests only

protected_scope:
- one active GDrive phase only
- no Server/API HTTP or Storage integration
- no scheduler or deployment wiring
- no real credentials or live-network CI
- no sibling or workflow changes

next_gate:
- SELF_ACCEPT plus exact-SHA green CI -> focused OAuth/security clean review
- implementation or CI defect -> fixer loop
