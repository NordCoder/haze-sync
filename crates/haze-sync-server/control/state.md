# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-P7B2 Branch Sync and PostgreSQL Run
prompt_revision: verified by Orchestrator after PostgreSQL CI tooling was committed but no PR workflow run was scheduled

wave: W1
phase: SRV-P7B2-BRANCH-SYNC-DB-CI

implementation_status: SELF_ACCEPT_PENDING_CI
fix_status: FIX_BLOCKED_BY_TOOLING
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: NOT_RUN_FOR_DB_TOOLING_SHA

accepted_srv_p7b2_implementation_sha: 81e6f6ae69f4bda92d284991e4909457a6ef8e60
post_source_fix_sha: 9304263f4be8234e513bf335dc886f96c53d0cce
postgres_tooling_sha: d48bbd847c8b79511a7ac32cfcb14c671f3880c1
prior_artifact_id: 8252500221
prior_artifact_status: READ_VERIFIED
prior_failure: mandatory DB-backed parity tests had no configured PostgreSQL database

observed_main_head: c1e69a664388b0cba028170e8398b9088218957d
observed_merge_base: 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2
pr_number: 45
pr_status: OPEN_DRAFT_UNMERGED
pr_mergeability: FALSE

accepted_application_services:
- reusable async file PUT, guarded DELETE, changes and revision-content services
- routes remain transport/auth/API mapping adapters
- shared transaction, advisory-lock, idempotency, Core, object-store and Storage choreography
- deterministic secret-safe future Worktree idempotency derivation
- public route behavior and SRV-P7A lifecycle preserved

required_branch_sync:
- true two-parent merge with current component/server head first and current main head second
- include every current-main change; no ours-only or fake merge
- preserve exact main versions of non-conflicting CI docs/scripts/workflows
- semantically merge component-ci.yml with accepted Server PostgreSQL provisioning
- update branch ref by fast-forward only
- verify PR becomes mergeable and receives a new pull-request CI run

required_db_evidence:
- PostgreSQL service healthy
- fmt, check, workspace test, clippy and finalizer green
- all three mandatory SRV-P7B2 DB-backed parity tests execute successfully
- no silent skips, ignored tests, fake repositories, or assertion weakening

parallel_owner_status:
- WT-P10 is CLEAN_ACCEPT at 1942946331e8362f19907ab6ad4eb779da70fd57
- STOR-P10 independently requires branch synchronization and strict PostgreSQL verification

blocked_downstream:
- SRV-P7B2 clean-code review remains blocked until synchronized DB-capable CI is green
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 are all CLEAN_ACCEPT with exact accepted SHAs synchronized
- SRV-P7B4, API-P8, SRV-P7B5, CLI-P6A and DEP-P5A remain blocked

next_gate_after_sync:
- SELF_ACCEPT plus mergeable PR and green DB-capable Component CI -> mandatory SRV-P7B2 clean-code review
- red CI -> exact artifact-based fixer after Orchestrator metadata retrieval
- no scheduled run or impossible merge -> exact tooling/scope decision; no clean review