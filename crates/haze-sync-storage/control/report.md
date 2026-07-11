REPORT_TYPE:
CLEAN_CODE_REVIEW

STATUS:
CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
agent_execution_id: W1-STOR-P10-CLEAN-storage-review
chat_name: storage — W1 STOR-P10 Clean-Code Review

COMPONENT:
name: storage
path: crates/haze-sync-storage
branch: component/storage
contract_path: crates/haze-sync-storage/docs/component-contract.md
plan_path: crates/haze-sync-storage/docs/stor-p10-implementation-plan.md
dependency_map_path: crates/haze-sync-storage/docs/dependency-map.md
control_prompt_path: crates/haze-sync-storage/control/prompt.md
control_report_path: crates/haze-sync-storage/control/report.md

WAVE:
id: W1
phase_id: STOR-P10-CLEAN
dependency_status: active control state was PROMPT_READY; active role was clean-code-reviewer; STOR-P10 implementation and artifact correction were complete; helper synchronization merge c62112375793002392851403299b09988e27212f was present; accepted candidate 2d2ffe03a0331f5e1f3b5cf3508fb9459c84daf2 had authoritative ordinary plus strict PostgreSQL CI success in run 29171728288

SUMMARY:
Reviewed the complete STOR-P10 range from accepted pre-phase baseline aa59064d641f4850f7c70fa615e638b52613dd95 through synchronized candidate 2d2ffe03a0331f5e1f3b5cf3508fb9459c84daf2. Migration 0010, versioned Worktree instance/path-state repositories, exact-contiguous cursor primitives, schema/test-support changes, live PostgreSQL tests, documentation, branch synchronization and the Storage-only PostgreSQL workflow are architecturally clean and preserve Storage ownership. One material evidence gap was found: the existing live migration test rejected non-empty legacy state through the test-support preflight and therefore did not directly execute migration 0010 SQL against a non-empty legacy table. Added a focused feature-gated ignored integration test that executes migration 0010 directly inside a savepoint, verifies the exact explicit guard error, rolls back to the savepoint, proves the legacy row and exact legacy table shape remain intact, and proves worktree_instances was not created. Extended Component CI evidence enforcement to require this fifth test and updated the runbook. On correction head a6f1edf48d23c767f0f9b34ab28aacd8bd586000, Storage PostgreSQL verification completed successfully, including readiness, the exact ignored-test command, all five mandatory evidence checks and diagnostics finalization. The Rust workspace job reported success for its individual aggregate-wrapper steps but failed at Finalize CI diagnostics, which means at least one underlying check marker exists. Clean-code role did not inspect the diagnostics artifact. Exact correction therefore requires artifact-based fixer triage before clean acceptance.

CHANGED_FILES:
- crates/haze-sync-storage/tests/stor_p10_migration_guard.rs
- .github/workflows/component-ci.yml
- crates/haze-sync-storage/docs/test-support.md
- crates/haze-sync-storage/control/report.md

REVIEW_RANGE:
accepted_pre_phase_sha: aa59064d641f4850f7c70fa615e638b52613dd95
initial_stor_p10_sha: 63d80764933cba5f23fb43bad44201a75e1dc16a
post_fix_storage_sha: abca69058390983894465cef9d66c38960fae4c7
helper_sync_merge_commit: c62112375793002392851403299b09988e27212f
initial_review_candidate_sha: 2d2ffe03a0331f5e1f3b5cf3508fb9459c84daf2
clean_review_correction_sha: a6f1edf48d23c767f0f9b34ab28aacd8bd586000
final_branch_head_before_report: a6f1edf48d23c767f0f9b34ab28aacd8bd586000
base_branch: main
base_sha: c1e69a664388b0cba028170e8398b9088218957d
pr_number: 47
pr_state: open, draft, unmerged, mergeable
pr_merge_commit_sha_at_correction_head: ac39f9c44dbddd9329e6ab6b0ea5c79c0eb13a80

BRANCH_AND_CONTROL:
current_branch: component/storage
default_branch_modified: no
sibling_branch_modified: no
history_rewritten: no
force_push_used: no
pr_draft_state_changed: no
control_prompt_read: crates/haze-sync-storage/control/prompt.md
control_report_written: crates/haze-sync-storage/control/report.md
control_state_modified_by_worker: no
control_files_archived_by_worker: no
ci_skip_used: yes for the final report-only commit only
ci_skip_reason: final commit creates only crates/haze-sync-storage/control/report.md; test, workflow and documentation correction commits used normal CI

SCOPE:
allowed_files_only: yes
scope_expansion_used: no
scope_expansion_rationale: none
cross_component_changes: none
forbidden_files_touched: none

CONTRACT:
contract_read: yes
contract_satisfied: yes for reviewed Storage implementation and focused correction; lifecycle clean acceptance remains pending artifact-based CI correction
contract_changes_requested: none
contract_change_rationale: none
affected_components: Storage test evidence and Storage-only Component CI gate

