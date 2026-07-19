WORKFLOW: workflow-minimal
PROMPT_ID: 5-api-orch-plan-20260719-01
CHAT_KEY: 5 api
ROLE: clean-code-reviewer
COMPONENT: api
REPOSITORY: NordCoder/haze-sync
CONTROL_BRANCH: agentic/workflow-minimal-acceptance/api
PRODUCT_BRANCH: component/api
INPUT_COMMIT: 09a4495ed139a0076260c6ba429d616b5d6df836
CURRENT_COMPLETION_PLAN: haze-sync-v1-completion-plan.md
WORK_ITEM: API-R0
PLAN_SECTIONS:
- §4.2 API — API-R0 — Private cursor contract closure
- §6 Wave R1 — Contract closure and independent foundations
- §8 First dispatch batch

# Objective

Complete the mandatory focused clean-code and security review for API-R0 and produce one accepted exact API SHA for the private Google Drive cursor contract. Review the already implemented candidate and its later exact-SHA CI fix; do not reimplement the contract unless the review proves an in-scope defect.

# Context

The completion plan fixes API-R0 as the first serial barrier for SRV-GDA-R1 and GDA-R1. Current GitHub evidence shows:

- implementation candidate code-bearing SHA: `618fda1d01ec636ba95884f5cf8f6a596560381b`;
- focused CI fixer final code-bearing SHA: `dba43751521c32aca53729c1c8dbddf2e7d8fbfb`;
- Component CI run `29585502722` / run number `2069`: completed successfully for `dba43751521c32aca53729c1c8dbddf2e7d8fbfb`;
- current branch HEAD `09a4495ed139a0076260c6ba429d616b5d6df836` is six control-only commits after the accepted code-bearing candidate, with no later API product-code changes.

The remaining unclosed gate is the focused clean-code/security review required by the component log and completion plan.

# Inputs and evidence

- current completion plan: `haze-sync-v1-completion-plan.md`, API-R0, Wave R1, first dispatch batch;
- product branch: `component/api` @ `09a4495ed139a0076260c6ba429d616b5d6df836`;
- implementation evidence: `crates/haze-sync-api/control/log/20260717-131500Z-W1-API-GDA-P2-PRIVATE-CURSOR-SNAPSHOT-CONTRACT-implementation-worker-report.md`;
- CI-fixer evidence: `crates/haze-sync-api/control/log/20260717-182500Z-W1-FIX-API-GDA-P2-CI-DIAGNOSTICS-fixer-worker-report.md`;
- exact successful CI run: `29585502722`, exact code-bearing SHA `dba43751521c32aca53729c1c8dbddf2e7d8fbfb`;
- contract and planning evidence:
  - `crates/haze-sync-api/docs/component-contract.md`;
  - `crates/haze-sync-api/docs/gdrive-state-contract.md`;
  - `crates/haze-sync-api/docs/implementation-log.md`;
  - `crates/haze-sync-api/docs/implementation-plan.md`;
  - `crates/haze-sync-api/docs/dependency-map.md`;
- review surface:
  - `crates/haze-sync-api/src/dto/gdrive.rs`;
  - `crates/haze-sync-api/src/routes/gdrive.rs`;
  - `crates/haze-sync-api/src/contracts/headers.rs`;
  - `crates/haze-sync-api/tests/gdrive_state_compatibility_fixture.rs`;
  - `crates/haze-sync-api/fixtures/gdrive-state-contract-v1.json`.

# Authorized scope

Allowed:

- read and review `component/api` at the exact input commit;
- modify only `crates/haze-sync-api/**` when a concrete API-R0 correctness, secrecy, fixture, validation, or maintainability defect is proven;
- add or correct focused API tests and documentation needed to close API-R0;
- commit and push justified review fixes to `component/api`.

Forbidden:

- `main`, sibling product branches, and other actor control branches;
- Server, Storage, Core, CLI, GDrive adapter runtime, deployment, or CI workflow changes;
- new public behavior outside the private cursor contract;
- broad refactoring, route registration, persistence, provider behavior, or product fan-in;
- force-push, rebase, reset, history rewrite, or unrelated cleanup.

# Required actions

1. Verify `HEAD(component/api) == 09a4495ed139a0076260c6ba429d616b5d6df836` before review or editing.
2. Verify that product-code changes after `dba43751521c32aca53729c1c8dbddf2e7d8fbfb` are absent; distinguish control-only commits from product changes.
3. Review API-R0 for correctness, validation bounds, authorization, raw-cursor secrecy, Debug/Display/error safety, fixture strictness, serde shape, naming, duplication, unnecessary complexity, and test honesty.
4. Confirm `GDrivePrivateCursorStateDto` has correct absent/present variants and that present state contains generation plus bounded raw cursor only for the matching adapter.
5. Confirm raw cursor material is absent from admin/public JSON, Debug, errors, logs, and report-safe summaries.
6. Confirm matching-adapter authorization, malformed state, invalid generation/value combinations, and deterministic compatibility fixtures are tested.
7. Fix only concrete findings within the allowed API scope; otherwise leave product HEAD unchanged.
8. Run the required checks and record the exact final product SHA.
9. Publish a workflow-minimal terminal report with a clear ACCEPTED or BLOCKED/NEEDS-FOLLOW-UP conclusion and evidence. Do not create the next assignment.

# Required checks

- `cargo fmt --check`;
- `cargo check -p haze-sync-api`;
- `cargo test -p haze-sync-api`;
- `cargo clippy -p haze-sync-api --all-targets -- -D warnings`;
- focused compatibility/secrecy tests for the GDrive private cursor surface;
- compare final product diff against the authorized API paths;
- verify the current prompt identity immediately before any product push;
- report every check not run and the exact reason.

# Completion

Write `.agentic/report.md` on `agentic/workflow-minimal-acceptance/api`. The report must repeat `PROMPT_ID`, `CHAT_KEY`, `ROLE`, `COMPONENT`, `WORK_ITEM`, `CURRENT_COMPLETION_PLAN`, the relevant `PLAN_SECTIONS`, input and output product commits, exact changed files, checks, CI evidence used, remaining work, and blockers.

When terminal, create `.agentic/done.json` with the same `prompt_id` and `chat_key`, and publish the final report plus done marker together in one control-branch commit. Reread and verify both files. Before product push or another external side effect, reread this prompt and verify that `PROMPT_ID` is unchanged.