# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: server — W1 SRV-P7B4 Lifecycle Fix
prompt_revision: verified by Orchestrator after functional review found fail-open enabled startup and missing Server-owned lifecycle coverage; formatting/style excluded from blocking scope

wave: W1
phase: SRV-P7B4-LIFECYCLE-FIX

implementation_status: SELF_ACCEPT
fix_status: NOT_STARTED
clean_review_status: CLEAN_NEEDS_FIX
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: CI_GREEN_FOR_REVIEWED_SHA
known_failed_checks: []

reviewed_candidate:
- code_bearing_sha: 5536d260bb4f95ec11c0cd07501c23903a72757d
- functional_review_report_commit: de06e35bb687448ea61fe6eacc0d1dd76ad04766
- functional_review_report_blob: 07e298a0622c6e5b1210160dc70b04c714a858e7
- review_status: CLEAN_NEEDS_FIX

substantive_blockers:
- enabled host returns before runtime/watcher startup acknowledgement
- startup failure is asynchronous and can allow HTTP serving to begin
- early task failure may leave shared status at Starting
- Server-owned lifecycle/join/concurrency tests are insufficient

required_fix:
- bounded cancellation-safe startup acknowledgement
- fail-closed coarse startup error propagation
- publish Failed on all task error paths
- cleanup and mandatory join/abort-await before returning startup error
- real Server-host Tokio lifecycle tests for startup, polling, manual, Busy, no-overlap, DryRun, cancellation, timeout and join
- no stylistic or formatting-only work

completion_requirements:
- real code-bearing lifecycle fix commit
- exact final SHA recorded
- authoritative DB-capable exact-SHA Component CI green or honest blocker
- committed FIX report exists

next_gate_after_fix:
- FIX_COMPLETE plus green exact-SHA CI -> final focused functional verification
- formatting/style does not block
- SRV-P7B5 and API-P8 remain blocked until SRV-P7B4 CLEAN_ACCEPT
