# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-P7B2 Local Git Sync and PostgreSQL Run
prompt_revision: verified by Orchestrator after connector-only synchronization proved blocked by missing tree-read/native merge capability

wave: W1
phase: SRV-P7B2-LOCAL-SYNC-DB-CI

implementation_status: SELF_ACCEPT_PENDING_CI
fix_status: FIX_BLOCKED_BY_TOOLING
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: NOT_RUN_FOR_SYNCHRONIZED_DB_HEAD

accepted_srv_p7b2_implementation_sha: 81e6f6ae69f4bda92d284991e4909457a6ef8e60
post_source_fix_sha: 9304263f4be8234e513bf335dc886f96c53d0cce
postgres_tooling_sha: d48bbd847c8b79511a7ac32cfcb14c671f3880c1
prior_artifact_id: 8252500221
prior_artifact_status: READ_VERIFIED
connector_only_sync_status: BLOCKED_BY_TOOLING

observed_component_head_before_rotation: 229baaf9cd00ca70ea294b6123b7e678ce50f272
observed_main_head: c1e69a664388b0cba028170e8398b9088218957d
observed_merge_base: 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2
pr_number: 45
pr_status: OPEN_DRAFT_UNMERGED
pr_mergeability: FALSE

explicit_tooling_authorization:
- local git clone/fetch/checkout is allowed for this phase
- git merge --no-ff origin/main into component/server is allowed
- genuine conflict resolution and parent/tree inspection are allowed
- normal authenticated non-force push to component/server is allowed
- GitHub connector remains required for PR, CI and report evidence
- rebase, reset, history rewrite, force-push and merge into main remain forbidden

accepted_application_services:
- reusable async file PUT, guarded DELETE, changes and revision-content services
- routes remain transport/auth/API mapping adapters
- shared transaction, advisory-lock, idempotency, Core, object-store and Storage choreography
- deterministic secret-safe future Worktree idempotency derivation
- public route behavior and SRV-P7A lifecycle preserved

required_sync_evidence:
- true two-parent merge with component head first and current main second
- all current-main CI files incorporated
- semantic component-ci merge preserves Server PostgreSQL provisioning
- no temporary write-enabled bootstrap workflow remains
- PR becomes mergeable and branch head equals pushed merge commit

required_db_evidence:
- PostgreSQL service healthy
- fmt, check, workspace test, clippy and finalizer green
- all three mandatory SRV-P7B2 DB-backed parity tests successful
- no silent skips, ignored tests, fake repositories or assertion weakening

parallel_owner_status:
- WT-P10 is CLEAN_ACCEPT at 1942946331e8362f19907ab6ad4eb779da70fd57
- STOR-P10 independently requires local-git synchronization and strict PostgreSQL verification

blocked_downstream:
- SRV-P7B2 clean-code review remains blocked until synchronized DB-capable CI is green
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 are all CLEAN_ACCEPT with exact accepted SHAs synchronized
- SRV-P7B4, API-P8, SRV-P7B5, CLI-P6A and DEP-P5A remain blocked

next_gate_after_sync:
- SELF_ACCEPT plus mergeable PR and green DB-capable Component CI -> mandatory SRV-P7B2 clean-code review
- red CI -> exact artifact-based fixer after Orchestrator metadata retrieval
- local git/push unavailable -> BLOCKED_BY_TOOLING manual intervention hold