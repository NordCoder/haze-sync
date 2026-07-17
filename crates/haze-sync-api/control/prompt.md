# W1-API-GDA-P2-CLEAN-CODE-SECURITY-REVIEW

## Routing envelope

- protocol_version: `3`
- repository: `NordCoder/haze-sync`
- component: `api`
- component_path: `crates/haze-sync-api`
- role: `clean-code-reviewer`
- agent_execution_id: `api-API-GDA-P2-clean-security-review-20260717181732-ee53d9`
- chat_key: `api`
- branch: `component/api`
- pull_request: `#44`
- wave: `W1`
- phase: `API-GDA-P2-CLEAN-CODE-SECURITY-REVIEW`
- control_prompt_path: `crates/haze-sync-api/control/prompt.md`
- control_report_path: `crates/haze-sync-api/control/report.md`
- expected_report_type: `CLEAN_CODE_REVIEW`

Use the existing dedicated API component chat. For this execution only, act as `clean-code-reviewer`. Do not create role-specific routing and do not derive current state from chat history.

## Mandatory source order

Read and apply, in order:

1. the current project implementation manifest, report template and clean-code-reviewer instructions;
2. `crates/haze-sync-api/docs/component-contract.md`;
3. `crates/haze-sync-api/docs/implementation-plan.md`;
4. `crates/haze-sync-api/docs/implementation-log.md`;
5. `crates/haze-sync-api/docs/dependency-map.md`;
6. `crates/haze-sync-api/docs/decisions.md`;
7. Architect report blob `dc95fa55d3b707da462beebe56b32d73cd54db86`;
8. implementation prompt blob `8a4008404c3c83876a731e41b7d151c1ca20a569`;
9. implementation report blob `f60ca2d90b233d541a70451883838dc873d6da49`;
10. fixer prompt blob `b0200262283f6d8aa2383a5e9de803458b13f9c2`;
11. fixer report blob `fc0b2c67ee9bb7195ab288eed82392b785033fbb`;
12. exact candidate code-bearing commit `dba43751521c32aca53729c1c8dbddf2e7d8fbfb` and all API-owned code/test/fixture/doc changes in the implementation and fixer chain;
13. exact Component CI run `29585502722`, run number `2069`, attempt `1`, job `87901373635`;
14. current API branch, PR metadata, control state and this active prompt.

Old prompts and reports are read-only evidence. Do not archive control files.

## Verified candidate evidence

The Orchestrator verified through GitHub:

- terminal fixer status: `FIX_COMPLETE`;
- final code-bearing SHA: `dba43751521c32aca53729c1c8dbddf2e7d8fbfb`;
- final Component CI run: `29585502722`, run number `2069`, conclusion `success`;
- `cargo fmt`: success;
- `cargo check`: success;
- `cargo test`: success;
- `cargo clippy`: success;
- diagnostics finalizer: success;
- diagnostics upload: skipped because no failure artifact was required;
- fixer report commit: `ef445c3ee464cd830116bd3ec13b42b14fd59cba`;
- API PR `#44` is open, draft, unmerged.

These facts establish review eligibility only. They do not establish `CLEAN_ACCEPT`.

## Review objective

Perform a focused clean-code, contract-integrity and secrecy review of the API-owned private GDrive cursor snapshot correction and its diagnostics fixes.

The accepted contract is intentionally breaking for strict old private GDrive clients. Review whether the exact candidate implements the Architect decision cleanly and safely without expanding API ownership or weakening the existing passive contracts.

Do not implement fixes. Do not edit product code, tests, fixtures, docs, dependencies, lockfiles or workflows. Write only the terminal review report.

## Required review areas

### 1. Exact private cursor DTO and serde

Verify that:

- `GDrivePrivateCursorStateDto` is exactly a strict tagged state model with `absent` and `present` variants;
- `Absent` contains only `generation: u64`;
- `Present` contains `generation: u64` and the existing `GDriveRawCursorDto`;
- `deny_unknown_fields` and snake_case tagging are effective;
- impossible optional-field combinations are not representable;
- `GDriveRawCursorDto` bounds remain unchanged, including `MAX_GDRIVE_CURSOR_BYTES = 8192`, non-empty and no-control-character validation;
- no value-bearing derived or manual formatting leaks the raw cursor.

### 2. Private/admin wire separation

Verify that:

- only `GDriveStateSnapshotResponse.cursor` uses the private cursor state;
- `GDriveStateAdminSummaryResponse.cursor` remains the existing `GDriveCursorSummaryDto { generation, present }`;
- the admin JSON legitimately contains the summary field named `cursor`, but never contains the private cursor value or private state shape;
- the fixer changed only overbroad assertions and did not weaken private-value exclusion;
- provider identifiers, mappings and private cursor values remain absent from the admin response;
- no new route or route version was added.

### 3. Cursor invariants and passive validation

Verify that:

