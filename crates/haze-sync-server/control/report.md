REPORT_TYPE:
CLEAN_CODE_REVIEW

STATUS:
CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
agent_execution_id: server-api-p8-http-functional-review-20260714-be2b1c1
chat_name: server — W1 API-P8 Worktree HTTP Review

COMPONENT:
name: server
path: crates/haze-sync-server
branch: component/server
contract_path: crates/haze-sync-server/docs/component-contract.md
plan_path: crates/haze-sync-server/docs/implementation-plan.md
dependency_map_path: crates/haze-sync-server/docs/dependency-map.md
control_prompt_path: crates/haze-sync-server/control/prompt.md
control_report_path: crates/haze-sync-server/control/report.md

WAVE:
id: W1
phase_id: SRV-API-P8-HTTP-FUNCTIONAL-REVIEW
dependency_status: accepted Server P7B5 and API-P8 inputs present

SUMMARY:
Exact API fan-in, weak HTTP ownership, passive GET status, authorization, strict body parsing, public mappings, secrecy, protected scope and exact-SHA CI are otherwise sound. One substantive concurrency/semantic defect remains in POST sync-once: the control returns Busy, NotStarted, Cancelling or Shutdown from a preliminary snapshot without performing the authoritative Worktree submission. Those snapshot-derived results can be stale and violate the explicit requirement that snapshot-to-submit races be resolved by WorktreeRuntimeManualSubmission.

CHANGED_FILES:
- crates/haze-sync-server/control/report.md only during review

BRANCH_AND_CONTROL:
current_branch: component/server
base_branch: main
base_sha: c1e69a664388b0cba028170e8398b9088218957d
head_sha_reviewed: be2b1c16fa6c4919d446b76b1f15dca5767b2482
default_branch_modified: no
sibling_branch_modified: no
control_prompt_read: yes
control_report_written: yes
control_files_archived_by_worker: no
ci_skip_used: yes
ci_skip_reason: review report only; no executable or validation change

SCOPE:
allowed_files_only: yes
scope_expansion_used: no
scope_expansion_rationale: none
cross_component_changes: none during review
forbidden_files_touched: none

CONTRACT:
contract_read: yes
contract_satisfied: no
contract_changes_requested: none
contract_change_rationale: existing typed Worktree submit outcomes are sufficient; this is a Server-only fix
affected_components: server only

IMPLEMENTATION_OR_REVIEW:
completed: yes
main_changes: no product changes during review
behavior_changes: none during review
bugs_found:
- ServerWorktreeHttpControl::submit_sync_once reads snapshot.manual_availability before submission
- Busy, NotStarted, Cancelling and Shutdown snapshot values return immediately without calling WorktreeRuntimeManualHandle::submit
- public response can therefore be based on stale observed state rather than the authoritative typed submission result
bugs_fixed: none; active prompt routes substantive defects to a focused Server fixer
cleanups_made: none
non_goals_preserved:
- no CLI-P6A work
- no Deployment work
- no formatting/style changes
- no API or Worktree owner semantic edits
deferred_work: focused Server sync-once race fix and re-review

POSITIVE_FINDINGS:
- accepted API-P8 pinned blobs match exact accepted SHAs
- required dto/public_contract_tests.rs dependency is exact accepted-source content
- no API control files were copied
- ServerWorktreeHttpControl contains only Weak<ServerWorktreeRuntimeHost> and a validated action budget
- route clones cannot retain strong shutdown ownership
- Arc::try_unwrap follows completed Axum serving and preserves mandatory host shutdown/join ownership
- GET status performs one bounded passive snapshot and maps every requested status field
- Disabled, Running+Busy and Failed semantic mappings are correct
- strict WorktreeSyncOnceRequest rejects unknown fields
- existing authentication gives 401/401/403 mappings
- accepted response is submission-only and ticket drop does not cancel the queued request
- no task, poller, retry, completion watcher, DB operation, filesystem operation or provider call was added
- public responses use accepted secret-safe API vocabulary

SUBSTANTIVE_FINDING:
Current ServerWorktreeHttpControl::submit_sync_once path:
1. upgrade Weak host;
2. read host.snapshot();
3. return Failed, Unavailable, NotStarted, Cancelling, Shutdown or Busy directly from snapshot;
4. call submit_manual only when snapshot says Available.

Incorrect observable cases:
- snapshot Busy, then the current cycle completes before response: endpoint returns 409 Busy although an authoritative submission could now be Accepted;
- snapshot NotStarted, then runtime starts before response: endpoint returns 503 NotStarted without checking submit;
- snapshot Cancelling or Shutdown can likewise be stale relative to the accepted gate;
- the route therefore does not perform the required authoritative submission for those typed outcomes.

