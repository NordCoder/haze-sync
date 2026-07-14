# Control State

component: api
branch: component/api
status: PROMPT_READY

active_prompt: crates/haze-sync-api/control/prompt.md
active_report: crates/haze-sync-api/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: api — W1 API-P8 CI Diagnostics Fix
prompt_revision: verified by Orchestrator after API-P8 product completion with Rust checks green and diagnostics finalizer red; artifact-first fixer required

wave: W1
phase: FIX-API-P8-CI

implementation_status: BLOCKED_BY_TOOLING
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: CI_RED_FINALIZER_ONLY
known_failed_checks:
- Finalize CI diagnostics

candidate:
- final_code_bearing_sha: 8eb6e0ce44612e1e2f111415026297df8fb1d82b
- implementation_report_commit: 8f430d8c05b5151b91039c280ea20cfe2b382f3e
- implementation_report_blob: 390fa0b676b57a1796b840eb0d63f9ebae2599b4
- product_scope_audit: allowed API DTO/routes/fixture/tests/docs only

ci_evidence:
- workflow: Component CI
- run_id: 29313793376
- run_number: 1922
- attempts: 2
- head_sha: 8eb6e0ce44612e1e2f111415026297df8fb1d82b
- conclusion: failure
- cargo_fmt: success
- cargo_check: success
- cargo_test: success
- cargo_clippy: success
- diagnostics_finalizer: failure
- diagnostics_upload: success

artifact:
- id: 8303189576
- name: ci-diag__component-api__wf-component-ci__run-29313793376__attempt-2
- expired: false
- expires_at: 2026-07-15T07:16:40Z
- digest: sha256:03e85bcdf1b7d796eadb6415767bab60cf1399f4565440f397f7f585c03a00b6

fix_policy:
- artifact summary/manifest/failed logs are authoritative
- no cause may be guessed before artifact inspection
- preserve API-P8 product blobs unless diagnostics prove a product defect
- no suppression or weakening of finalizer
- exact post-fix Component CI success required

next_gate:
- FIX_COMPLETE plus green exact-SHA CI -> focused API-P8 functional review
- artifact insufficient -> FIX_BLOCKED_BY_LOGS
- CLI-P6A, Server HTTP fan-in and Deployment remain blocked
