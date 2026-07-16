# W1-FIX-GDA-GDA-P2-HTTP-SECURITY

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 FIX-GDA-GDA-P2 HTTP Security`

Repository: `NordCoder/haze-sync`
Component: gdrive-adapter
Path: `crates/haze-gdrive-adapter`
Branch/ref: `component/gdrive-adapter`
PR: #50
Role: fixer-worker
Phase: `FIX-GDA-GDA-P2-HTTP-SECURITY`

This is a focused review-fixer for the existing `GDA-GDA-P2-HTTP-AND-DURABLE-STATE-CLIENT` phase. Do not begin the long-running runtime or another product phase.

Review target:
- code-bearing SHA: `cb9c85169e6f212e11824858501e70b182b26a29`;
- clean-review report blob: `e84988ad9aea6881f142b30c98461c2279a08106`;
- green Component CI run: `29507840727`, number `2049`.

The review accepted exact API fan-in, identity typing, route/body/header contracts, mode-before-transport gating, pagination bounds, strict status/body decoding, explicit no-retry behavior and most secrecy boundaries. Fix only these four findings:

1. Concrete overall request deadline and timeout classification.
   - Apply `policy.request_timeout` as the ureq overall request deadline, while retaining the separate connect bound.
   - Preserve bounded response-body reading.
   - Map concrete timed-out transport errors to the safe typed `HttpTransportError::Timeout` category without exposing dependency errors, URLs, headers or bodies.
   - Add deterministic in-process transport tests proving the overall deadline, timeout classification and bounded body behavior. Do not use external network.

2. Exact origin-only Server base URL.
   - Parse and validate the configured HTTP(S) endpoint as an origin-only base.
   - Reject userinfo, query, fragment and any non-root path prefix.
   - Accept equivalent root forms with or without one trailing slash and construct the exact accepted `/v1/adapters/{adapter_id}/gdrive/...` routes.
   - Add tests for root trailing slash, forbidden prefixes, malformed authority, userinfo, query and fragment.
   - Do not introduce a path-prefix contract.

3. Provider-root/config/runtime redaction.
   - Replace derived Debug where needed so `AdapterConfig` and `AdapterRuntime` never expose `drive_root_folder_id`, raw Server endpoint, adapter identity, token or secret path.
   - Use fixed markers or safe summaries only.
   - Add provider-root and endpoint sentinels to config/runtime Debug and error secrecy tests.

4. Reproducible dependency lock.
   - Regenerate and commit `Cargo.lock` from the final manifest so the adapter dependency list is represented.
   - Do not change unrelated dependency intent.
   - Validate a locked build/check path in addition to the standard component checks where feasible; record exact evidence.

Allowed scope:
- `crates/haze-gdrive-adapter/src/durable_state.rs` and focused tests;
- `crates/haze-gdrive-adapter/src/config.rs`, `src/runtime.rs` and focused secrecy tests;
- `Cargo.lock`;
- minimal GDrive docs only if behavior must be corrected;
- control report.

Preserve:
- exact byte-identical accepted API owner blobs and module semantics;
- explicit `HAZE_GDRIVE_ADAPTER_ID` contract;
- exact GET/POST routes, bearer and Idempotency-Key contracts;
- mode gating before request construction and transport;
- pagination and aggregate bounds;
- strict outcome/error classification;
- no automatic compare-and-commit retry;
- fake/synthetic ordinary CI and all existing secrecy coverage.

Forbidden:
- API semantic edits or Server/Storage/Core product changes;
- direct DB access;
- Google/provider synchronization or mutation;
- scheduler or long-running runtime composition;
- status-control, CLI or Deployment work;
- live credentials, live Server or external-network CI;
- dependency-error, URL, secret or private-state leakage;
- sibling/workflow changes, test weakening, merge, rebase, force-push or PR draft-state changes.

Create code/test/lockfile changes without CI skip. Obtain full exact-SHA Component CI with fmt/check/test/clippy and diagnostics finalization green.

Write only `crates/haze-gdrive-adapter/control/report.md` with:
- `REPORT_TYPE: FIX`;
- `phase_id: FIX-GDA-GDA-P2-HTTP-SECURITY`;
- `chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P2 HTTP Security`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Record exact timeout configuration/classification, URL validation, redaction tests, lockfile evidence, changed paths, final code-bearing SHA and exact CI. Do not claim CLEAN_ACCEPT; a repeat HTTP/security clean review follows.