This directly conflicts with active-prompt requirements that sync perform one bounded authoritative submission and that snapshot-to-submit races be resolved by the Worktree submission result.

REQUIRED_FIX:
- retain snapshot precheck only for Server-only conditions not expressible by WorktreeRuntimeManualSubmission: Failed and manual/mode Unavailable;
- for Available, Busy, NotStarted, Cancelling and Shutdown snapshot categories, call submit_manual exactly once and map its typed result;
- do not retry submission;
- preserve one bounded WorktreeRuntimeManualRequest::dry_run built from validated host budget;
- add a deterministic focused control test proving a stale Busy/lifecycle snapshot cannot bypass the authoritative submit result;
- preserve weak ownership, ticket-drop semantics, no completion wait and all accepted HTTP mappings.

TEST_GAP:
- existing tests verify public enum-to-HTTP mapping and absent-control routes
- they do not test ServerWorktreeHttpControl against a changing accepted gate state
- no test proves that Busy/NotStarted/Cancelling/Shutdown responses originate from submit rather than a stale snapshot

TESTS_AND_CHECKS:
checks_run:
- inspected exact candidate source and focused tests
- inspected accepted API DTO/request contract
- inspected accepted Worktree shared-gate, submit and ticket implementation
- reviewed authoritative GitHub Component CI metadata
- compared exact candidate with current pre-report head
checks_not_run: local shell unavailable through connector-only execution
ci_status: CI_GREEN_BUT_FUNCTIONALLY_INSUFFICIENT
workflow_urls: Component CI run 29321038276, number 1938, attempt 1
known_failures: no CI check failure; uncovered concurrency/semantic defect remains

CI_DIAGNOSTICS:
artifact_based_logs: no
artifact_name: none
artifact_id: none
workflow_run_id: 29321038276
workflow_run_attempt: 1
artifact_status: not required by active clean-review prompt
summary_read: no
manifest_read: no
logs_read: no
raw_job_logs_used: no
diagnostics_failure: none

EXACT_API_FAN_IN:
- dto/worktree.rs blob 7c592184d58c1cda98314fd0e2eae8baed1de1e8 exact
- dto/mod.rs blob 5534f79606639fb13857de0729793d9083523e03 exact
- dto/public_contract_tests.rs blob 903fea0d7cdc28a8eda8140cff23f521db58e061 exact accepted dependency
- routes/worktree.rs blob 032f43a1a513e7fb25d6103de281f7e5d8e08930 exact
- routes/mod.rs blob 439bebd09d3aa9252188d2d3eb106e65943c5846 exact
- fixture blob da0a197b1e6d5425f05cfd6fe772a4c96f6a830c exact
- compatibility test blob 968f1e9825a2c54385615bf75db54a975218fd20 exact
- contract doc blob f2c8d2d53c09a0e1ac0caf3ac8c6ee8d15754009 exact
- API control files copied: no
- accepted API semantics edited: no

CI:
workflow: Component CI
run_id: 29321038276
run_number: 1938
run_attempt: 1
head_sha: be2b1c16fa6c4919d446b76b1f15dca5767b2482
conclusion: success
db_capable: yes
cargo_fmt: success
cargo_check: success
cargo_test: success
cargo_clippy: success
diagnostics_finalizer: success
assessment: green CI does not cover stale snapshot submission bypass

LATER_COMMIT_VALIDATION:
Compared be2b1c16fa6c4919d446b76b1f15dca5767b2482..6e77a41653280093f5936138f4f524e22ad8b050. All later changes before this corrected review report are confined to Server control prompt/state/log/report history. No later product or tooling commit corrected or invalidated the candidate.

SAFETY_AND_SECRECY:
secrets_committed: no
unsafe_public_output: no
raw_errors_exposed: no
provider_calls_added: no
hard_delete_added: no
background_jobs_added: no

ISSUES_FOUND:
- stale preliminary snapshot can bypass authoritative manual submission and produce stale Busy/NotStarted/Cancelling/Shutdown public outcomes

BLOCKERS:
- POST sync-once does not satisfy authoritative submission/race-resolution requirements for four typed states
- focused concurrency/control test is missing

NEXT_RECOMMENDED_AGENT:
fixer-worker

FINAL_VERDICT:
CLEAN_NEEDS_FIX. Exact fan-in, ownership, GET status, auth, body validation, secrecy and CI are accepted, but POST sync-once can return snapshot-derived Busy/NotStarted/Cancelling/Shutdown without calling the authoritative Worktree submit API. The route result can therefore be stale and violates the pinned race-resolution requirement. CLI-P6A remains blocked.

PUSHED:
yes
