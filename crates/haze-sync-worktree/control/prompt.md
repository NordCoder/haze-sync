# W1-WT-P11-CLEAN-FIX — Correct lifecycle, concurrency and validated-root findings

Before starting, name this worker chat exactly:

`worktree — W1 WT-P11 Clean Review Fix`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker
Phase: WT-P11-CLEAN-FIX

Do not merge PR #49, change draft state, rewrite history, modify sibling branches, begin Server fan-in, or change API/CLI/Deployment behavior.

## Reviewed candidate and findings

- accepted WT-P10 baseline: `1942946331e8362f19907ab6ad4eb779da70fd57`;
- reviewed WT-P11 SHA: `61c24544a3fb9d785cb95ab2016f6d29f4661d3e`;
- clean-review report commit: `de1454fceed9cecc11b7ef6ebc177af35f2764a4`;
- clean-review report blob: `52e9811d1b18eb5b74bda383ea67961c16bea446`;
- review status: `CLEAN_NEEDS_FIX`;
- reviewed CI: run `29266803510`, number `1870`, success.

Correct all four routed findings without unrelated redesign.

## Required corrections

### 1. Deterministic watcher lifecycle/degradation tests

Add a minimal Worktree-owned internal test seam or deterministic event injection that does not expose paths publicly and does not weaken production encapsulation.

Prove deterministically:

- bounded-channel overflow/coalescing produces a coarse path-free hint and preserves full-scan fallback;
- backend callback failure / backend closure map to coarse typed failure/closed outcomes;
- repeated shutdown and Drop are safe and release backend ownership;
- public hints and Debug/errors remain path-free and root-redacted.

Do not rely only on timing-dependent real filesystem events.

### 2. Host-visible concurrent manual request semantics

The WT-P11 contract requires an externally reachable typed `Busy` outcome. Do not remove that requirement.

Provide a bounded Worktree-owned host-facing manual request handle/channel or equivalent scheduler boundary that:

- allows a host to submit a manual request while another cycle is pending;
- returns typed coarse `Busy`, `NotStarted`, `Cancelling`, `Shutdown`, accepted/completed outcomes;
- preserves at-most-one-cycle and all Worktree-owned accounting;
- never lets Server call the executor directly;
- has bounded capacity and explicit request/response lifecycle;
- introduces no detached unmanaged runtime/task and no public HTTP/DTO semantics.

The runtime service remains the sole owner of executor, cancellation, lifecycle, counters and last-cycle status.

Add pending-future tests proving concurrent manual request rejection/Busy, cancellation, no overlap and accounting after completion/drop.

### 3. Automatic/manual compatibility matrix

Add focused interaction tests proving manual requests do not consume or advance:

- startup pending state;
- watcher hint/debounce state;
- next periodic deadline.

Also prove automatic startup, periodic and watcher-triggered cycles retain prior behavior after manual scheduling changes.

### 4. Validated Worktree root construction

Do not allow production watcher construction from an arbitrary merely-absolute `PathBuf`.

Construct it from an existing validated Worktree-owned root/config capability, or add a narrow validated factory that reuses established root safety validation. Preserve root redaction in Debug/errors.

## Scope

Allowed:

- Worktree runtime/watcher modules, exports, focused tests and directly required docs/manifest changes;
- Worktree control report.

Forbidden:

- Server, Storage, Core, API, CLI, Deployment product files;
- migrations/schema/workflows/sibling control files;
- unrelated refactors, provider behavior, hard delete or destructive repair;
- detached tasks, hidden runtimes, internal HTTP or raw path exposure.

## Completion

Create a real code-bearing fix commit without CI skip. Obtain authoritative Component CI on the exact final code-bearing SHA.

Write `crates/haze-sync-worktree/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: WT-P11-CLEAN-FIX`;
- `chat_name: worktree — W1 WT-P11 Clean Review Fix`;
- honest status `FIX_COMPLETE`, `FIX_NEEDS_MORE_WORK`, `FIX_BLOCKED_BY_CONTRACT`, `FIX_BLOCKED_BY_SCOPE`, or `FIX_BLOCKED_BY_TOOLING`.

Record each finding and correction, changed paths, concurrency/request-handle design, deterministic test seam, validated-root boundary, exact final SHA and exact CI evidence.

Do not claim `CLEAN_ACCEPT`, begin Server fan-in or reactivate SRV-P7B4. A repeated mandatory Worktree clean review is required.
