# Control State

component: api
repository: NordCoder/haze-sync
branch: component/api
status: PROMPT_READY
repository_access_verified: yes
control_ref_source: component/api
default_branch_control_is_active: no

active_prompt: crates/haze-sync-api/control/prompt.md
active_report: crates/haze-sync-api/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: api — W1 FIX-API-GDA-P1 Debug Redaction

wave: W1
phase: FIX-API-GDA-P1-DEBUG-REDACTION
implementation_status: COMPLETE_AFTER_CI_FIX
fix_status: REVIEW_FIX_NOT_STARTED
clean_review_status: CLEAN_NEEDS_FIX
ci_status: CI_GREEN_ON_REVIEWED_SHA
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

review_candidate:
- code_bearing_sha: 77945118a37c6e8efec04ecd054e0e5a5e4435ba
- clean_review_report_blob: bcd51da350f8d7bdb7dec19a7a9f6757addff3e3
- ci_run_id: 29440533856
- ci_run_number: 2017

required_fix:
- eliminate private provider/path fact Debug exposure
- prevent recursive authenticated commit body formatting
- add sentinel secrecy tests
- preserve wire fixtures and semantics

next_gate:
- FIX_COMPLETE plus full exact-SHA green CI -> repeat API clean review
- unresolved secrecy defect -> FIX_NEEDS_MORE
