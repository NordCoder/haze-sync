# W1-GDA-FAN-IN-ARCHITECTURE-REVIEW

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 GDA Fan-In Architecture Review`

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: architect-reviewer
Phase: GDA-FAN-IN-ARCHITECTURE-REVIEW

This is an architecture gate. Do not implement product code, transport, persistence, provider clients, scheduling, status surfaces, operator controls, Deployment files, migrations, workflows, or E2E tests.

Do not merge, change draft state, rewrite history, rebase, force-push, or modify sibling branches.

## Synchronized baseline

- accepted component-local product SHA: `06a7051a7e14c1da45de8cf96a78658b59cb823e`;
- post-sync SHA: `f9a2da6eb9ac6f59b1ec18ae4d85eb51964f3cbe`;
- exact main ancestor: `c1e69a664388b0cba028170e8398b9088218957d`;
- pre-sync report blob: `5609c422a4a7a7046bc6537a1dc90d58559bb071`;
- Component CI run `29409560791`, number `1962`, success.

## Current accepted facts

- GDrive is an external replica; Core remains authoritative.
- Adapter owns provider mechanics, OAuth/token-file loading, polling/full scan, import/export planning, echo guard, provider retry/backoff and safe summaries.
- Core/API owns conflict, revision, delete/tombstone and idempotency semantics.
- Direct database ownership is forbidden by default.
- Current binary validates config and exits; it does not perform provider/Core calls or a long-running loop.
- Component-local planners and conservative delete guardrails are accepted, but concrete persistence, transport, provider lifecycle and hosting are unresolved fan-in work.

## Architecture decisions required

Resolve the following before any implementation worker receives product scope.

### 1. Process and lifecycle ownership

Decide whether V1 uses the existing standalone `haze-gdrive-adapter` binary as the sole provider runtime.

Preferred boundary to evaluate:
- standalone long-running adapter process;
- adapter owns provider polling/full scans, retry/backoff and graceful shutdown;
- Server does not host Google provider lifecycle;
- Deployment later packages/configures the accepted binary only.

Reject ambiguous dual hosting or hidden Server-hosted lifecycle.

### 2. Core/API transport

Choose the concrete boundary for:
- submitting normalized import facts/content;
- reading accepted Core changes for export;
- submitting guarded delete candidates/outcomes;
- receiving safe conflict/rejection results;
- adapter authentication and idempotency.

Decide whether existing Server/API routes are sufficient or which exact new API/Server fan-in contracts are required. Do not invent direct Core calls or Server-private state access.

### 3. Durable persistence

Choose ownership for:
- Drive↔Core mapping;
- Drive change cursor;
- Core export sequence cursor;
- echo state;
- delete candidates and confirmation state;
- operation/idempotency identifiers.

Evaluate Server/API-mediated Storage persistence as the default. Direct SQLx/Storage repository use by the standalone adapter requires an explicit architecture exception and must not be inferred.

Specify transactional consistency requirements, especially validation plus mutation for delete-candidate state and cursor advancement only after successful processing.

### 4. Live provider/OAuth boundary

Define:
- accepted Google client implementation ownership inside GDrive adapter;
- OAuth token file format/loading/refresh persistence boundary;
- whether refresh-token updates may rewrite the configured secret file;
- file ownership/mode and atomic replacement requirements;
- prohibition on raw token/path/provider payload output;
- mock/fake-first CI policy and no real credentials in normal CI.

### 5. Scheduling and correctness loop

Define the long-running loop:
- full scan remains correctness backstop;
- change feed is latency optimization;
- cursor advances only after durable successful processing;
- invalid cursor falls back safely to full scan;
- retry/backoff classifications;
- no overlapping cycles unless explicitly safe;
- graceful shutdown and in-flight operation semantics.

### 6. Modes and rollout

Define enforceable behavior for:
- disabled;
- dry_run;
- read_only;
- import_only;
- export_only;
- bidirectional.

Clarify the existing `dry_run` boolean versus mode interaction and prohibit contradictory configurations. Deployment rollout must begin disabled/dry-run and cannot enable bidirectional automatically.

### 7. Status, doctor and operator controls

Choose ownership and delivery for:
- safe process status;
- last scan/feed/import/export facts;
- cursor presence and pending delete candidates;
- safe error category;
- doctor checks;
- audited manual unlock/override if any.

Decide whether these are local process output, CLI-mediated Server/API surfaces, or both. No unaudited mutation control or raw provider facts.

### 8. Deployment readiness gate

Specify the minimum accepted product state before Deployment may add a GDrive service:
- real long-running binary;
- concrete authenticated Server/API transport;
- durable persistence boundary;
- live OAuth/provider lifecycle;
- safe shutdown/status;
- fake-provider integration tests;
- secret-file contract.

Deployment must not package a skeleton that starts and exits as if it were a working adapter.

## Required verdict

Produce a phased fan-in plan with exact ownership and dependency order. Prefer small implementation phases rather than one broad integration task.

For each phase state:
- component/branch owner;
- exact allowed paths/components;
- prerequisites;
- deliverables;
- protected boundaries;
- acceptance evidence;
- whether a separate clean review and CI gate is required.

Do not edit architecture documents unless the existing docs cannot carry the verdict. If wording changes are required, report them as a focused policy-fix need rather than silently implementing product work.

## Report

Write `crates/haze-gdrive-adapter/control/report.md` with:

- `REPORT_TYPE: ARCHITECTURE_REVIEW`;
- `phase_id: GDA-FAN-IN-ARCHITECTURE-REVIEW`;
- `chat_name: gdrive-adapter — W1 GDA Fan-In Architecture Review`;
- status `ARCHITECT_ACCEPT`, `ARCHITECT_NEEDS_DECISION`, `ARCHITECT_NEEDS_POLICY_FIX`, `ARCHITECT_BLOCKED_BY_CONTRACT`, or `ARCHITECT_BLOCKED_BY_TOOLING`.

For `ARCHITECT_ACCEPT`, explicitly pin:
- runtime/process owner;
- transport owner and required API/Server contracts;
- persistence owner and transaction rules;
- OAuth refresh/token-file ownership;
- scheduling/cursor rules;
- mode consistency rules;
- status/doctor/operator-control ownership;
- Deployment readiness gate;
- ordered implementation phases and first executable worker slot.

Do not implement, claim merge readiness, or assign product work directly. Return control to Orchestrator.
