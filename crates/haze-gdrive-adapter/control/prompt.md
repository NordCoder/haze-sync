# W1-FIX-GDA-GDA-P1-CI

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 FIX-GDA-GDA-P1 CI`

Repository: `NordCoder/haze-sync`
Component: gdrive-adapter
Branch/ref: `component/gdrive-adapter`
PR: #50
Role: fixer-worker
Phase: `FIX-GDA-GDA-P1-CI`

Fix only the diagnostics-artifact-proven failure for code-bearing SHA `afd263d621723952f11ecd0c16c09e209710feac`.

Authoritative failing run:
- Component CI run `29431493842`;
- artifact id `8349581571`;
- artifact name `ci-diag__component-gdrive-adapter__wf-component-ci__run-29431493842__attempt-1`.

Download the artifact and read `summary.md`, `manifest.json`, and every failed-check log before editing. Raw job logs are fallback-only. If the artifact cannot be read, report `FIX_BLOCKED_BY_LOGS`.

Preserve the implemented contract: authoritative `HAZE_GDRIVE_MODE`, six accepted modes, fail-closed legacy `HAZE_GDRIVE_DRY_RUN`, capability matrix, strict aliases and redaction. Do not add OAuth, provider, HTTP, persistence, scheduler, status, Deployment, sibling or workflow work. Do not weaken tests.

Make the minimum proven fix and obtain a new full exact-SHA green Component CI. PR #50 must remain open, draft and unmerged.

Write only `crates/haze-gdrive-adapter/control/report.md` with:
- `REPORT_TYPE: FIX`;
- `phase_id: FIX-GDA-GDA-P1-CI`;
- `chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P1 CI`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_LOGS`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Record artifact files read, exact failure cause, minimal changed paths, final code-bearing SHA and full CI evidence. Do not claim CLEAN_ACCEPT.
