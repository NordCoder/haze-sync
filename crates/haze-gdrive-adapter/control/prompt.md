# W1-GDA-GDA-P3-LIVE-GOOGLE-OAUTH

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 GDA-GDA-P3 Live Google OAuth`

Repository: `NordCoder/haze-sync`
Component: gdrive-adapter
Path: `crates/haze-gdrive-adapter`
Branch/ref: `component/gdrive-adapter`
PR: #50
Role: implementation-worker
Phase: `GDA-GDA-P3-LIVE-GOOGLE-OAUTH`

This is the only active GDrive phase. Do not begin HTTP durable-state transport or long-running runtime work in parallel inside this component.

Accepted inputs:
- config/mode normalization SHA: `fcc04afd1fe9808656d9bc2effbfff7160efe9fc`;
- clean-review report blob: `d9bc16c1a50755ccaecd1b51add231bf52e7e35f`;
- architecture report blob: `14c427880e1201d851cdc9ee04b9cd0e83334de4`;
- exact main ancestor: `c1e69a664388b0cba028170e8398b9088218957d`.

## Fixed architecture

1. GDrive Adapter owns Google client construction, credential loading and access-token refresh.
2. Deployment/operator owns initial authorization and placement/rotation of the credential file outside the repository.
3. V1 credential file is read-only to the service. Refreshed access tokens remain in memory; the adapter does not rewrite the file.
4. Corrupt or incomplete credentials fail startup closed.
5. Revoked authorization or insufficient scope stops mutations and produces safe categorized state; no secret/provider body is exposed.
6. Ordinary CI uses fakes and synthetic fixtures only. No real Google credentials or network calls are required.
7. Adapter remains database-independent and does not call Server/API in this phase.

## Required deliverables

- bounded versioned credential-file model and strict parser;
- absolute configured secret-path validation without displaying the path;
- provider client abstraction suitable for fake and live implementations;
- concrete Google authentication/client construction boundary;
- in-memory access-token refresh lifecycle with bounded expiry handling;
- safe typed categories for not configured, invalid, revoked, insufficient scope, refresh unavailable and provider unavailable;
- startup preflight for required scopes/root configuration without provider mutation;
- Debug/Display/error redaction for credentials, authorization metadata, provider bodies and secret paths;
- deterministic fake-based tests for valid load, malformed file, missing fields, expired access refresh, revoked authorization, insufficient scope, safe retry classification and redaction;
- minimal docs/implementation-log alignment.

## Allowed scope

- focused new or existing modules under `crates/haze-gdrive-adapter/src/**` for credentials, auth and provider-client construction;
- `Cargo.toml`/lockfile only for minimum approved Google/OAuth/HTTP dependencies;
- focused tests and synthetic fixtures;
- GDrive docs and control report.

## Forbidden

- real credentials or live-provider CI;
- writing or rotating the configured credential file;
- Server/API HTTP client or Storage access;
- import/export/change-feed execution expansion;
- long-running scheduler, polling loop or deployment wiring;
- public status/operator contracts;
- sibling component or workflow changes;
- logging raw tokens, client secrets, authorization headers, provider bodies or absolute secret paths;
- merge, rebase, force-push or PR draft-state changes.

Create product/test changes without CI skip. Obtain full exact-SHA Component CI with fmt/check/test/clippy and diagnostics finalization green.

Write only `crates/haze-gdrive-adapter/control/report.md` with:
- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: GDA-GDA-P3-LIVE-GOOGLE-OAUTH`;
- `chat_name: gdrive-adapter — W1 GDA-GDA-P3 Live Google OAuth`;
- status `SELF_ACCEPT`, `SELF_ACCEPT_PENDING_CI`, `SELF_NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_DEPENDENCY`, or `BLOCKED_BY_TOOLING`.

Do not claim CLEAN_ACCEPT or deployment readiness.
