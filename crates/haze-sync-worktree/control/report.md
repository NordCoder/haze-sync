REPORT_TYPE: CLEAN_CODE_REVIEW

STATUS: CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
agent_execution_id: not provided
chat_name: worktree — W1 WT-P11 Hosted Runtime Contract Clean Review

COMPONENT:
name: worktree
path: crates/haze-sync-worktree
branch: component/worktree
contract_path: crates/haze-sync-worktree/docs/component-contract.md
plan_path: crates/haze-sync-worktree/docs/implementation-plan.md
dependency_map_path: crates/haze-sync-worktree/docs/dependency-map.md
control_prompt_path: crates/haze-sync-worktree/control/prompt.md
control_report_path: crates/haze-sync-worktree/control/report.md

WAVE:
id: W1
phase_id: WT-P11-CLEAN
dependency_status: Server SRV-P7B4 remains blocked pending corrected WT-P11 candidate, green exact-SHA CI and a repeated clean review

REVIEW_TARGET:
accepted_baseline_sha: 1942946331e8362f19907ab6ad4eb779da70fd57
reviewed_code_bearing_sha: 61c24544a3fb9d785cb95ab2016f6d29f4661d3e
review_range: 1942946331e8362f19907ab6ad4eb779da70fd57..61c24544a3fb9d785cb95ab2016f6d29f4661d3e
fix_report_commit: 21ac67c51bb2286b6fe26b890658f64320244067
post_candidate_changes: control/report-only; no later product or tooling commit invalidated the reviewed SHA

SUMMARY:
The WT-P11 implementation compiles, passes its current tests and preserves the major Worktree ownership boundaries, but the candidate cannot be clean-accepted because required lifecycle and concurrency guarantees are not honestly verified and one advertised typed outcome is not externally reachable through the current exclusive-borrow API. No product correction was made during this review; exact findings are routed for a focused Worktree-owned fix.

CHANGED_FILES:
- crates/haze-sync-worktree/control/report.md

BRANCH_AND_CONTROL:
current_branch: component/worktree
base_branch: main
head_sha: 61c24544a3fb9d785cb95ab2016f6d29f4661d3e
default_branch_modified: no
sibling_branch_modified: no
control_prompt_read: yes
control_report_written: yes
control_files_archived_by_worker: no
ci_skip_used: yes
ci_skip_reason: review report-only commit; no code-bearing correction was made

SCOPE:
allowed_files_only: yes
scope_expansion_used: no
cross_component_changes: none
forbidden_files_touched: none

REVIEW_FINDINGS:

1. HIGH — Required watcher degradation behavior is not tested.
Evidence:
- `wt_p11_tests.rs` contains one production watcher happy-path test only: create/start, one file write, one hint, shutdown and redacted Debug.
- There is no focused test for bounded-channel overflow/coalescing, backend callback failure, backend closure, repeated shutdown/drop, or fallback-to-authoritative-full-scan behavior.
Impact:
- The core production boundary introduced by WT-P11 is accepted only by inspection and one timing-dependent filesystem event test.
- Regressions in overflow/failure/closure handling can pass CI while violating the explicit phase contract.
Required correction:
- Introduce a minimal Worktree-owned testable producer/backend seam or deterministic internal event injection that does not expose paths publicly.
- Add deterministic tests for overflow coalescing, callback/backend failure, closure, shutdown/drop and path-free output.

2. HIGH — Manual busy outcome is effectively unreachable and no-overlap is not tested at the new public entrypoint.
Evidence:
- `run_manual_cycle(&mut self, ...)` owns an exclusive mutable borrow for the entire returned future.
- While either `poll()` or `run_manual_cycle()` is in flight, safe callers cannot obtain another mutable borrow to invoke `run_manual_cycle()` and observe `WorktreeRuntimeManualOutcome::Busy`.
- The `cycle_in_progress` check therefore does not provide the advertised host-visible busy response in normal safe use.
- WT-P11 tests use only immediately-ready cycle futures and contain no pending manual-cycle/no-overlap test.
Impact:
- The public contract advertises a typed `Busy` state that the host cannot practically receive.
- The new entrypoint's no-overlap and cancellation behavior is not directly proven; older automatic-poll tests do not cover manual execution.
Required correction:
- Either define and document exclusive-borrow serialization as the intended no-overlap contract and remove the unreachable Busy outcome/requirement, which may require Orchestrator contract approval, or provide a bounded host-facing scheduler/request handle through which concurrent manual requests can receive typed Busy without bypassing Worktree accounting.
- Add pending-future tests proving manual no-overlap, cancellation and accounting on completion/drop.

