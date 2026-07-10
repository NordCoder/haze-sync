REPORT_TYPE:
CLEAN_CODE_REVIEW

STATUS:
CLEAN_BLOCKED_BY_CONTRACT

AGENT:
role: clean-code-reviewer
agent_execution_id: W1-STOR-P9C-storage-test-support-clean-code-review
chat_name: storage — W1 STOR-P9C Clean-Code Review

COMPONENT:
name: storage
path: crates/haze-sync-storage
branch: component/storage
contract_path: crates/haze-sync-storage/docs/component-contract.md
plan_path: crates/haze-sync-storage/docs/implementation-plan.md
dependency_map_path: crates/haze-sync-storage/docs/dependency-map.md
control_prompt_path: crates/haze-sync-storage/control/prompt.md
control_report_path: crates/haze-sync-storage/control/report.md

WAVE:
id: W1
phase_id: STOR-P9C
dependency_status: active control state was PROMPT_READY; active_prompt matched crates/haze-sync-storage/control/prompt.md; active role was clean-code-reviewer; STOR-P9 implementation and artifact-based CI correction were complete; Component CI run 29120476441 was recorded and observed green before this review

SUMMARY:
Reviewed STOR-P9 test-support configuration, feature gating, optional and strict database helpers, URL/error secrecy, migration locking, partial-schema detection, fixture generation, object-root cleanup, downstream usage, and documentation. Fixed four Storage-local correctness and safety defects: generated operation ids did not satisfy the shared `OperationId` `op_` prefix contract; partial object-root setup could leave filesystem residue and failed explicit cleanup disabled Drop retry; URL query parameters could alter database target/session behavior despite path validation; and schema completeness counted views as owned tables. Added contract-valid fixture tests, partial/failing cleanup tests, query-parameter safety tests, and base-table probe assertions. Updated the runbook and implementation log to distinguish optional not-run behavior from strict mandatory execution and to state downstream feature-isolation requirements. Final source/docs head aa59064d641f4850f7c70fa615e638b52613dd95 passed Component CI run 29124956486. Clean acceptance is blocked because `crates/haze-sync-server/Cargo.toml` enables Storage `test-support` in the Server's normal dependency set, so production Server builds compile/export test-only Storage helpers. Correcting that requires a sibling Server dependency edit forbidden by this prompt.

CHANGED_FILES:
- crates/haze-sync-storage/src/test_support/env.rs
- crates/haze-sync-storage/src/test_support/ids.rs
- crates/haze-sync-storage/src/test_support/object_root.rs
- crates/haze-sync-storage/src/test_support/postgres.rs
- crates/haze-sync-storage/docs/test-support.md
- crates/haze-sync-storage/docs/implementation-log.md
- crates/haze-sync-storage/control/report.md

BRANCH_AND_CONTROL:
current_branch: component/storage
base_branch: main
base_sha: current main head c1e69a664388b0cba028170e8398b9088218957d; merge base 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2
head_sha: aa59064d641f4850f7c70fa615e638b52613dd95 before report write; report write creates the next branch head
default_branch_modified: no
sibling_branch_modified: no
control_prompt_read: crates/haze-sync-storage/control/prompt.md
control_report_written: crates/haze-sync-storage/control/report.md
control_files_archived_by_worker: no
ci_skip_used: yes for the final report-only commit only
ci_skip_reason: final commit updates only crates/haze-sync-storage/control/report.md; all source/test/docs review commits used normal CI and source/docs head aa59064d641f4850f7c70fa615e638b52613dd95 completed Component CI run 29124956486 successfully

SCOPE:
allowed_files_only: yes
scope_expansion_used: no
scope_expansion_rationale: none
cross_component_changes: none
forbidden_files_touched: none by this worker; phase comparison also contains orchestrator-owned control-slot/archive changes created before review execution

CONTRACT:
contract_read: yes
contract_satisfied: no at the workspace dependency boundary; Storage's local cfg gate is correct, but Server enables the test-support feature in its normal production dependency declaration
contract_changes_requested: no semantic Storage contract expansion; a cross-component dependency correction is required so Server uses default Storage features normally and enables test-support only as a dev-dependency or dedicated test harness
contract_change_rationale: Cargo features are enabled by dependents; the current Server normal dependency defeats Storage's invariant that test support is not part of the production dependency graph
 affected_components: storage and server

IMPLEMENTATION_OR_REVIEW:
completed: yes for all in-scope review and corrections; final clean acceptance blocked by the cross-component dependency declaration
main_changes:
- Reviewed exclusive HAZE_SYNC_TEST_DATABASE_URL discovery, redacted URL/errors, optional compatibility behavior, strict required/prepare behavior, advisory-lock setup/cleanup, partial-schema rejection, cfg/feature gates, fixture helpers, filesystem cleanup, and downstream usage.
- Changed TestNamespace::operation_id to emit the required `op_` prefix and added Common `AdapterId`, `OperationId`, and `VaultPath` acceptance tests.
- Removed partially created object roots after layout failure and kept Drop retry enabled after explicit cleanup failure.
- Allowlisted safe PostgreSQL URL query keys and rejected target/session-changing, duplicate, encoded-key, missing-value, and empty query configurations through the existing redacted invalid-URL error.
- Restricted schema completeness checks to information_schema base tables so views cannot satisfy migration readiness.
- Clarified downstream feature placement, optional not-run semantics, strict mandatory helper behavior, base-table requirements, typed fixture guarantees, and cleanup retry behavior in docs.
behavior_changes: test fixture operation ids are now valid Common operation identifiers; unsafe database URL options are rejected; views no longer count as complete storage schema; failed filesystem setup is cleaned and failed explicit cleanup receives a Drop retry
bugs_found:
- operation_id fixtures omitted the required `op_` prefix
- object-root setup and cleanup failure paths could leave avoidable residue
- unrestricted PostgreSQL URL query options could override target/session behavior
- schema completeness could count views as owned tables
- Server's normal dependency enables Storage test-support in production builds
bugs_fixed: all four Storage-local defects were fixed with focused tests and documentation
cleanups_made: aligned documentation with actual optional/strict APIs and made downstream production feature isolation explicit
non_goals_preserved: no production repository semantics, migrations/schema expansion, Server code, provider behavior, workflow/dependency edits, sibling changes, test deletion, or assertion weakening
deferred_work: move `haze-sync-storage` feature `test-support` out of Server's normal dependency and into a dev-only/test-harness dependency; execute live PostgreSQL feature tests with a dedicated reachable test database

