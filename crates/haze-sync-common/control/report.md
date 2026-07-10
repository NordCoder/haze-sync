REPORT_TYPE:
FIX

STATUS:
FIX_COMPLETE

AGENT:
role: fixer-worker
agent_execution_id: W1-FIX-CMM-P6C-CI-common-fixture-review-format
chat_name: common — W1 FIX-CMM-P6C-CI

COMPONENT:
name: common
path: crates/haze-sync-common
branch: component/common
contract_path: crates/haze-sync-common/docs/component-contract.md
plan_path: crates/haze-sync-common/docs/implementation-plan.md
dependency_map_path: crates/haze-sync-common/docs/dependency-map.md
control_prompt_path: crates/haze-sync-common/control/prompt.md
control_report_path: crates/haze-sync-common/control/report.md

WAVE:
id: W1
phase_id: FIX-CMM-P6C-CI
dependency_status: CMM-P6 implementation, initial formatting fixer, and CMM-P6C clean-code review complete; Component CI run 29084340110 failed; diagnostics artifact 8224118461 identified a single rust-fmt failure

SUMMARY:
Downloaded and read the CMM-P6C diagnostics artifact. The only failed check was `rust-fmt` from `cargo fmt --all --check`. The artifact-provided diff required the `assert_complete_unique_wires` function signature in `tests/compatibility_fixtures.rs` to be formatted on one line. Applied exactly that formatting-only correction. Strict fixture schema validation, complete unique unordered vocabulary checks, JSON-safe roundtrip assertions, fixture values, documentation, production Common behavior, dependencies, and coverage remain unchanged. A new Component CI run is in progress for the fixer code-bearing commit.

CHANGED_FILES:
- crates/haze-sync-common/tests/compatibility_fixtures.rs
- crates/haze-sync-common/control/report.md

BRANCH_AND_CONTROL:
current_branch: component/common
base_branch: main
base_sha: PR metadata reports base_sha 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2; compare_commits reports current main c1e69a664388b0cba028170e8398b9088218957d with merge base 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2
head_sha: a3941f35bac9544bf55be608010da6f3d1e6ac94 before this report-only commit; final branch head is the report update commit returned by GitHub contents API
default_branch_modified: no
sibling_branch_modified: no
control_prompt_read: yes
control_report_written: yes
control_files_archived_by_worker: no
ci_skip_used: yes
ci_skip_reason: final commit updates only crates/haze-sync-common/control/report.md after the formatting fix was committed without CI skip; skipped report-only workflow is not CI evidence

SCOPE:
allowed_files_only: yes
scope_expansion_used: no
scope_expansion_rationale: none
cross_component_changes: none
forbidden_files_touched: no

CONTRACT:
contract_read: yes
contract_satisfied: yes
contract_changes_requested: none
contract_change_rationale: none
affected_components: none

IMPLEMENTATION_OR_REVIEW:
completed: yes
main_changes:
- read Project Sources for implementation manifest, report template, fixer-worker prompt, GitHub connector protocol, and wave-plan background
- reloaded active common state and FIX-CMM-P6C-CI prompt from component/common
- read the previous CMM-P6C clean-code report, component contract, implementation plan, implementation log, dependency map, compatibility fixture documentation, and current fixture integration test
- downloaded diagnostics artifact 8224118461 named ci-diag__component-common__wf-component-ci__run-29084340110__attempt-1
- read summary.md, manifest.json, failures/rust-fmt.txt, and logs/rust-fmt.log
- verified artifact schema, component, branch, run id, attempt, head SHA, failed check, command, exit code, and referenced paths were mutually consistent
- observed that the ZIP files were packaged at the archive root rather than under ci-diagnostics; all required files were present and readable
- confirmed the only artifact-proven failure was rustfmt layout of the `assert_complete_unique_wires` signature
- changed only that signature layout to the exact artifact-provided format
- re-read the formatted source lines and confirmed the correction matches the rustfmt diff
- inspected PR #46 metadata and branch comparison after the fix
- observed PR #46 open/draft and mergeable true at fixer source head a3941f35bac9544bf55be608010da6f3d1e6ac94 before report write
- observed Component CI run 29086420405 in_progress for fixer source commit a3941f35bac9544bf55be608010da6f3d1e6ac94
behavior_changes: none; formatting-only function-signature layout
bugs_found: no product defect; CI formatting mismatch only
bugs_fixed: corrected the artifact-proven rustfmt layout in the compatibility fixture integration test
cleanups_made: none beyond artifact-required formatting
non_goals_preserved: no TypeScript edits, generated client pipeline, API DTO ownership, Core policy, runtime/provider behavior, workflow/dependency changes, sibling changes, fixture value changes, fixture coverage removal, test deletion, or assertion weakening
deferred_work: observe the new Component CI conclusion; if red, a future fixer prompt must use that run's diagnostics artifact

