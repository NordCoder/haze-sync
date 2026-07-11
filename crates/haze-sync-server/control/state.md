# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-P7A Wiring Fix
prompt_revision: verified by Orchestrator after SRV-P7A report recovery found uncompiled and unwired Server composition code

wave: W1
phase: SRV-P7A-FIX

implementation_status: SELF_NEEDS_FIX
clean_review_status: NOT_STARTED
ci_status: CI_GREEN_INSUFFICIENT
ci_workflow: Component CI
ci_code_bearing_sha: 71fd46ceb8b50f2523cacd70165dcca63881aa82
ci_run_id: 29158883879
ci_run_number: 1701
ci_run_attempt: 1
known_failed_checks: []
architect_status: ARCHITECT_ACCEPT

fan_in_source_branch: component/worktree
fan_in_source_sha: 4f7bc748d9b901d7d5c3e43c845ba407c0c36e59
fan_in_source_status: 32_OF_32_PRODUCT_BLOBS_IDENTICAL

verified_defects:
- crates/haze-sync-server/src/worktree_runtime.rs is not included in the active crate module graph
- its six focused tests were not compiled or executed by the green CI run
- production startup does not construct the boundary from ServerConfig.worktree
- production startup/shutdown does not call ServerWorktreeRuntime start/shutdown

correction_scope: activate and wire the existing honest composition boundary only; real Core/API/Storage-backed cycle execution remains SRV-P7B
next_gate_after_fix: mandatory SRV-P7A clean-code review only after a new code-bearing Component CI run compiles the activated module and concludes green
