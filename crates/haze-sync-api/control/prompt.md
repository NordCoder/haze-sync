# W1-FIX-API-GDA-P2-CI-DIAGNOSTICS

## Routing envelope

- protocol_version: `3`
- repository: `NordCoder/haze-sync`
- component: `api`
- component_path: `crates/haze-sync-api`
- role: `fixer-worker`
- agent_execution_id: `api-FIX-API-GDA-P2-ci-diagnostics-20260717130949-ee53d9`
- chat_key: `api`
- branch: `component/api`
- pull_request: `#44`
- wave: `W1`
- phase: `FIX-API-GDA-P2-CI-DIAGNOSTICS`
- control_prompt_path: `crates/haze-sync-api/control/prompt.md`
- control_report_path: `crates/haze-sync-api/control/report.md`
- expected_report_type: `FIX`

Use the existing dedicated API component chat. For this execution only, act as `fixer-worker`. Do not create role-specific routing and do not derive current state from chat history.

## Mandatory source order

Read and apply, in order:

1. the current project implementation manifest, report template, fixer-worker instructions and GitHub connector instructions;
2. `crates/haze-sync-api/docs/component-contract.md`;
3. `crates/haze-sync-api/docs/implementation-plan.md`;
4. `crates/haze-sync-api/docs/implementation-log.md`;
5. `crates/haze-sync-api/docs/dependency-map.md`;
6. `crates/haze-sync-api/docs/decisions.md`;
7. Architect report blob `dc95fa55d3b707da462beebe56b32d73cd54db86`;
8. completed implementation prompt blob `8a4008404c3c83876a731e41b7d151c1ca20a569`;
9. completed implementation report blob `f60ca2d90b233d541a70451883838dc873d6da49`;
10. exact code-bearing commit `618fda1d01ec636ba95884f5cf8f6a596560381b` and its API-owned changed files;
11. Component CI run `29582479954`, run number `2066`, attempt `1`;
12. diagnostics artifact `8407621247` named `ci-diag__component-api__wf-component-ci__run-29582479954__attempt-1`;
13. current API branch code, tests, docs, control state and PR metadata;
14. this active prompt.

Old prompts and reports are read-only evidence unless explicitly identified above. Do not archive control files.

## Verified starting evidence

The Orchestrator verified:

- implementation report status: `SELF_NEEDS_FIX`;
- exact code-bearing SHA: `618fda1d01ec636ba95884f5cf8f6a596560381b`;
- workflow: `Component CI`;
- workflow run ID: `29582479954`;
- run number: `2066`;
- conclusion: `failure`;
- job ID: `87891297074`;
- GitHub step conclusions show checkout, toolchain, context resolution, cargo fmt/check/test/clippy and artifact upload as completed, while `Finalize CI diagnostics` failed;
- step conclusions are not sufficient to infer which wrapped cargo command failed;
- diagnostics artifact ID: `8407621247`;
- artifact digest: `sha256:077d47353cd09a4575a26cf83a7f7962b9cb667411cbb359aeeb08cac4481ba9`;
- artifact size: `6407` bytes;
- artifact is currently unexpired and identifies branch `component/api` and head SHA `618fda1d01ec636ba95884f5cf8f6a596560381b`.

Do not guess the root cause from the finalizer step or from this summary. The artifact is the authoritative failure source.

## Required artifact-first diagnosis

Through the GitHub connector:

1. download artifact `8407621247`;
2. read `ci-diagnostics/summary.md`;
3. read `ci-diagnostics/manifest.json`;
4. verify the manifest identifies:
   - schema `haze-ci-diagnostics-v1`;
   - component `api`;
   - branch `component/api`;
   - run ID `29582479954`;
   - attempt `1`;
   - head SHA `618fda1d01ec636ba95884f5cf8f6a596560381b`;
5. read every failure marker and every log named by `failed_checks`;
6. identify the minimum proven API-owned cause;
7. make only the minimum correction required by that evidence.

If the artifact is missing, expired, malformed, inconsistent with the verified coordinates or unreadable, do not infer a cause. Write `FIX_BLOCKED_BY_LOGS` with exact sanitized evidence.

Raw GitHub job logs are not the primary source and are not authorized as a substitute for a valid diagnostics artifact. Do not paste raw logs into the report.

## Fix objective

Restore full exact-SHA Component CI while preserving the accepted private cursor contract implementation.

The accepted product shape must remain:

- `GDrivePrivateCursorStateDto::Absent { generation }`;
- `GDrivePrivateCursorStateDto::Present { generation, cursor: GDriveRawCursorDto }`;
- strict tagged `snake_case` serde with `deny_unknown_fields`;
- `GDriveStateSnapshotResponse.cursor` uses the private state type;
- admin summary continues to use unchanged `GDriveCursorSummaryDto` and contains no cursor value;
- absent generation must be `0`;
- present generation must be greater than `0`;
- no new route, reset/clear semantics or implicit fresh-cursor policy;
- no raw cursor in admin output, logs, errors, diagnostics, reports, Debug or Display.

