# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: clean-code-reviewer
assigned_chat_name: server — W1 SRV-P7B4 Hosted Runtime Functional Review
prompt_revision: verified by Orchestrator after SELF_ACCEPT implementation and green DB-capable exact-SHA CI; formatting/style excluded from blocking scope

wave: W1
phase: SRV-P7B4-FUNCTIONAL-REVIEW

implementation_status: SELF_ACCEPT
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED_FUNCTIONAL
architect_status: ARCHITECT_NOT_REQUIRED_OWNER_BLOCKER_RESOLVED
ci_status: CI_GREEN_DB_VERIFIED
known_failed_checks: []

candidate:
- accepted_integration_baseline: 1b2b572a1a200f2968d005e48e9c0674f9db8bc0
- final_code_bearing_sha: 5536d260bb4f95ec11c0cd07501c23903a72757d
- implementation_report_commit: 896f28c3c1d5d8f86d4b58e3428b1d289ee6aa8a
- implementation_report_blob: c9fa7ce4ae3876b8d602fbf15662a3651babc023
- post_candidate_changes: report-only before rotation

ci_evidence:
- workflow: Component CI
- run_id: 29278756276
- run_number: 1888
- run_attempt: 1
- head_sha: 5536d260bb4f95ec11c0cd07501c23903a72757d
- conclusion: success
- db_capable: yes
- cargo_fmt: success
- cargo_check: success
- cargo_test: success
- cargo_clippy: success
- diagnostics_finalizer: success

review_policy:
- formatting, rustfmt, naming taste and style are non-blocking and out of scope
- verify only host lifecycle, task ownership/join, shutdown, modes, manual boundary, secrecy, tests, CI and protected scope

completion_requirements:
- committed CLEAN_CODE_REVIEW report exists
- report phase SRV-P7B4-FUNCTIONAL-REVIEW
- report chat name server — W1 SRV-P7B4 Hosted Runtime Functional Review
- exact final SHA reviewed
- no later product/tooling invalidation

next_gate_after_review:
- CLEAN_ACCEPT -> begin SRV-P7B5 status/readiness work
- substantive defect -> focused fix only
- API-P8 remains blocked until SRV-P7B5 contract is accepted
