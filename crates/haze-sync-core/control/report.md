REPORT_TYPE:
CLEAN_CODE_REVIEW

STATUS:
CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
agent_execution_id: W1-CORE-P7C
chat_name: core — W1 CORE-P7C Clean-Code Review

COMPONENT:
name: core
path: crates/haze-sync-core
branch: component/core
contract_path: crates/haze-sync-core/docs/component-contract.md
plan_path: crates/haze-sync-core/docs/implementation-plan.md
dependency_map_path: crates/haze-sync-core/docs/dependency-map.md
control_prompt_path: crates/haze-sync-core/control/prompt.md
control_report_path: crates/haze-sync-core/control/report.md

WAVE:
id: W1
phase_id: CORE-P7C
dependency_status: CORE-P7 implementation and artifact-based compatibility corrections were complete. Final pre-review source CI run 29107333698 on ae5fe8a8099973a8705af98e091b369038dbf71e was green before this clean-code review began.

SUMMARY:
Reviewed CORE-P7 passive doctor models, both compatibility fixes, accepted CLI usage, component contracts, tests, and documentation without reading CI diagnostics artifacts. Found a correctness gap in the claimed validating serde boundary: DoctorCheckResult deserialization verified message/status/check tags but did not verify that detail counts and booleans actually supported the declared status. Crafted payloads could therefore deserialize as `ok` while reporting missing blobs or invalid cursors. The restored public compatibility fields also allowed callers to mutate check IDs, messages, report summaries, or check ordering and then serialize misleading JSON. Hardened DoctorCheckResult and DoctorReport with centralized semantic validation used by both serialization and deserialization, while preserving public field syntax, fixed message strings, existing wire field names/order, and accepted CLI source compatibility. Simplified the large message/check/status match table into one macro-defined fixed vocabulary. Hardened classifiers so invalid or stale cursor facts are not hidden by an empty adapter scope and so disabled DB/object-store/GDrive/worktree checks do not retain contradictory live facts. Added regression tests for crafted false-ok payloads, impossible counts, public-field mutation, cursor anomaly precedence, disabled-fact normalization, existing safe JSON shape, CLI compatibility, and report roundtrips. Updated the doctor safety contract. Final code/docs head 08038795df8be46f3727b3b50f09b46d37ba122c passed cargo fmt, cargo check, cargo test, and cargo clippy in Component CI run 29110520533, run number 1422, but the workflow concluded failure at Finalize CI diagnostics and published artifact 8234601738. The artifact was not downloaded or read because the active role is clean-code-reviewer.

CHANGED_FILES:
- crates/haze-sync-core/src/doctor/types.rs
- crates/haze-sync-core/src/doctor/checks.rs
- crates/haze-sync-core/src/doctor/report.rs
- crates/haze-sync-core/src/doctor/tests.rs
- crates/haze-sync-core/docs/doctor-safety-reports.md
- crates/haze-sync-core/control/report.md

BRANCH_AND_CONTROL:
current_branch: component/core
base_branch: main
base_sha: PR #43 base observed as 1a82bea5c87953db378e5e03429326df38320ee8
head_sha: final code/docs head before this report-only commit was 08038795df8be46f3727b3b50f09b46d37ba122c
source_and_test_commits:
- d0714ae6abd503a38222959c218220b16b53d982
- 192445a540d21b674263cb802679ec280344ba4a
- c1361149f51741166a038b9d5b41682ee08c5944
- d1989b41a66bec9381cd1aa4d7559edc90c05cf3
- d2fe9f0d429a3a7eb41416171b9ce6392ccfdb87
documentation_commit:
- 08038795df8be46f3727b3b50f09b46d37ba122c
default_branch_modified: no
sibling_branch_modified: no
control_prompt_read: yes
control_report_written: yes
control_files_archived_by_worker: no
ci_skip_used: yes
ci_skip_reason: used [skip ci] only for this final control/report-only commit. Source, tests, and documentation commits did not skip CI. This skipped report commit is not CI evidence.

SCOPE:
allowed_files_only: yes
scope_expansion_used: no
scope_expansion_rationale: none
cross_component_changes: no
forbidden_files_touched: no

CONTRACT:
contract_read: yes
contract_satisfied: yes
contract_changes_requested: no
contract_change_rationale: strengthened the existing CORE-P7 passive/redacted/validating model without changing runtime ownership, public wire field names, fixed message text, status names, CLI behavior, or sibling contracts
affected_components: core only; accepted CLI source was read for compatibility verification but not modified

