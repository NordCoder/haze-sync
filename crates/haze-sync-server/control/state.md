# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: clean-code-reviewer
assigned_chat_name: server — W1 API-P8 Worktree HTTP Review
prompt_revision: verified by Orchestrator after SELF_ACCEPT and green DB-capable exact-SHA CI; formatting/style excluded from blocking scope

wave: W1
phase: SRV-API-P8-HTTP-FUNCTIONAL-REVIEW

implementation_status: SELF_ACCEPT
fix_status: NOT_REQUIRED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: CI_GREEN_DB_VERIFIED
known_failed_checks: []

candidate:
- accepted_server_baseline: 1d1fc8ca62c97db041cca09dd8316370285dfba1
- accepted_api_p8_sha: 56ae94570441d68715f34b5d54381a0fc4d7c231
- final_server_code_bearing_sha: be2b1c16fa6c4919d446b76b1f15dca5767b2482
- implementation_report_blob: 9b4244bb4d6f37edbca85d6ab809ad171999f4f1
- post_candidate_changes_before_rotation: report/control only

ci_evidence:
- workflow: Component CI
- run_id: 29321038276
- run_number: 1938
- run_attempt: 1
- head_sha: be2b1c16fa6c4919d446b76b1f15dca5767b2482
- conclusion: success
- db_capable: yes
- cargo_fmt: success
- cargo_check: success
- cargo_test: success
- cargo_clippy: success
- diagnostics_finalizer: success

review_policy:
- formatting, rustfmt, naming taste and style are non-blocking and out of scope
- verify exact API fan-in, unique shutdown ownership, weak HTTP control, passive status, single submission, ticket-drop semantics, HTTP/auth/body mappings, secrecy and protected scope

completion_requirements:
- committed CLEAN_CODE_REVIEW report exists
- report phase SRV-API-P8-HTTP-FUNCTIONAL-REVIEW
- report chat name server — W1 API-P8 Worktree HTTP Review
- exact final SHA reviewed
- no later product/tooling invalidation

next_gate:
- CLEAN_ACCEPT -> resolve and activate CLI-P6A control slot
- substantive defect -> focused Server fixer only
- Deployment remains blocked until CLI/downstream acceptance