3. MEDIUM — Required compatibility matrix is not explicitly covered by WT-P11 tests.
Evidence:
- The new focused tests cover manual success, DryRun and basic lifecycle states.
- They do not exercise automatic startup, periodic or watcher-triggered behavior after the runtime refactor.
- Existing WT-P10 tests remain green, but no focused regression test combines the new manual entrypoint with pending automatic scheduling to prove that a manual cycle does not consume startup/watcher state or advance periodic scheduling.
Impact:
- A future change to shared `run_one_cycle`/schedule accounting can break automatic behavior without a phase-local test identifying the interaction.
Required correction:
- Add focused interaction tests showing manual cycles leave startup pending state, watcher hints/debounce and next periodic deadline unchanged, while automatic cycles retain prior behavior.

4. MEDIUM — Production watcher construction accepts an arbitrary absolute path rather than a validated Worktree root object.
Evidence:
- `ProductionWorktreeWatcher::new(root: PathBuf, capacity)` checks only `root.is_absolute()` and non-zero capacity.
- Existing Worktree architecture requires strict configured-root validation and safe parent/root handling.
Impact:
- A Server host can instantiate the production watcher on any absolute path without passing through `WorktreeConfig` validation, weakening the component boundary even though hints remain path-free.
Required correction:
- Construct the watcher from a validated Worktree-owned root/config capability, or add an explicit validated factory that reuses existing Worktree root safety checks. Do not expose the absolute root in errors or Debug.

POSITIVE_FINDINGS:
- `notify` is a focused and reasonable backend dependency for a production filesystem watcher.
- The callback emits only unit signals into a bounded synchronous channel; public hints expose only a monotonic sequence.
- Debug output redacts the configured root.
- The watcher handle owns the notify backend and releases it on shutdown/drop; no explicit detached Worktree task or nested async runtime was introduced.
- Manual cycles use the same executor, cancellation token, summary validation and last-cycle accounting path as automatic cycles.
- DryRun forces full scan and disables import/export mutation flags.
- No Server, Storage, Core, API, CLI, Deployment, migration or workflow product files changed.

TESTS_AND_CHECKS:
checks_reviewed:
- authoritative Component CI run 29266803510 on exact SHA 61c24544a3fb9d785cb95ab2016f6d29f4661d3e
- cargo fmt: success
- cargo check: success
- cargo test: success
- cargo clippy: success
- diagnostics finalizer: success
additional_checks_run: static review of exact candidate source/tests and baseline comparison
ci_status: CI_GREEN_FOR_REVIEWED_SHA
new_code_bearing_sha: none
new_ci_required_for_this_report: no

SAFETY_AND_SECRECY:
secrets_committed: no
unsafe_public_output_found: no
raw_backend_errors_exposed: no
absolute_root_exposed_in_debug: no
provider_calls_added: no
hard_delete_added: no
unmanaged_runtime_or explicit detached task added: no

CORRECTIONS_MADE:
none; findings require a focused implementation/fixer slot and potentially an Orchestrator decision on Busy semantics

BLOCKERS:
- Deterministic watcher failure/overflow/lifecycle coverage is missing.
- Manual Busy semantics are inconsistent with the exclusive mutable-borrow API and are untested.
- Production watcher root construction does not require the existing validated Worktree configuration boundary.

NEXT_RECOMMENDED_AGENT:
implementation-worker or fixer-worker with a focused WT-P11 clean-review correction prompt; escalate only the Busy API choice if Orchestrator considers the host-visible Busy outcome mandatory

SERVER_FAN_IN:
may_begin: no
reason: WT-P11 is CLEAN_NEEDS_FIX; exact-SHA fan-in must wait for corrected product, green CI and repeated clean review

FINAL_VERDICT:
CLEAN_NEEDS_FIX. The reviewed candidate is green but lacks required deterministic lifecycle/concurrency evidence and exposes a production watcher constructor weaker than the established configured-root boundary. Do not begin Server fan-in.

PUSHED:
yes