- `Absent { generation: 0 }` is valid;
- absent with non-zero generation fails through the existing safe invalid-cursor-state category;
- `Present` requires generation greater than zero;
- present with generation zero fails safely;
- commit `advance = None` still means unchanged, not cleared;
- no reset, clear, fallback or implicit fresh-cursor policy was introduced;
- state-format, state-version, pagination, numeric and collection bounds remain unchanged.

### 4. Authorization and ownership boundaries

Verify that passive route contracts still express:

- matching `gdrive_adapter` principal receives the private response;
- non-matching adapter remains forbidden;
- unauthenticated caller remains unauthorized;
- authenticated admin receives only the sanitized admin DTO;
- API contains no Axum execution, database access, transaction choreography, Storage implementation, GDrive client/runtime logic or Deployment behavior;
- no sibling component or workflow changes occurred in the candidate chain.

### 5. Redaction and secrecy

Review all relevant `Debug`, `Display`, error, request/response wrapper and test surfaces. Confirm that no raw cursor, bearer token, Idempotency-Key, provider payload, private path or raw dependency body can appear in:

- admin output;
- errors;
- logs or tracing;
- diagnostics;
- `Debug` or `Display`;
- fixtures or reports.

Synthetic sentinels are allowed only as bounded test values and must be proven absent from formatted and sanitized surfaces.

### 6. Test and fixture quality

Verify that the tests prove, rather than merely execute:

- exact absent/present wire variants;
- strict unknown-field and wrong-shape rejection;
- cursor bounds and control-character rejection;
- invalid generation/value pair rejection;
- exact admin summary shape with generation and presence;
- private cursor value exclusion from admin JSON;
- redacted formatting across DTO, snapshot, wrappers and errors;
- compatibility fixture alignment;
- preservation of mapping, pagination, commit, idempotency and safe-error contracts.

Specifically review the fixer changes at intermediate SHA `73154140c31f424f18c2c58241d2b5449428e96e` and final SHA `dba43751521c32aca53729c1c8dbddf2e7d8fbfb` for test weakening or false-positive assertions.

### 7. Compatibility boundary

Confirm that:

- the existing private route change is acknowledged as intentionally breaking for strict old GDrive clients;
- API acceptance is not treated as a deployment event;
- no mixed old/new compatibility is claimed;
- Server and corrected GDrive client remain gated as a coordinated compatibility unit;
- Server phase is not authorized until this review reaches terminal clean acceptance.

## Findings standard

A finding must be concrete and actionable and include:

- severity;
- exact file and code location;
- violated contract or safety invariant;
- observable failure or risk;
- minimum owner-scoped correction;
- whether it blocks acceptance.

Do not request style-only churn. Do not broaden scope into Server, Storage, GDrive runtime, Deployment or unrelated API cleanup.

## Allowed scope

- read repository files, blobs, commits, PR and CI metadata;
- inspect the exact candidate and predecessor diffs;
- write only `crates/haze-sync-api/control/report.md` as a report-only commit using CI skip.

## Forbidden scope

- any product, test, fixture, documentation, dependency, lockfile or workflow edit;
- Server, Storage, GDrive adapter or other component writes;
- merge, rebase, force-push, history rewrite or PR draft-state changes;
- real credentials, provider values, raw cursors, private paths or raw job logs in the report;
- invented commit, blob, run, job or artifact identifiers;
- claiming Server, GDrive runtime, Deployment or merge readiness.

## Reporting

Write only `crates/haze-sync-api/control/report.md` using the project report template.

The routing envelope must include:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- terminal `STATUS`: `CLEAN_ACCEPT`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, or `CLEAN_BLOCKED_BY_TOOLING`;
- `role: clean-code-reviewer`;
- `agent_execution_id: api-API-GDA-P2-clean-security-review-20260717181732-ee53d9`;
- `chat_name: api`;
- component `api`;
- branch `component/api`;
- wave `W1`;
- phase_id `API-GDA-P2-CLEAN-CODE-SECURITY-REVIEW`;
- control prompt/report paths;
- prompt commit SHA and prompt blob SHA supplied by the dispatcher launch envelope.

Also record:

- exact reviewed code-bearing SHA `dba43751521c32aca53729c1c8dbddf2e7d8fbfb`;
- implementation and fixer report blobs;
- exact CI run/job conclusions;
- reviewed files and evidence;
- findings or explicit statement that none remain;
- contract, authorization, compatibility and secrecy verdicts;
- PR state observed;
- confirmation that only the report file changed.

Do not archive control files.

## Next gate

- `CLEAN_ACCEPT` on exact SHA `dba43751521c32aca53729c1c8dbddf2e7d8fbfb` with verified CI -> Orchestrator may accept API phase and prepare `SRV-GDA-P2-PRIVATE-CURSOR-SNAPSHOT-ROUTE`;
- `CLEAN_NEEDS_FIX` -> Orchestrator opens one focused API fixer from the exact findings;
- blocked status -> Orchestrator records the exact hold and does not dispatch Server.
