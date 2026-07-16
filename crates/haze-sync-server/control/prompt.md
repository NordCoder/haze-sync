# W1-FIX-SRV-GDA-P1-CI

Before starting, name this worker chat exactly:

`server — W1 FIX-SRV-GDA-P1 CI`

Repository: `NordCoder/haze-sync`
Component: server
Path: `crates/haze-sync-server`
Branch/ref: `component/server`
PR: #45
Role: fixer-worker
Phase: `FIX-SRV-GDA-P1-CI`

This is the artifact-first fixer for the completed SRV-GDA-P1 product candidate. Do not repeat implementation or start clean review.

Candidate and evidence:
- exact code-bearing SHA: `94375e36976c87b62e524c0f1cb490224b778179`;
- implementation report blob: `efc05e8e2a903e7197b90cc996d471f753f5a7fa`;
- Component CI run `29488329906`, number `2035`, attempt `1`;
- fmt/check/test/clippy and Server/PostgreSQL route verification: success;
- diagnostics finalizer: failure;
- artifact id: `8371364050`;
- artifact name: `ci-diag__component-server__wf-component-ci__run-29488329906__attempt-1`.

Required protocol:
1. Download artifact `8371364050`.
2. Read `summary.md`, `manifest.json`, every failure marker and every failed-check log.
3. Use raw job logs only if the artifact is missing, expired, malformed or incomplete.
4. Fix only the artifact-proven cause. If evidence cannot be read, report `FIX_BLOCKED_BY_LOGS`.

Preserve the completed product:
- authenticated private/admin GDrive state reads;
- compare-and-commit route and caller-owned PostgreSQL transactions;
- exact accepted API/Storage fan-in identity;
- route-level PostgreSQL authorization, replay, conflict, rollback, concurrency, isolation and redaction tests;
- no API/Storage semantic changes, provider/OAuth/Core policy/status-control/CLI/Deployment/sibling/workflow expansion.

Create code/test fixes without CI skip. Obtain full exact-SHA DB-capable Component CI with diagnostics finalization green.

Write only `crates/haze-sync-server/control/report.md` with:
- `REPORT_TYPE: FIX`;
- `phase_id: FIX-SRV-GDA-P1-CI`;
- `chat_name: server — W1 FIX-SRV-GDA-P1 CI`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_LOGS`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Record artifact files read, exact cause, minimum changed paths, final code-bearing SHA and exact DB-capable CI. Do not claim CLEAN_ACCEPT.
