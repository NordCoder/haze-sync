# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: architect
assigned_chat_name: server — W1 SRV-P7B Architecture Decision
prompt_revision: verified by Orchestrator after SRV-P7B1 returned BLOCKED_BY_CONTRACT

wave: W1
phase: ARCH-SRV-P7B-CONTRACTS

implementation_status: BLOCKED_BY_CONTRACT
fix_status: NOT_REQUIRED
clean_review_status: NOT_APPLICABLE_FOR_BLOCKED_PHASE
architect_status: NOT_STARTED
ci_status: NOT_RUN

accepted_srv_p7a_code_bearing_sha: 37706634fd8dd2d9b299a1c453718f2de63981d0
accepted_srv_p7a_ci_run: 29161721748
accepted_srv_p7a_ci_run_number: 1704
accepted_srv_p7a_ci_status: CI_GREEN
accepted_srv_p7a_clean_review_status: CLEAN_ACCEPT

blocked_phase: SRV-P7B1
blocked_report_status: BLOCKED_BY_CONTRACT
blocked_report_archive: crates/haze-sync-server/control/log/20260711-184500Z-W1-SRV-P7B1-implementation-worker-report.md

verified_contract_gaps:
- synchronous WorktreeRuntimeCycle and WorktreeRuntimeService poll cannot safely await asynchronous SQLx/Tokio authoritative operations
- correct PUT and DELETE transaction semantics are private async route orchestration rather than reusable Server application services
- durable Worktree applied/reconciliation state and export cursor ownership/contracts are absent
- runtime budgets, invocation policy, and adapter-state binding are absent from accepted config

forbidden_workarounds:
- nested Tokio runtime or Handle::block_on
- ad hoc blocking around async database operations
- internal HTTP self-calls
- detached tasks returning fabricated synchronous summaries
- duplicated Core/route policy
- production fake or in-memory-only state

architect_scope:
- choose one async-compatible Worktree/Server execution architecture
- define reusable Server application-service boundaries
- define durable Storage state and cursor ownership
- define bounded runtime policy/config and host lifecycle
- produce owner-aligned phased prompts and dependency order
- update Server-owned architecture documentation only

blocked_downstream:
- SRV-P7B1 implementation retry remains blocked until architecture and owner contracts are accepted
- SRV-P7B2 hosted scheduling remains blocked
- CLI remains blocked until accepted Server operator/runtime fan-in exists
- Deployment remains blocked until explicit runtime/config/deployment fan-in exists

next_gate_after_architect:
- ARCHITECT_CHANGED_CONTRACTS or ARCHITECT_ACCEPT: Orchestrator activates the first contract-owner implementation phase in the architect-defined order
- ARCHITECT_NEEDS_CHANGES: route an exact architecture revision prompt
- ARCHITECT_BLOCKED: resolve the exact missing evidence before implementation