IMPLEMENTATION_OR_REVIEW:
completed: yes
main_changes:
- added semantic detail validation for DB, object-store, missing-blob, cursor, GDrive mapping, adapter-token, and worktree results
- made DoctorCheckResult serialization and deserialization reject message/check/status/detail contradictions
- made DoctorReport serialization reject mutated inconsistent summaries, invalid checks, and non-deterministic ordering
- preserved public DoctorReport summary/checks and DoctorCheckResult check_id/message compatibility fields
- preserved DoctorCheckMessage as a fixed enum with safe string dereference for CLI compatibility
- consolidated fixed message text, status, and check ownership into one macro-defined vocabulary
- changed cursor precedence so invalid/stale facts are not masked as skipped for an empty scope
- normalized unperformed live facts for disabled DB/object-store/GDrive/worktree checks
- added focused semantic serde, mutation, classifier-boundary, redaction, and roundtrip tests
- documented serialization validation and compatibility-field obligations
behavior_changes: safety hardening only. Contradictory crafted or publicly mutated doctor values now fail serialization/deserialization instead of emitting misleading JSON. Empty-scope invalid cursors are failed and stale cursors are warnings rather than skipped. Disabled checks omit facts that could only come from an unperformed live check.
bugs_found:
- detail payloads were not semantically checked against status/message, allowing false-ok or impossible results to deserialize
- public compatibility fields could be mutated and serialized without revalidation
- empty adapter scope was checked before invalid/stale cursor counts, masking anomalies as skipped
- disabled checks could preserve contradictory live facts in otherwise skipped/configuration-warning outputs
- initial report ordering validation used enum declaration order instead of stable wire-ID order; corrected before final CI
bugs_fixed: all listed clean-review findings were fixed with regression coverage
cleanups_made: reduced the large duplicated DoctorCheckMessage mapping table to a single macro source of truth; centralized result/report invariant validation; kept the responsibility split across checks, types, report, and tests
non_goals_preserved: no live checks, CLI implementation changes, HTTP policy, persistence, provider behavior, repair execution, workflow/dependency changes, sibling edits, test deletion, assertion weakening, arbitrary public diagnostic strings, raw provider data, paths, URLs, or secrets
deferred_work: fixer-worker must inspect diagnostics artifact 8234601738 for failed Component CI run 29110520533 and produce a green follow-up run; no known product-semantic issue remains from review

TESTS_AND_CHECKS:
checks_run:
- read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, and chatgpt-gh-connector.md from Project Sources
- read active control state, prompt, and previous fixer report
- read component contract, CORE-P7 implementation plan, implementation log, dependency map, passive-doctor decision, and doctor safety-report contract
- read current doctor module, types, classifiers, report aggregation, and tests
- read accepted CLI doctor consumer and preserved its direct summary/check/check_id/message access
- reviewed PR #43 metadata and component diff; PR remained open, draft, and unmerged
- did not read any CI diagnostics artifact
- observed Component CI run 29110520533 on final code/docs head 08038795df8be46f3727b3b50f09b46d37ba122c
- observed cargo fmt success
- observed cargo check success, including accepted CLI consumer compatibility
- observed cargo test success, including new semantic serde and public-mutation regressions
- observed cargo clippy success
- observed Finalize CI diagnostics failure
- observed workflow conclusion failure
- listed diagnostics artifact metadata without downloading or reading contents
checks_not_run:
- local repository cargo commands were not used as acceptance evidence because repository work is GitHub-connector-only; Component CI supplied product-check evidence
ci_status: CI_RED for Component CI run 29110520533 despite cargo fmt/check/test/clippy success; Finalize CI diagnostics failed
workflow_urls: failed Component CI run 29110520533
known_failures: Finalize CI diagnostics failed; root cause was not inspected in clean-code-reviewer role

CI_DIAGNOSTICS:
artifact_based_logs: no
artifact_name: ci-diag__component-core__wf-component-ci__run-29110520533__attempt-1
artifact_id: 8234601738
workflow_run_id: 29110520533
workflow_run_attempt: 1
artifact_status: metadata observed; artifact is available, unexpired, and expires at 2026-07-11T17:20:23Z; contents were not downloaded or read because the active prompt forbids diagnostics artifact reading
summary_read: no
manifest_read: no
logs_read: none
raw_job_logs_used: no
diagnostics_failure: not inspected; workflow metadata shows Finalize CI diagnostics failed after all Rust product checks succeeded

SAFETY_AND_SECRECY:
secrets_committed: no
unsafe_public_output: no
raw_errors_exposed: no
provider_calls_added: no
hard_delete_added: no
background_jobs_added: no

ISSUES_FOUND:
- Fixed: crafted detail payloads could contradict a declared successful status.
- Fixed: restored public compatibility fields could be mutated into misleading serializable output.
- Fixed: invalid/stale cursor facts could be hidden by the empty-scope skipped branch.
- Fixed: disabled checks could retain facts from checks that were not performed.
- Fixed during review: report order validation now uses stable wire identifiers, matching construction order.
- Remaining: Component CI run 29110520533 is formally red because Finalize CI diagnostics failed; artifact 8234601738 requires fixer-worker triage.
- This final report-only commit uses CI skip and is not CI evidence.

BLOCKERS:
Component CI is formally red until artifact-based fixer triage and a green follow-up run.

NEXT_RECOMMENDED_AGENT:
fixer-worker

FINAL_VERDICT:
CLEAN_NEEDS_FIX. CORE-P7 doctor models now enforce semantic check/detail/status/message consistency at serialization and deserialization boundaries, preserve accepted CLI source compatibility without arbitrary strings, keep aggregate ordering deterministic, and cover mutation and crafted-payload regressions. Final code/docs head 08038795df8be46f3727b3b50f09b46d37ba122c passed cargo fmt, cargo check, cargo test, and cargo clippy, but Component CI run 29110520533 concluded failure at Finalize CI diagnostics. Fixer-worker must use diagnostics artifact 8234601738 as source of truth.

PUSHED:
yes
