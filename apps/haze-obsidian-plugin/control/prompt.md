# W1-OBS-FAN-IN-P1-SERVER-COMPAT-E2E

Before starting, name this worker chat exactly:

`obsidian-plugin — W1 OBS-FAN-IN-P1 Server Compatibility E2E`

Repository: `NordCoder/haze-sync`
Component: obsidian-plugin
Path: `apps/haze-obsidian-plugin`
Branch/ref: `component/obsidian-plugin`
PR: #51
Role: implementation-worker
Phase: `OBS-FAN-IN-P1-SERVER-COMPAT-E2E`

This is the only active Obsidian phase. It is an explicit component-owned fan-in/integration phase after OBS-P1 through OBS-P9 completion.

## Accepted baseline

- accepted Obsidian code-bearing SHA: `3b47c3c96fd25d434ec3ce0f8821c0c3feb6a423`;
- accepted Component CI run: `29120283487`, run number `1528`, success;
- exact main ancestor: `c1e69a664388b0cba028170e8398b9088218957d`;
- accepted API contract SHA: `56ae94570441d68715f34b5d54381a0fc4d7c231`;
- accepted Server HTTP/application SHA: `50461354c18ddc4d2e47202d9303b4358a27ee45`;
- PR #51 remains open, draft and unmerged.

Fetch the actual branch head before editing because Orchestrator control-only commits follow the accepted source SHA.

## Goal

Turn the complete plugin-local implementation into a contract-checked Server integration surface without changing Server/API or inventing new public behavior.

Default CI must remain deterministic and require no external network, production database, private data or credentials.

## Required deliverables

1. Verify TypeScript DTO compatibility against accepted public API fixtures and endpoint vocabulary.
2. Add a component-owned integration harness with:
   - deterministic fake HTTP transport for normal CI;
   - optional explicitly enabled loopback Server smoke mode using operator-provided local test configuration.
3. Cover at minimum:
   - server-info capability negotiation;
   - changes-page parsing and cursor persistence rules;
   - file upload metadata including idempotency, content hash and base/null-base semantics;
   - file metadata/content retrieval and hash verification boundary;
   - guarded delete request construction;
   - conflict list and supported resolve actions;
   - safe public error mapping for authorization, conflict, unavailable and validation failures;
   - redaction of sensitive request metadata, raw response bodies and absolute local paths.
4. Add an optional local Server smoke procedure that is skipped honestly when configuration is absent.
5. Use only synthetic vault paths and synthetic content in fixtures.
6. Update integration/testing docs and implementation log minimally.

## Scope

Allowed:

- `apps/haze-obsidian-plugin/src/api-client/**` only for compatibility fixes proven necessary by accepted API/Server contracts;
- `apps/haze-obsidian-plugin/tests/**`;
- focused integration-test helpers/scripts under `apps/haze-obsidian-plugin/**`;
- `apps/haze-obsidian-plugin/package.json` only for focused test commands;
- Obsidian docs and control report.

Production plugin behavior should remain unchanged unless a real accepted-contract incompatibility is proven. Prefer test/harness changes over product refactoring.

## Forbidden

- API, Server, Core, Storage, Worktree, GDrive, CLI or Deployment changes;
- route or DTO invention;
- direct database access;
- provider calls;
- real user vault data or credentials;
- tests requiring public network access;
- weakening existing compatibility, typecheck or build checks;
- generated release bundle or marketplace publication;
- background/mobile correctness claims;
- merge, draft-state change, rebase, force-push or history rewrite;
- workflow changes; report blocked if the current validation surface makes the phase impossible.

If the accepted Server/API surface is insufficient, report `BLOCKED_BY_CONTRACT` with the exact missing contract. Do not modify the sibling owner.

## Validation

Create code/test/doc commits without CI skip. Control/report-only commits may use CI skip.

Required exact final code-bearing SHA evidence:

- npm ci through Component CI;
- plugin tests green;
- plugin typecheck green;
- plugin build green;
- Rust workspace green;
- optional loopback Server smoke reported as run or honestly skipped;
- no private data or credentials;
- PR #51 remains open, draft and unmerged.

## Report

Write only `apps/haze-obsidian-plugin/control/report.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: OBS-FAN-IN-P1-SERVER-COMPAT-E2E`;
- `chat_name: obsidian-plugin — W1 OBS-FAN-IN-P1 Server Compatibility E2E`.

Allowed statuses:

- `SELF_ACCEPT`;
- `SELF_ACCEPT_PENDING_CI`;
- `SELF_NEEDS_FIX`;
- `BLOCKED_BY_CONTRACT`;
- `BLOCKED_BY_DEPENDENCY`;
- `BLOCKED_BY_TOOLING`.

Record exact changed paths, fixture/endpoint coverage, fake and optional loopback evidence, any proven compatibility correction, secrecy evidence, final code-bearing SHA and exact CI. Do not claim CLEAN_ACCEPT or merge readiness.
