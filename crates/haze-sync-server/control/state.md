# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: server — W1 SRV-P7A CI Fix
prompt_revision: verified by Orchestrator after SRV-P7A-FIX implementation report and exact diagnostics artifact metadata retrieval

wave: W1
phase: FIX-SRV-P7A-CI

implementation_status: SELF_ACCEPT
clean_review_status: NOT_STARTED
ci_status: CI_RED
ci_workflow: Component CI
ci_code_bearing_sha: fe9101871462fc271a320726f4ad18668d1a9a5b
ci_run_id: 29160824666
ci_run_number: 1703
ci_run_attempt: 1
known_failed_checks:
- Finalize CI diagnostics
architect_status: ARCHITECT_ACCEPT

artifact_name: ci-diag__component-server__wf-component-ci__run-29160824666__attempt-1
artifact_id: 8250773210
artifact_head_sha: fe9101871462fc271a320726f4ad18668d1a9a5b
artifact_digest: sha256:a02d9916569faa4457f8b3509b6b11908b8805f113b82c9aa96e58d10e1b7d6d
artifact_size_bytes: 1993
artifact_expires_at: 2026-07-12T17:02:32Z
artifact_status: AVAILABLE_UNEXPIRED

accepted_implementation:
- Worktree snapshot remains 32/32 exact at source SHA 4f7bc748d9b901d7d5c3e43c845ba407c0c36e59
- Server worktree_runtime module is active and compiled
- production lifecycle constructs, starts, retains, and shuts down the boundary explicitly
- disabled is inert; enabled execution remains honestly unavailable until SRV-P7B

next_gate_after_fix: mandatory SRV-P7A clean-code review only after FIX_COMPLETE and independently verified green post-fix code-bearing Component CI
