REPORT_TYPE:
FIX

STATUS:
FIX_COMPLETE

AGENT:
role: fixer-worker
agent_execution_id: cli-W1-FIX-CLI-P6A-TEST-REGRESSION
chat_name: cli — W1 CLI-P6A Test Restoration Fix

COMPONENT:
name: cli
path: crates/haze-sync-cli
branch: component/cli
control_prompt_path: crates/haze-sync-cli/control/prompt.md
control_report_path: crates/haze-sync-cli/control/report.md

WAVE:
id: W1
phase_id: FIX-CLI-P6A-TEST-REGRESSION

SUMMARY:
Restored the deleted pre-existing CLI P1-P5 parser and binary regression tests from synchronized baseline behavior, preserving all current Worktree tests. Added explicit valid-HTTP-200 Worktree status assertions for Disabled and Failed lifecycle responses with success exit classification and accepted public fields. No product behavior was changed. Initial restoration SHA 2cb3e5a9176d1073473a22459c2863f7a9b82321 passed check/test/clippy but failed rustfmt diagnostics. Read artifact 8325184539 for run 29368705987 and applied only the exact rustfmt corrections. Final code-bearing SHA d33fa105398d9731bfc1b7927e98d5d085c6fe59 passed Component CI run 29368943579, run number 1953, including fmt, check, test, clippy and diagnostics finalization.

CHANGED_FILES:
- crates/haze-sync-cli/src/commands.rs
- crates/haze-sync-cli/src/main.rs
- crates/haze-sync-cli/control/report.md

BRANCH_AND_CONTROL:
current_branch: component/cli
base_branch: main
baseline_sha: 8a3012a20440066422e7ad6c4e52d1a859b1bd51
previous_candidate_sha: 70c3567f587a249a180eb8b9abb155065d197e5c
initial_restoration_sha: 2cb3e5a9176d1073473a22459c2863f7a9b82321
final_code_bearing_sha: d33fa105398d9731bfc1b7927e98d5d085c6fe59
default_branch_modified: no
sibling_branch_modified: no
pr_48_state: open, draft, unmerged
ci_skip_used: yes, report-only commit only

RESTORED_TEST_GROUPS:
commands.rs:
- status normal parser
- status offline parser
- adapters list normal parser
- adapters list offline parser
- doctor offline/live parser
- conflicting doctor modes rejection
- root help, empty invocation and doctor help
- unscoped network argument rejection
- safe non-echoing parser errors
- all current Worktree parser/usage/forbidden-argument tests preserved
main.rs:
- status not-configured output
- status offline output
- live-success placeholder annotation guard
- adapters not-configured output
- doctor default offline output
- live doctor missing-config error
- safe parse-error output
- doctor help output
- all current Worktree no-fake-success and secrecy tests preserved

ADDED_STATUS_CASES:
- Disabled lifecycle + HTTP 200 -> CliExitCode::Success; accepted lifecycle/readiness/manual fields rendered
- Failed lifecycle + HTTP 200 -> CliExitCode::Success; accepted lifecycle/readiness/manual fields rendered
- existing Running+Busy readiness `ready` test preserved in worktree_api.rs

PRODUCT_BEHAVIOR_CHANGES:
none

API_P8_BLOB_VERIFICATION:
- crates/haze-sync-api/src/dto/worktree.rs remains 7c592184d58c1cda98314fd0e2eae8baed1de1e8
- no accepted API-P8 file was changed
all_exact: yes

CI_DIAGNOSTICS:
artifact_based_logs: yes
artifact_id: 8325184539
artifact_name: ci-diag__component-cli__wf-component-ci__run-29368705987__attempt-1
summary_read: yes
manifest_read: yes
logs_read:
- failures/rust-fmt.txt
- logs/rust-fmt.log
root_cause: rustfmt-only differences in restored commands.rs tests and one main.rs assertion layout
raw_job_logs_used: no

TESTS_AND_CHECKS:
checks_run:
- authoritative Component CI run 29368943579, run number 1953, exact SHA d33fa105398d9731bfc1b7927e98d5d085c6fe59
- cargo fmt: success
- cargo check: success
- cargo test: success
- cargo clippy: success
- diagnostics finalizer: success
ci_status: CI_GREEN
known_failures: none

SCOPE_AND_SAFETY:
allowed_files_only: yes
product_code_changed: no
accepted_api_files_changed: no
server_or_runtime_changes: no
workflow_or_migration_changes: no
lint_or_test_suppression: no
polling_retry_wait_fake_success_added: no
secrets_committed: no
unsafe_output_added: no

BLOCKERS:
none

NEXT_RECOMMENDED_AGENT:
clean-code-reviewer

FINAL_VERDICT:
FIX_COMPLETE. Required legacy regression coverage is restored, explicit Disabled and Failed HTTP-200 status success cases are covered, existing Worktree tests remain, API-P8 blobs are exact, product behavior is unchanged, and exact final-SHA Component CI is green. Route to final focused CLI-P6A review.

PUSHED:
yes