MIGRATION_REVIEW:
- Migration 0010 checks for any legacy worktree_state row before drop table.
- The guard raises a fixed operator-actionable message and PostgreSQL transactional behavior prevents subsequent destructive statements from committing.
- No default adapter identity or root is invented.
- worktree_instances stores only canonical sha256 root fingerprints with state_format_version = 1.
- Replacement worktree_state is keyed by adapter_id and path and enforces present/tombstoned hash and observation invariants.
- Adapter cursor non-negativity is enforced at the schema level.
- Test-support accepts only fresh, exact pre-P10-empty and exact current schemas; altered/partial schemas fail closed.
- Direct SQL guard evidence was missing from the original candidate and was added during review.

REPOSITORY_REVIEW:
- bind_or_verify_worktree_instance is create-or-verify and returns a redacted mismatch without rendering the supplied or persisted fingerprint.
- Worktree instance and path rows are validated on database read using Common identifier/path/hash types and supported format versions.
- Path-state writes are passive caller-decided facts and use caller-provided executors; no pool, runtime, filesystem or policy ownership was introduced.
- Present state requires a canonical content hash; tombstoned state clears content and observations.
- Reconciliation observations are revision/hash guarded and stale facts update no row.
- Snapshot reads are adapter-scoped, bounded to MAX_CHANGES_LIMIT, fetch limit + 1, order by path ascending and use an exclusive normalized path cursor.
- No hard-delete Worktree repository operation exists.
- Repository errors discard SQLx details and use path/fingerprint/cursor-safe messages.

CURSOR_AND_TRANSACTION_REVIEW:
- initialize_and_lock and lock_current operate inside a caller-owned PostgreSQL transaction.
- SELECT FOR UPDATE serializes competing exact advances.
- advance_exact_contiguous validates nonnegative values, checked-add overflow, regression/equality, gaps, missing rows and stale expected values.
- The update is compare-and-set on adapter_id plus expected last_core_seq and returns no raw external cursor.
- Rollback tests prove an advanced sequence is not persisted after rollback.
- Concurrent winner/loser PostgreSQL evidence proves the loser observes CursorStaleExpected after the winner commits.
- Path-state writes use caller-owned executors and live tests prove rollback removes uncommitted path claims.
- Storage does not claim filesystem materialization success or choose checkpoint timing; Server remains responsible for composing state plus cursor in one transaction.

SECRECY_AND_VISIBILITY_REVIEW:
- WorktreeInstanceBinding and WorktreeInstanceRow custom Debug implementations redact root fingerprints.
- Worktree root fingerprints are excluded from serde/public-status surfaces.
- Raw external cursor JSON remains in internal rows; AdapterCursorSummary exposes only presence.
- Repository/TestSupport errors do not retain raw SQLx/database details, URLs, credentials or local paths.
- Dedicated database URL parsing ignores DATABASE_URL, requires a test marker, rejects production markers and target/session-changing query parameters, and redacts Display/Debug.
- CI credentials are synthetic values scoped to an ephemeral localhost PostgreSQL service; no repository secret or external managed database is used.
- Final synchronized branch contains current-main CI infrastructure and no temporary contents: write synchronization workflow.

TEST_QUALITY_REVIEW:
original_strengths:
- Pure tests cover unsupported versions, overflow, invalid row shapes, path/identifier/hash validation, bounded SQL and safe error rendering.
- Live path-state test covers bind replay/mismatch, two-adapter isolation, present/tombstoned state, two-page deterministic snapshots, stale and successful observation updates, commit and rollback.
- Live cursor test covers initialization, missing cursor, regression, gap, successful advance, stale expected value, rollback and concurrent winner/loser behavior.
- Test-support live tests cover fresh/current idempotence and exact pre-P10 migration behavior.
- Component CI uses strict required database setup, serial execution, bounded readiness, exact command evidence and diagnostics finalization.
material_gap_found:
- Non-empty legacy rejection in the existing live test stopped at prepare_storage_schema preflight and did not execute the SQL migration guard itself.
correction:
- Added migration_0010_sql_guard_preserves_nonempty_legacy_state as a feature-gated ignored integration test.
- The test executes migrations 0001 through 0009, inserts a legacy row, creates a savepoint, executes migration 0010 directly, verifies the exact database guard message, rolls back to the savepoint, checks the row and seven legacy columns remain, checks worktree_instances does not exist, then rolls back the outer transaction.
- Component CI now requires explicit successful log evidence for this fifth test.
assertions_weakened_or_tests_deleted: no

