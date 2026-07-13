# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: clean-code-reviewer
assigned_chat_name: server — W1 SRV-P7B4 Lifecycle Verification
prompt_revision: verified by Orchestrator after FIX_COMPLETE lifecycle correction and green DB-capable exact-SHA CI; formatting/style excluded from blocking scope

wave: W1
phase: SRV-P7B4-LIFECYCLE-VERIFY

implementation_status: SELF_ACCEPT
fix_status: FIX_COMPLETE
clean_review_status: NOT_STARTED_FINAL_FUNCTIONAL
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: CI_GREEN_DB_VERIFIED
known_failed_checks: []

final_candidate:
- previous_code_bearing_sha: 5536d260bb4f95ec11c0cd07501c23903a72757d
- final_code_bearing_sha: 55ed0d6c6ab9b78a953b954fcf5a9a68a6a708fe
- fix_report_commit: a0dc770a14e2a9d1a9a0aab2c30aef8e0c4e5ea5
- fix_report_blob: 7963d704e7dfba89ef8d7e26178e772d7f3a947a
- post_candidate_changes: report-only before rotation

ci_evidence:
- workflow: Component CI
- run_id: 29281569666
- run_number: 1893
- run_attempt: 1
- head_sha: 55ed0d6c6ab9b78a953b954fcf5a9a68a6a708fe
- conclusion: success
- db_capable: yes
- cargo_fmt: success
- cargo_check: success
- cargo_test: success
- cargo_clippy: success
- diagnostics_finalizer: success

review_policy:
- formatting, rustfmt, naming taste and style are non-blocking and out of scope
- verify only startup acknowledgement, failed-start cleanup, status, task ownership/join, lifecycle tests, secrecy, CI and protected scope

completion_requirements:
- committed CLEAN_CODE_REVIEW report exists
- report phase SRV-P7B4-LIFECYCLE-VERIFY
- report chat name server — W1 SRV-P7B4 Lifecycle Verification
- exact final SHA reviewed
- no later product/tooling invalidation

next_gate_after_review:
- CLEAN_ACCEPT -> begin SRV-P7B5 status/readiness work immediately
- substantive defect -> focused fix only
- API-P8 remains blocked until SRV-P7B5 contract is accepted
