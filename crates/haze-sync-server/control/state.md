# Control State

component: server
branch: component/server
status: BLOCKED_BY_CONTRACT

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: orchestrator-hold
assigned_chat_name: none

wave: W1
phase: SRV-P7B5-WAIT-WT-MANUAL-STATUS

implementation_status: SELF_ACCEPT
fix_status: FIX_BLOCKED_BY_CONTRACT
clean_review_status: CLEAN_NEEDS_FIX
ci_status: CI_GREEN_FOR_REVIEWED_SHA

blocked_candidate:
- server_code_bearing_sha: 78f4e4525327ff03fa1af1e8387e9e3ea07091d6
- blocker_report_commit: d612196a33c56711e9102f09eaf1340ed7b696e0
- blocker_report_blob: 1b4d3ee699aecb155e90916c6b52a5bd5d34644c
- ci_run_id: 29283227887
- ci_run_number: 1901
- ci_conclusion: success

contract_blocker:
- accepted Worktree handle has no passive authoritative lifecycle/busy status
- manual completion has no request identity/generation
- Server-only projection is racy or duplicates gate ownership

required_owner_extension:
- preferred: cloneable passive Worktree manual status handle exposing lifecycle and busy only
- acceptable: owner-assigned opaque generation carried through Accepted and Manual completion
- no payload, path, token, cursor or backend details
- no new task/poller/runtime

next_gate:
- Worktree owner implementation and exact-SHA clean acceptance
- exact-SHA fan-in to Server
- resume narrow SRV-P7B5 manual projection fix
- API-P8 remains blocked