Do not undo or weaken the implementation merely to satisfy CI.

## Allowed scope

Only files inside API ownership that are directly required by the artifact-proven failure:

- `crates/haze-sync-api/src/**`;
- `crates/haze-sync-api/tests/**`;
- `crates/haze-sync-api/fixtures/**`;
- `crates/haze-sync-api/docs/**` only when factual alignment is required by the fix;
- `crates/haze-sync-api/control/report.md`.

A broader API-internal edit is allowed only when the diagnostics prove it is necessary and the report explains why the smaller correction was insufficient.

## Forbidden scope

- `.github/**` workflows, scripts or diagnostics harness changes;
- Server, Storage, GDrive adapter, Core, Common, CLI, Worktree, Obsidian or Deployment files;
- dependency or lockfile changes unless a later Orchestrator prompt explicitly authorizes them;
- new routes, versioned routes, status/doctor/operator surfaces;
- cursor reset, clear or fallback policy;
- weakening/removing tests or changing assertions only to hide a real failure;
- ignoring a failed check, deleting fixtures, lowering validation or redaction guarantees;
- real cursor values, credentials, tokens, provider payloads, private paths or raw response bodies;
- external provider/network tests;
- merge, rebase, force-push, history rewrite or PR draft-state changes.

If the artifact proves the root cause belongs to the CI harness or another component, do not edit that owner’s files. Report `FIX_BLOCKED_BY_CONTRACT` with exact sanitized evidence and the required owner.

If connector/tooling prevents a safe component-owned fix after the artifact was read, report `FIX_BLOCKED_BY_TOOLING` without inventing commits or CI results.

## Tests and verification

Preserve and run the complete API contract test surface, including:

- exact absent/present private wire variants;
- strict unknown/wrong-shape rejection;
- cursor size/control-character validation;
- absent/non-zero and present/zero invariant rejection;
- admin cursor-value exclusion;
- synthetic secrecy sentinel coverage;
- compatibility fixture validation;
- existing mapping, pagination, commit, idempotency, error and public-contract tests.

Use synthetic values only. Never include a raw cursor or secret in reports, diagnostics commentary or commit messages.

## Commit and CI honesty

- Product, test, fixture and documentation fix commits must not use CI skip.
- A final report-only control commit may use CI skip.
- Obtain a real final code-bearing SHA after the fix.
- Obtain full `Component CI` for that exact SHA.
- Verify the actual wrapped cargo outcomes and the diagnostics finalizer, not only step labels.
- Full acceptance requires fmt, check, test, clippy and finalizer success.
- A skipped, cancelled, pending or unrelated run is not green evidence.
- Do not fabricate commit, blob, run, job or artifact identifiers.
- If the next exact-SHA CI is red, use its new diagnostics artifact and continue only for API-owned proven failures within this same focused scope.
- If another correction is still required when reporting, use `FIX_NEEDS_MORE` and record the exact latest SHA/run/artifact.

## Reporting

Write only `crates/haze-sync-api/control/report.md` using the project report template.

The report routing envelope must include:

- `REPORT_TYPE: FIX`;
- terminal `STATUS`: `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_LOGS`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`;
- `role: fixer-worker`;
- `agent_execution_id: api-FIX-API-GDA-P2-ci-diagnostics-20260717130949-ee53d9`;
- `chat_name: api`;
- component `api`;
- branch `component/api`;
- wave `W1`;
- phase_id `FIX-API-GDA-P2-CI-DIAGNOSTICS`;
- control prompt/report paths;
- prompt commit SHA and prompt blob SHA supplied by the dispatcher launch envelope.

Also record, without raw secret-bearing logs:

- artifact summary and manifest verification;
- exact `failed_checks` names;
- sanitized root cause;
- changed files and minimum-fix rationale;
- previous code-bearing SHA `618fda1d01ec636ba95884f5cf8f6a596560381b`;
- final code-bearing SHA, when one exists;
- exact final CI run, number, attempt, job and conclusion;
- fmt/check/test/clippy/finalizer outcomes;
- any new artifact metadata if CI remains red;
- confirmation that Server, Storage and GDrive implementation were not changed;
- confirmation that no raw cursor, token, Idempotency-Key, provider payload or private path was exposed.

Do not archive control files and do not claim clean-review acceptance, Server readiness, GDrive runtime readiness, deployment readiness or merge readiness.

## Next gate

- `FIX_COMPLETE` with exact-SHA full green Component CI -> Orchestrator opens focused API clean-code/security review in the same dedicated API chat;
- `FIX_NEEDS_MORE` -> Orchestrator evaluates the latest exact diagnostics and may continue the focused fixer loop;
- blocked status -> Orchestrator routes only the proven owner or hold.

Server phase `SRV-GDA-P2-PRIVATE-CURSOR-SNAPSHOT-ROUTE` remains blocked until API implementation, fixer and clean review are accepted with exact-SHA green CI.
