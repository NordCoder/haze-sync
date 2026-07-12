# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-P7B3 Exact-SHA Fan-In
prompt_revision: verified by Orchestrator after full component/dependency audit and exact-SHA divergence checks

wave: W1
phase: SRV-P7B3-EXACT-SHA-FAN-IN

implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: NOT_RUN
known_failed_checks: []

accepted_owner_shas:
- worktree: 1942946331e8362f19907ab6ad4eb779da70fd57
- storage: 66b6a1f554aae1d1b774cc88560d46dd140c7a54
- server: 647dce7b624d67663632808906896cb6745ea7e7

accepted_owner_evidence:
- WT-P10 CLEAN_ACCEPT; CI 29167289593 run 1799 success
- STOR-P10 CLEAN_ACCEPT; CI 29185466870 run 1833 success; cross-branch report commit 13a0c80123061848235676b2a7dc7c3a3c644dee
- SRV-P7B2 CLEAN_ACCEPT; CI 29186058268 run 1835 success; clean report commit 053eea1496bf9b541b462a82989b4cd956ed7276

fan_in_validation:
- component/server diverges from accepted Storage SHA
- component/server diverges from accepted Worktree SHA
- accepted owner product snapshots are not yet synchronized in one integration line
- whole component-branch merges are forbidden
- exact non-control owner product path transfer is required

required_preservation:
- current accepted Server application-service behavior
- transport-only routes
- Server Storage test-support normal/dev dependency separation
- isolated Server and Storage PostgreSQL CI services
- complete workspace test coverage
- no executor, host, scheduler or public DTO implementation in this phase

allowed_integration_scope:
- exact Worktree non-control product/test/docs snapshot
- exact Storage migrations and non-control product/test/docs snapshot
- Cargo.lock when required
- minimal Server-local compile/test compatibility corrections directly caused by accepted contracts
- Server control report

forbidden_scope:
- sibling control files or stale workflows
- SRV-P7B3 executor behavior
- SRV-P7B4 hosted runtime
- API-P8 status DTOs
- CLI or Deployment changes
- Core policy or schema redesign

next_gate_after_implementation:
- SELF_ACCEPT plus green DB-capable Component CI -> mandatory fan-in clean-code review
- fan-in CLEAN_ACCEPT -> activate SRV-P7B3 Bounded Worktree Executor
- blocked or red -> route exact contract/scope/tooling/CI evidence