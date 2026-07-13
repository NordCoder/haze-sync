# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: server — W1 SRV-P7B5 Manual Availability Fix
prompt_revision: verified by Orchestrator after functional review found duplicated/racy Server manual-state mirror; formatting/style excluded from blocking scope

wave: W1
phase: SRV-P7B5-MANUAL-AVAILABILITY-FIX

implementation_status: SELF_ACCEPT
fix_status: NOT_STARTED
clean_review_status: CLEAN_NEEDS_FIX
architect_status: ARCHITECT_NOT_REQUIRED_UNLESS_OWNER_CONTRACT_PROVES_INSUFFICIENT
ci_status: CI_GREEN_FOR_REVIEWED_SHA
known_failed_checks: []

reviewed_candidate:
- code_bearing_sha: 78f4e4525327ff03fa1af1e8387e9e3ea07091d6
- functional_review_report_commit: 8082bc6fdd65053f66945fba5ed9047b369442e1
- functional_review_report_blob: 2f43f88adb950f59100aad80d860b268a54f4f73
- review_status: CLEAN_NEEDS_FIX

substantive_blocker:
- Server manual availability mirror is toggled around every poll
- idle DryRun poll can report false Busy
- older poll completion can overwrite a newer accepted request to Available
- manual gate/accounting ownership is duplicated rather than projected

required_fix:
- remove poll-based Busy/Available toggles
- preserve Worktree as authoritative manual gate owner
- implement race-safe request/completion projection or report precise owner-contract blocker
- deterministic tests for idle DryRun, per-request Busy lifetime, stale completion race and lifecycle overrides
- no public surface or new task/poller/runtime

completion_requirements:
- real code-bearing fix commit or honest FIX_BLOCKED_BY_CONTRACT
- exact final SHA recorded when code changes exist
- authoritative DB-capable exact-SHA Component CI green or honest blocker
- committed FIX report exists

next_gate_after_fix:
- FIX_COMPLETE plus green exact-SHA CI -> final focused manual-availability verification
- FIX_BLOCKED_BY_CONTRACT -> route minimal Worktree owner extension
- API-P8 remains blocked until SRV-P7B5 CLEAN_ACCEPT
