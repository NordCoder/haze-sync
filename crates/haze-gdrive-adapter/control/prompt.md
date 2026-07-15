# W1-FIX-GDA-GDA-P3-CI

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 FIX-GDA-GDA-P3 CI`

Repository: `NordCoder/haze-sync`
Component: gdrive-adapter
Path: `crates/haze-gdrive-adapter`
Branch/ref: `component/gdrive-adapter`
PR: #50
Role: fixer-worker
Phase: `FIX-GDA-GDA-P3-CI`

This is the artifact-first fixer loop for the existing `GDA-GDA-P3-LIVE-GOOGLE-OAUTH` phase. Do not repeat implementation or begin another GDrive product phase.

Failing candidate:
- code-bearing SHA: `6044c6d2cb160250a890415c0ff0d2785be6a981`;
- Component CI run: `29446145224`, run number `2022`, attempt `1`;
- cargo fmt/check/test/clippy: success;
- diagnostics finalizer: failure;
- artifact id: `8355565313`;
- artifact name: `ci-diag__component-gdrive-adapter__wf-component-ci__run-29446145224__attempt-1`;
- implementation report blob: `5f0aea2859eba8575f3c8e302b72a09ca7aa24fc`.

Diagnostics protocol:
1. Download artifact `8355565313`.
2. Read `summary.md`, `manifest.json`, every failure marker and every log named by `failed_checks`.
3. Raw job logs are fallback-only if the artifact is unavailable, malformed, expired or incomplete.
4. Do not infer the cause from the finalizer step alone.
5. If evidence cannot be read, report `FIX_BLOCKED_BY_LOGS`.

Fix only the artifact-proven cause. Preserve:
- read-only versioned credential-file boundary;
- Google auth/client construction and in-memory refresh lifecycle;
- fakeable token/provider abstractions;
- safe auth/scope/provider categories and retryability;
- observation-only preflight;
- credential/token/request/path/provider-body redaction;
- fake-only deterministic tests;
- no credential rewriting, live-network CI, Server/API, Storage, scheduler, Deployment or sibling behavior.

Do not weaken tests, alter workflows, add real credentials, broaden product behavior, merge, rebase, force-push or change PR draft state.

Create code/test fixes without CI skip. Obtain a new full exact-SHA Component CI with fmt/check/test/clippy and diagnostics finalization green.

Write only `crates/haze-gdrive-adapter/control/report.md` with:
- `REPORT_TYPE: FIX`;
- `phase_id: FIX-GDA-GDA-P3-CI`;
- `chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P3 CI`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_LOGS`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Record artifact files read, exact cause, minimum changed paths, final code-bearing SHA and exact CI. Do not claim CLEAN_ACCEPT.