WORKFLOW_REVIEW:
- permissions remain contents: read.
- pull_request main trigger, workflow_dispatch, concurrency and ordinary Rust job are preserved.
- Storage PostgreSQL job runs only for component/storage PR/dispatch context.
- postgres:16-alpine uses synthetic credentials and a dedicated test-named database.
- service health check plus bounded explicit pg_isready prevents unbounded waiting.
- RUST_TEST_THREADS=1 serializes destructive shared-database harness operations.
- exact strict command remains cargo test -p haze-sync-storage --features test-support -- --ignored.
- evidence validation now requires five exact successful test names and fails on missing/renamed/skipped evidence.
- both jobs use diagnostics finalizers; failure artifacts remain job-specific.
- no temporary write-enabled workflow is present.

CORRECTIONS:
completed: yes
product_behavior_changed: no
migration_sql_changed: no
repository_api_changed: no
test_evidence_strengthened: yes
workflow_evidence_strengthened: yes
documentation_updated: yes

TESTS_AND_CHECKS:
accepted_baseline_ci:
- Component CI run 29171728288, run number 1818, SHA 2d2ffe03a0331f5e1f3b5cf3508fb9459c84daf2, conclusion success.
- Rust workspace job succeeded: fmt/check/workspace tests/clippy/finalizer.
- Storage PostgreSQL verification succeeded: readiness, strict ignored command, four original evidence tests and finalizer.
correction_ci:
- Component CI run 29172303439, run number 1825, attempt 1, SHA a6f1edf48d23c767f0f9b34ab28aacd8bd586000, conclusion failure.
- Storage PostgreSQL verification job succeeded completely.
- Strict command executed successfully: cargo test -p haze-sync-storage --features test-support -- --ignored.
- Evidence validation succeeded for all five required tests, including migration_0010_sql_guard_preserves_nonempty_legacy_state.
- PostgreSQL diagnostics finalizer succeeded.
- Rust workspace step summaries showed cargo fmt/check/test/clippy completed with success under the aggregate diagnostics wrapper.
- Rust workspace Finalize CI diagnostics failed and uploaded a diagnostics artifact; therefore an underlying failed-check marker exists and individual step summaries are not sufficient green evidence.
checks_not_run_locally:
- all repository shell commands, because this worker is GitHub-connector-only.
ci_status: CI_RED on correction head; exact failed check requires fixer artifact inspection
known_failures:
- Rust workspace Finalize CI diagnostics failed in run 29172303439.

CI_DIAGNOSTICS:
artifact_based_logs: not read; clean-code reviewer role and active prompt prohibit diagnostics artifact inspection
artifact_name: ci-diag__component-storage__wf-component-ci__run-29172303439__attempt-1
artifact_id: 8253874582
artifact_digest: sha256:41c87f30baa40ab384db02ff7884895d2727a94dcb50ea4e5b9b410fd467bf34
artifact_size_bytes: 1824
artifact_expired: false
artifact_expires_at: 2026-07-12T23:34:59Z
workflow_run_id: 29172303439
workflow_run_number: 1825
workflow_run_attempt: 1
artifact_head_sha: a6f1edf48d23c767f0f9b34ab28aacd8bd586000
summary_read: no
manifest_read: no
logs_read: none
raw_job_logs_used: no
next_diagnostics_action: Orchestrator should assign fixer-worker with this exact artifact; fixer must read summary, manifest and every failed-check log before correcting

SAFETY_AND_SECRECY:
secrets_committed: no
unsafe_public_output: no
raw_errors_exposed: no
raw_root_or_fingerprint_exposed: no
production_credentials_added: no
provider_calls_added: no
hard_delete_added: no
background_jobs_added: no

ISSUES_FOUND:
- Original strict evidence did not directly execute migration 0010 SQL against a non-empty legacy table; corrected with a direct savepoint-backed integration test and mandatory evidence grep.
- Correction CI is red at the Rust diagnostics finalizer. Exact failed check is intentionally unknown in this role until the artifact is read by a fixer.
- Current report-only commit uses [skip ci] and is not CI evidence.

BLOCKERS:
- CLEAN_NEEDS_FIX: correction head a6f1edf48d23c767f0f9b34ab28aacd8bd586000 has a red Rust workspace diagnostics finalizer in run 29172303439.
- Mandatory clean acceptance requires a post-fix Component CI run where both Rust workspace and Storage PostgreSQL verification are fully green.

NEXT_RECOMMENDED_AGENT:
fixer-worker after Orchestrator records artifact 8253874582 in a precise fixer prompt

FINAL_VERDICT:
CLEAN_NEEDS_FIX. The STOR-P10 architecture, migration semantics, repository boundaries, cursor transaction behavior, secrecy and live PostgreSQL behavior are clean. Review strengthened a real migration-evidence gap by directly executing the SQL guard and requiring that result in CI; the PostgreSQL job and all five mandatory evidence checks are green. The correction cannot be clean-accepted because the Rust workspace diagnostics finalizer is red. Route artifact 8253874582 to a fixer, obtain both jobs green, then return for final clean acceptance. STOR-P10 is not yet accepted for SRV-P7B3 SHA synchronization.

PUSHED:
yes
