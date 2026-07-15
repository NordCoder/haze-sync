# W1-FIX-OBS-FAN-IN-P1-CI

Before starting, name this worker chat exactly:

`obsidian-plugin — W1 FIX-OBS-FAN-IN-P1 CI`

Repository: `NordCoder/haze-sync`
Component: obsidian-plugin
Branch/ref: `component/obsidian-plugin`
PR: #51
Role: fixer-worker
Phase: `FIX-OBS-FAN-IN-P1-CI`

Fix only the diagnostics-artifact-proven failure for code-bearing SHA `517b8cd77dc7c24eec78ae6c2f1ab55e7a8aeb3a`.

Authoritative failing run:
- Component CI run `29434046014`, run number `1999`;
- Node job `87416059909` failed only at diagnostics finalization after npm install, tests, typecheck and build succeeded;
- Rust workspace job `87416059992` succeeded;
- diagnostics artifact id `8350606040`;
- artifact name `ci-diag__component-obsidian-plugin__wf-component-ci__run-29434046014__attempt-1`.

Download the artifact and read `summary.md`, `manifest.json`, and every failed-check log before editing. Raw job logs are fallback-only. If artifact evidence is unavailable, report `FIX_BLOCKED_BY_LOGS`.

Preserve the implemented fan-in surface: deterministic fake HTTP compatibility harness, loopback-only optional smoke, accepted endpoint/header semantics, synthetic fixtures, public error/path redaction, no sibling changes. Do not add routes, DTOs, production behavior, external-network CI, credentials, release packaging or workflow changes. Do not weaken tests.

Apply only the proven minimum correction and obtain a new full exact-SHA green Component CI with Node validation and Rust workspace green. PR #51 remains open, draft and unmerged.

Write only `apps/haze-obsidian-plugin/control/report.md` with:
- `REPORT_TYPE: FIX`;
- `phase_id: FIX-OBS-FAN-IN-P1-CI`;
- `chat_name: obsidian-plugin — W1 FIX-OBS-FAN-IN-P1 CI`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_LOGS`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Record artifact files read, exact cause, minimal changed paths, final code-bearing SHA and full CI evidence. Do not claim CLEAN_ACCEPT.
