REPORT_TYPE:
CLEAN_CODE_REVIEW

STATUS:
CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
agent_execution_id: server-srv-p7b5-functional-review-20260713-78f4e45
chat_name: server — W1 SRV-P7B5 Status Readiness Review

COMPONENT:
name: server
path: crates/haze-sync-server
branch: component/server
control_prompt_path: crates/haze-sync-server/control/prompt.md
control_report_path: crates/haze-sync-server/control/report.md

WAVE:
id: W1
phase_id: SRV-P7B5-FUNCTIONAL-REVIEW

CANDIDATE:
accepted_srv_p7b4_sha: 55ed0d6c6ab9b78a953b954fcf5a9a68a6a708fe
final_code_bearing_sha: 78f4e4525327ff03fa1af1e8387e9e3ea07091d6
implementation_report_commit: c70b789111aab2ea4eadc4bc9d7d06b21c6b6b4a
implementation_report_blob: 1f5d044b9625fe42e87f97fc2cf37743f079ade9

REVIEW_SCOPE:
Functional verification of readiness semantics, passive snapshot behavior, manual availability, lifecycle transitions, secrecy, exact CI and protected scope. Formatting, rustfmt, naming taste and style were excluded as required.

POSITIVE_FINDINGS:
- ServerWorktreeStatusSnapshot contains only coarse mode, lifecycle, readiness, reason, count, boolean and manual-availability fields.
- Disabled maps to Ready / DisabledInert.
- Starting, Cancelling, Shutdown and Failed map deterministically to NotReady with coarse reasons.
- Running maps to Ready independently of counters and watcher-hint counts.
- Busy does not make a Running snapshot unready.
- DryRun is the only Running mode mapped to manual Available/Busy; other Running modes map to Unavailable.
- snapshot() performs no I/O, backend access, cycle trigger or manual probe submission.
- No public route, DTO, OpenAPI/readiness payload, CLI output or deployment probe was added.
- No accepted Worktree/Storage, migration/schema, sibling component or workflow source was modified by the candidate.
- Exact-SHA DB-capable Component CI is green.

SUBSTANTIVE_FINDING:
Manual availability is maintained by a second Server-owned atomic gate that is not an authoritative projection of the accepted Worktree gate and can report incorrect state.

Evidence:
- ServerWorktreeRuntimeHost::submit_manual sets the Server atomic to Busy from typed Accepted or Busy outcomes.
- run_hosted independently sets the same atomic to Busy before every WorktreeHostedRuntime::poll call, regardless of whether any manual request exists.
- run_hosted then unconditionally sets the atomic to Available after every completed poll, before interpreting whether the poll was Idle, Automatic or Manual.
- Accepted WorktreeHostedRuntime already owns the authoritative lifecycle/busy gate and releases that gate inside its in-flight guard before poll returns.

Incorrect observable cases:
1. False Busy in DryRun.
   - The Server ticker calls runtime.poll periodically even with no manual request.
   - Accepted WorktreeRuntimeService::poll returns WorktreeRuntimePoll::DryRun immediately for DryRun mode.
   - During that poll window the Server snapshot reports Busy even though no manual request is pending or executing.
   - Manual availability therefore depends on host polling timing rather than accepted manual state.

2. Lost Busy under concurrent submission.
   - Accepted hosted-runtime automatic/manual guard may release its authoritative gate immediately before runtime.poll returns.
   - A concurrent submit can then be accepted and set the Server mirror to Busy.
   - The host resumes and unconditionally stores Available after poll return, overwriting the newer accepted submission state.
   - Snapshot can therefore report Available while an accepted manual request is pending.

3. Duplicate manual accounting.
   - The Server atomic independently approximates accepted gate transitions rather than mapping an accepted read-only state.
   - This violates the phase requirement that Worktree remain owner of manual gating/accounting and that Server only map accepted internal status into Server vocabulary.

TEST_GAP:
- Current tests verify sequential Accepted -> Busy -> completion -> Available behavior.
- They do not exercise an idle DryRun ticker snapshot during poll.
- They do not exercise concurrent submission at the accepted-gate-release / Server-mirror-clear boundary.
- Therefore green tests do not establish deterministic manual availability.

REQUIRED_FIX:
- Remove unconditional Busy-before-every-poll and unconditional Available-after-every-poll state transitions.
- Make manual availability a race-safe projection of accepted typed manual state rather than a second independent gate.
- Preserve Worktree ownership of the authoritative gate and no-overlap semantics.
- A valid Server-only fix may use submission generations or another atomic protocol that cannot overwrite a newer Accepted state; if the accepted contract cannot expose sufficient state without duplication, report BLOCKED_BY_CONTRACT and route a narrow Worktree owner extension.
- Add deterministic focused tests for:
  - idle DryRun polling remains Available when no request exists;
  - accepted request remains Busy until its own completion;
  - concurrent submission during poll completion cannot be overwritten to Available;
  - Busy remains Ready;
  - shutdown/failure categories override any stale manual mirror.
- Do not add probe submissions, background tasks, pollers, public routes or DTOs.

OTHER_REVIEW_RESULTS:
readiness_mapping: accepted
counter_and_hint_semantics: accepted
passive_snapshot_read: accepted except manual-state correctness above
mode_mapping: accepted
lifecycle_reason_mapping: accepted
secrecy: accepted
public_surface_scope: accepted
protected_scope: accepted

CI:
workflow: Component CI
run_id: 29283227887
run_number: 1901
run_attempt: 1
head_sha: 78f4e4525327ff03fa1af1e8387e9e3ea07091d6
conclusion: success
db_capable: yes
cargo_fmt: success
cargo_check: success
cargo_test: success
cargo_clippy: success
diagnostics_finalizer: success
ci_assessment: green but insufficient because the manual-availability race and false-Busy timing are not covered

LATER_COMMIT_VALIDATION:
Compared 78f4e4525327ff03fa1af1e8387e9e3ea07091d6..f567a28ee2ddd5f2b62c258921b9e09ff92d6c27. All later changes before this report are confined to crates/haze-sync-server/control prompt/state/log files. No later product or tooling commit corrected or invalidated the candidate.

CODE_CHANGES_DURING_REVIEW:
none

PROTECTED_SCOPE:
accepted_worktree_modified: no
accepted_storage_modified: no
migrations_or_schema_modified: no
core_api_cli_deployment_modified: no
public_routes_dtos_openapi_or_readiness_modified: no
workflows_modified: no
sibling_branches_modified: no
pr_merged_or_draft_state_changed: no
history_rewritten: no

BLOCKERS:
- Server-owned manual availability mirror can report false Busy during idle DryRun polling
- concurrent accepted submission can be overwritten to Available by an older poll completion
- manual gate ownership/accounting is duplicated instead of authoritatively projected

NEXT_RECOMMENDED_AGENT:
fixer-worker

NEXT_GATE:
Route a focused SRV-P7B5 manual-availability fix. API-P8 remains blocked. A new exact code-bearing SHA must pass authoritative DB-capable Component CI and then receive focused functional review.

FINAL_VERDICT:
CLEAN_NEEDS_FIX. Readiness categories, secrecy, passive field reads and scope are otherwise sound, but manual availability is nondeterministic because a Server-owned mirror is toggled around every poll and can overwrite newer accepted submission state. SRV-P7B5 is not accepted.

PUSHED:
yes
