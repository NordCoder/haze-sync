# Control State

component: gdrive-adapter
branch: component/gdrive-adapter
status: PROMPT_READY

active_prompt: crates/haze-gdrive-adapter/control/prompt.md
active_report: crates/haze-gdrive-adapter/control/report.md
active_agent_role: fixer-worker

wave: W1
phase: FIX-GDA-P8-CI

implementation_status: SELF_ACCEPT_PENDING_CI
clean_review_status: NOT_STARTED
ci_status: CI_RED
ci_workflow: Component CI
ci_run_id: 29125187320
ci_run_number: 1587
ci_run_attempt: 1
ci_artifact_id: 8240045016
ci_artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29125187320__attempt-1
ci_artifact_expires_at: 2026-07-11T21:37:36Z
known_failed_checks:
- diagnostics-artifact-required
architect_status: ARCHITECT_ACCEPT
dependency_note: durable delete-candidate persistence and concrete runtime wiring remain explicit fan-in concerns