TESTS_AND_CHECKS:
checks_run:
- Re-read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, and chatgpt-gh-connector.md from Project Sources.
- Read current control state, exact STOR-P9C prompt, prior FIX-STOR-P9-CI report, component contract, STOR-P9 implementation-plan section, implementation log, dependency map, current test-support source/docs, Storage lib/Cargo feature gates, Server Cargo dependency declaration, representative Server production modules, PR diff, phase comparison, PR metadata, and branch comparison through the GitHub connector.
- Static review confirmed Storage itself exports test_support only under cfg(test) or feature=test-support, reads no general DATABASE_URL, maps connection/setup failures to redacted errors, serializes setup/cleanup through the same transaction-scoped advisory lock, and keeps production repository/runtime behavior unchanged.
- Re-fetched changed source ranges and verified contract-valid operation ids, safe query allowlisting, partial-layout cleanup, explicit-cleanup Drop retry, base-table schema probing, and corresponding tests/docs.
- Phase comparison from code-bearing head 9571dc4deef60f4a35923139feb85fc53e94ab03 to source/docs head aa59064d641f4850f7c70fa615e638b52613dd95 showed worker changes only in allowed Storage test_support and docs paths; other control-slot changes were orchestrator-owned.
- Observed Component CI run 29124956486, run_number 1580, on source/docs head aa59064d641f4850f7c70fa615e638b52613dd95 complete successfully: cargo fmt, cargo check, cargo test, cargo clippy, and diagnostics finalization all passed.
checks_not_run:
- local shell cargo commands because repository work is connector-only
- a standalone no-feature `cargo check -p haze-sync-storage --no-default-features`; workspace feature unification currently enables test-support through Server
- cargo test -p haze-sync-storage --features test-support against a live dedicated PostgreSQL test database
ci_status: CI_GREEN for Component CI run 29124956486 on the final source/docs head; green CI does not resolve the production dependency-contract violation
workflow_urls:
- prior fixer run: Component CI 29120476441, run_number 1536, conclusion success
- STOR-P9C source/docs run: Component CI 29124956486, run_number 1580, conclusion success
known_failures: none on the final Storage source/docs head

CI_DIAGNOSTICS:
artifact_based_logs: no; active role is clean-code-reviewer and the active prompt prohibits diagnostics-artifact reading
artifact_name: none
artifact_id: none
workflow_run_id: 29124956486 from workflow metadata only
workflow_run_attempt: 1
artifact_status: not applicable because the final source/docs run was green
summary_read: no
manifest_read: no
logs_read: none
raw_job_logs_used: no
diagnostics_failure: none

SAFETY_AND_SECRECY:
secrets_committed: no
unsafe_public_output: no
raw_errors_exposed: no
provider_calls_added: no
hard_delete_added: no
background_jobs_added: no

ISSUES_FOUND:
- `crates/haze-sync-server/Cargo.toml` declares `haze-sync-storage = { path = "../haze-sync-storage", features = ["test-support"] }` under normal dependencies. This makes test-only Storage APIs part of Server production compilation and violates the accepted Storage production-isolation invariant.
- The correct dependency placement is outside the active Storage allowed files and cannot be changed by this worker.
- Optional PostgreSQL tests remain intentionally not run when the dedicated variable is absent; mandatory tests must use the required/prepare APIs.
- Live PostgreSQL roundtrips were not executed because no dedicated database is available through the connector workflow.
- Branch remains diverged from main: main head c1e69a664388b0cba028170e8398b9088218957d, merge base 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2, component branch ahead 218 and behind 12 before report write.
- The final report-only commit uses [skip ci] and is not CI evidence.

BLOCKERS:
- Cross-component Server dependency correction required to remove Storage `test-support` from the production dependency graph

NEXT_RECOMMENDED_AGENT:
orchestrator, then the Server component owner for the dependency-only correction and workspace CI rerun

FINAL_VERDICT:
CLEAN_BLOCKED_BY_CONTRACT. STOR-P9 test-support code is locally hardened, documented, and green in Component CI run 29124956486. However, the accepted production-isolation contract is currently violated because Server enables `test-support` in its normal dependency declaration. That sibling dependency edit is outside this review's allowed scope, so clean acceptance must wait for orchestrator-directed Server correction and CI verification. The report-only commit uses CI skip and must not be treated as CI evidence.

PUSHED:
yes
