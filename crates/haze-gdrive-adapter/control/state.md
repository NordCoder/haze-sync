# Control State

component: gdrive-adapter
branch: component/gdrive-adapter
status: PROMPT_READY

active_prompt: crates/haze-gdrive-adapter/control/prompt.md
active_report: crates/haze-gdrive-adapter/control/report.md
active_agent_role: fixer-worker

wave: W1
phase: FIX-GDA-P8C-CI

implementation_status: SELF_ACCEPT_PENDING_CI
clean_review_status: CLEAN_ACCEPT_PENDING_CI
ci_status: CI_RED
ci_workflow: Component CI
ci_code_bearing_sha: cfaf931bfa3ec58b19924b535475859300216272
ci_run_id: 29143925456
ci_run_number: 1653
ci_run_attempt: 1
ci_artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29143925456__attempt-1
ci_artifact_id: 8246115447
known_failed_checks:
- Finalize CI diagnostics
architect_status: ARCHITECT_ACCEPT
next_gate_after_fix: component phase completion after a verified green post-fix code-bearing CI run
dependency_note: durable delete-candidate persistence and concrete runtime wiring remain explicit Storage/Server/API fan-in concerns
