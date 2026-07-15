# Control State

component: obsidian-plugin
repository: NordCoder/haze-sync
branch: component/obsidian-plugin
status: PROMPT_READY
repository_access_verified: yes
control_ref_source: component/obsidian-plugin
default_branch_control_is_active: no

active_prompt: apps/haze-obsidian-plugin/control/prompt.md
active_report: apps/haze-obsidian-plugin/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: obsidian-plugin — W1 OBS-FAN-IN-P1 Server Compatibility E2E

wave: W1
phase: OBS-FAN-IN-P1-SERVER-COMPAT-E2E
implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: NOT_RUN
architect_status: ARCHITECT_ACCEPT_COMPONENT_COMPLETE
known_failed_checks: []

accepted_baseline:
- obsidian_code_bearing_sha: 3b47c3c96fd25d434ec3ce0f8821c0c3feb6a423
- prior_ci_run_id: 29120283487
- prior_ci_run_number: 1528
- prior_ci_conclusion: success
- exact_main_ancestor: c1e69a664388b0cba028170e8398b9088218957d
- api_contract_sha: 56ae94570441d68715f34b5d54381a0fc4d7c231
- server_http_sha: 50461354c18ddc4d2e47202d9303b4358a27ee45

required_surface:
- accepted API fixture compatibility
- deterministic fake HTTP integration harness
- optional loopback Server smoke mode
- upload/download/delete/conflict/error contract coverage
- redaction and synthetic-only fixtures

protected_scope:
- one active Obsidian phase only
- no sibling component changes
- no route or DTO invention
- no external-network-required CI
- no private data, credentials or release packaging
- no workflow changes

next_gate:
- SELF_ACCEPT plus exact-SHA green CI -> focused Obsidian clean/integration review
- missing accepted Server/API contract -> BLOCKED_BY_CONTRACT
- implementation or CI defect -> fixer loop
