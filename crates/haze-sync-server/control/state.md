# Control State

component: server
repository: NordCoder/haze-sync
branch: component/server
status: PROMPT_READY
repository_access_verified: yes
control_ref_source: component/server
default_branch_control_is_active: no

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: server — W1 FIX-SRV-GDA-P1 Outcome Mapping

wave: W1
phase: FIX-SRV-GDA-P1-OUTCOME-MAPPING
implementation_status: COMPLETE_AFTER_CONTINUATION
fix_status: REVIEW_FIX_NOT_STARTED
clean_review_status: CLEAN_NEEDS_FIX
ci_status: CI_GREEN_DB_VERIFIED_ON_REVIEWED_SHA
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

review_candidate:
- code_bearing_sha: b0ae522229bbc6422763a2cd075b995768346963
- clean_review_report_blob: d4fe8c9150184f34383d708048d402df7c008078
- ci_run_id: 29490745222
- ci_run_number: 2041
- ci_conclusion: success
- db_capable: yes

required_fix:
- map persisted cursor-generation mismatch to invalid_cursor_state
- map internal and unexpected Storage failures to safe internal envelope
- keep validation_failed only for genuine validation categories
- add/update PostgreSQL route assertions with rollback and secrecy evidence

protected_scope:
- Server outcome mapping and focused route tests only
- accepted API and Storage owner files remain byte-identical
- no provider/OAuth/Core policy/status-control/CLI/Deployment/sibling/workflow changes

next_gate:
- FIX_COMPLETE plus exact-SHA DB-capable green CI -> repeat Server clean functional review
- unresolved owner mismatch -> FIX_BLOCKED_BY_CONTRACT
