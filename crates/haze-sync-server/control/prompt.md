# W1-SRV-P7B3-EXECUTOR-CI-FIX — Fix failed exact-SHA executor review correction CI

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B3 Executor CI Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker
Phase: SRV-P7B3-EXECUTOR-CI-FIX

Work through the GitHub connector. Do not merge PR #45, change draft state, rewrite history, modify sibling branches, or begin SRV-P7B4.

## Failed candidate

Fix the exact review-correction candidate:

- implementation candidate: `a08739cc8146b4f224475c83fa0652b60782db82`;
- clean-review correction SHA: `784a45f879f13914a1732b8ea071ea8f281d6721`;
- clean-review report commit: `c00f8d4c128f6770ad3062b6d753e7310299faf0`;
- clean-review report blob: `9bafafc37d0fb789687aedb1920d8b4c52c2eb45`;
- failed Component CI run ID: `29255656933`;
- run number: `1859`;
- run attempt: `1`;
- exact head SHA: `784a45f879f13914a1732b8ea071ea8f281d6721`.

## Mandatory diagnostics artifact

Use this exact artifact:

- artifact ID: `8281161500`;
- artifact name: `ci-diag__component-server__wf-component-ci__run-29255656933__attempt-1`;
- artifact digest: `sha256:b005e355fd1cef4f4df37ee0b13e5e6156964eaa79b6fdc46bbe9a8e3a1cc2dc`;
- expired: `false` at rotation time;
- expires at: `2026-07-14T13:56:33Z`.

Download and read:

- `ci-diagnostics/summary.md`;
- `ci-diagnostics/manifest.json`;
- every file listed in `failed_checks`.

Do not infer root cause from the wrapper job summary. Raw GitHub job logs are fallback-only if the artifact is missing, corrupt or materially insufficient, and that limitation must be reported honestly.

Observed wrapper evidence only:

- cargo fmt: success;
- cargo check: success;
- cargo test: success;
- cargo clippy: success;
- diagnostics finalizer: failure;
- diagnostics artifact upload: success.

The wrapper evidence does not identify the root cause.

## Scope

Fix only the exact artifact-proven failure caused by the SRV-P7B3 clean-review correction.

Allowed:

- Server-owned executor validation code and focused tests;
- directly related Server-owned test/tooling correction proven by the artifact;
- Server control report.

Forbidden:

- accepted Worktree or Storage files;
- migrations;
- Core, API, CLI or Deployment product files;
- sibling control files or workflows;
- hosted runtime, startup/shutdown task, manual channel, public DTO/status/readiness work;
- unrelated cleanup or redesign.

Preserve the clean-review corrections unless the artifact proves they are themselves wrong:

- reject import/delete/export budgets above 1000;
- require `full_scan_required` for DryRun;
- keep DryRun non-mutating.

## Completion

Create a real code-bearing fix commit without CI skip when executable/test/tooling content changes. Run authoritative DB-capable Component CI on the exact final code-bearing SHA.

Write `crates/haze-sync-server/control/report.md` using `report-template.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: SRV-P7B3-EXECUTOR-CI-FIX`;
- `chat_name: server — W1 SRV-P7B3 Executor CI Fix`.

Use one honest status:

- `FIX_COMPLETE`;
- `FIX_NEEDS_MORE_WORK`;
- `FIX_BLOCKED_BY_LOGS`;
- `FIX_BLOCKED_BY_CONTRACT`;
- `FIX_BLOCKED_BY_SCOPE`;
- `FIX_BLOCKED_BY_TOOLING`.

The report must include artifact metadata, files read from the artifact, exact diagnosed root cause, changed paths, exact final code-bearing SHA, exact CI run evidence and whether the candidate is ready to return to mandatory clean-code review.

Do not claim `CLEAN_ACCEPT` or activate SRV-P7B4. After a successful fix, Orchestrator must rotate another clean-review slot.
