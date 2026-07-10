# W1-OBS-P9-NODE-CI — Add Obsidian Node validation to Component CI

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Authorization and context

The user explicitly authorized a scoped CI change for Obsidian Plugin validation.

OBS-P9 implementation and OBS-P9C clean-code corrections are present. Rust-oriented Component CI is green for the final plugin source head, but it does not execute the required Node commands.

- plugin_source_sha: d43ef5fe946de1f4467572ef0df5311e1b239717
- existing_component_ci_run: 29107572342
- existing_component_ci_conclusion: success
- missing independent checks:
  - npm test --workspace haze-obsidian-plugin
  - npm run typecheck --workspace haze-obsidian-plugin
  - npm run build --workspace haze-obsidian-plugin

The existing scripts are already defined in apps/haze-obsidian-plugin/package.json, and the repository has a root package-lock.json.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, .github/workflows/component-ci.yml, .github/scripts/ci-run.sh, .github/scripts/ci-finalize.sh, root package.json/package-lock.json, apps/haze-obsidian-plugin/package.json, current control files, and the relevant PR diff.

Do not read old diagnostics artifacts unless a new workflow run fails and the resulting active task explicitly authorizes fixer diagnostics.

## Task

Extend `.github/workflows/component-ci.yml` with a separate Obsidian Node validation job while preserving the existing Rust job unchanged.

Required behavior:

1. The new job runs only for the `component/obsidian-plugin` PR branch, plus an equivalent explicit manual-dispatch branch condition if practical.
2. Use `actions/checkout@v4` and `actions/setup-node@v4`.
3. Use Node.js 20 and npm cache backed by the root `package-lock.json`.
4. Set diagnostics context for the job:
   - `HAZE_COMPONENT=obsidian-plugin`
   - `HAZE_COMPONENT_BRANCH` from the PR head branch or current ref
   - `HAZE_HEAD_SHA` from the PR head SHA or current SHA
   - `HAZE_WORKFLOW_SLUG=component-ci`
5. Run these commands through the existing `.github/scripts/ci-run.sh` wrapper with distinct stable check names:
   - `node-install`: `npm ci`
   - `node-test`: `npm test --workspace haze-obsidian-plugin`
   - `node-typecheck`: `npm run typecheck --workspace haze-obsidian-plugin`
   - `node-build`: `npm run build --workspace haze-obsidian-plugin`
6. Run `.github/scripts/ci-finalize.sh` with `if: ${{ !cancelled() }}`.
7. On failure, upload `ci-diagnostics/` with `actions/upload-artifact@v4`, one-day retention, the existing artifact naming convention, and no secret or raw provider data.
8. Preserve the current Rust job, its commands, finalizer, artifact behavior, permissions, and concurrency semantics.
9. Do not run the Node job for unrelated component branches.
10. Do not commit `node_modules`, `.test-dist`, `main.js`, source maps, generated bundles, caches, or other build output.

A dedicated conditional job inside the existing workflow is preferred over creating a second overlapping workflow, because it keeps one canonical CI lifecycle and one diagnostics protocol.

## Validation

The workflow/config commit must not use CI skip.

After the non-skipped workflow commit:

- observe the new pull-request workflow run through GitHub metadata;
- verify the Rust job still completes normally;
- verify the Obsidian Node job exposes separate steps for install, test, typecheck, build, and finalization;
- record the exact workflow run id, job conclusions, and step conclusions;
- treat only an actually completed successful Node job as evidence that all three required plugin checks passed.

If a Node command fails, do not broaden into product-code repair in this phase. Preserve the failing evidence and report `SELF_NEEDS_FIX` with the exact run/job/check names so Orchestrator can assign a scoped fixer. If the workflow cannot produce diagnostics because setup fails before the wrapper, report `BLOCKED_BY_TOOLING` with the exact setup failure evidence.

## Allowed files

- .github/workflows/component-ci.yml
- apps/haze-obsidian-plugin/control/report.md

Only if strictly necessary to make the already-defined commands executable, and with explicit justification in the report:

- package.json
- package-lock.json
- apps/haze-obsidian-plugin/package.json

## Forbidden changes

No plugin product-code fixes, test deletion, assertion weakening, dependency upgrades unrelated to CI execution, changes to Rust validation commands, changes to other component behavior, new secrets, provider calls, generated artifact commits, PR lifecycle actions, merges, or sibling branch changes.

## CI trigger policy

Workflow/package/config changes must not skip CI. A final report-only commit may skip CI only after the code-bearing workflow commit has produced observable CI evidence.

## Report

Write only apps/haze-obsidian-plugin/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id OBS-P9-NODE-CI.