TESTS_AND_CHECKS:
checks_run:
- GitHub connector read of crates/haze-sync-common/control/state.md
- GitHub connector read of crates/haze-sync-common/control/prompt.md
- GitHub connector read of previous crates/haze-sync-common/control/report.md
- GitHub connector download of diagnostics artifact 8224118461
- artifact read of summary.md
- artifact read of manifest.json
- artifact read of failures/rust-fmt.txt
- artifact read of logs/rust-fmt.log
- GitHub connector read of fixed crates/haze-sync-common/tests/compatibility_fixtures.rs around the corrected signature
- GitHub connector read of current component contract, implementation plan, implementation log, dependency map, and compatibility fixture documentation
- GitHub connector get_pr_info for PR #46; observed mergeable true at fixer source head before report write
- GitHub connector compare_commits for main...component/common and CMM-P6C-source...fixer-source context
- GitHub connector fetch_commit_workflow_runs for fixer source commit a3941f35bac9544bf55be608010da6f3d1e6ac94; observed Component CI run 29086420405 in_progress
checks_not_run:
- cargo fmt --all --check locally: not run because repository work is restricted to the GitHub connector; exact formatter output came from the diagnostics artifact
- cargo check -p haze-sync-common locally: not run because repository work is restricted to the GitHub connector
- cargo test -p haze-sync-common locally: not run because repository work is restricted to the GitHub connector
- cargo clippy -p haze-sync-common --all-targets -- -D warnings locally: not run because repository work is restricted to the GitHub connector
ci_status: CI_PENDING
workflow_urls: Component CI run 29086420405 in_progress for fixer source commit a3941f35bac9544bf55be608010da6f3d1e6ac94 before this report-only commit
known_failures: original run 29084340110 failed only rust-fmt according to artifact; no new failure observed before report write

CI_DIAGNOSTICS:
artifact_based_logs: yes
artifact_name: ci-diag__component-common__wf-component-ci__run-29084340110__attempt-1
artifact_id: 8224118461
workflow_run_id: 29084340110
workflow_run_attempt: 1
artifact_status: downloaded, extracted, and required contents readable; files packaged at ZIP root rather than under ci-diagnostics
summary_read: yes
manifest_read: yes
logs_read: failures/rust-fmt.txt; logs/rust-fmt.log
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
- The diagnostics ZIP flattened the expected ci-diagnostics directory, but contained complete and mutually consistent required files; this did not block diagnosis.
- New Component CI run 29086420405 was still in progress when this report was written.
- Local shell repository checks were not run because repository operations are restricted to the GitHub connector.
- The final report-only commit uses `[skip ci]`; it is not CI evidence. The relevant post-fix CI evidence is run 29086420405 for commit a3941f35bac9544bf55be608010da6f3d1e6ac94.
- component/common remains diverged from current main by normal commit graph, but PR metadata reports mergeable true; no merge/rebase/reset was performed.

BLOCKERS:
none

NEXT_RECOMMENDED_AGENT:
orchestrator

FINAL_VERDICT:
FIX_COMPLETE — the only artifact-proven CMM-P6C CI failure was rustfmt formatting of one helper signature, and the exact formatting correction was applied without changing behavior, fixture values, strict validation, coverage, or documentation. Post-fix Component CI is pending. The final report-only commit intentionally uses CI skip and is not CI evidence.

PUSHED:
yes
