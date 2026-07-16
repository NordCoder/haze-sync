REPORT_TYPE:
CLEAN_CODE_REVIEW

STATUS:
CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
chat_name: gdrive-adapter — W1 GDA-GDA-P2 HTTP Security Review

COMPONENT:
name: gdrive-adapter
branch: component/gdrive-adapter
pr: 50

WAVE:
id: W1
phase_id: GDA-GDA-P2-CLEAN-REVIEW

REVIEW_TARGET:
code_bearing_sha: cb9c85169e6f212e11824858501e70b182b26a29
implementation_report_blob: 4574f7a8e3a91892062b86c329025011ea332c71
ci_run_id: 29507840727
ci_run_number: 2049
ci_conclusion: success

AUTHORITATIVE_INPUTS:
accepted_api_sha: c60c3976696da1970d539e5cff6e9f74a61fc10e
accepted_api_clean_report_blob: 55ff6047c9c6c0f6f548f10197b76706c0a244e1
accepted_server_sha: c023b83e1e6f502e7d2261acccb871dd5588edf1
accepted_server_clean_report_blob: 1c223ebda33a1550fa8cbf38a15079ef7810c63d
accepted_oauth_sha: 7f00a60641ca157907d0e75e4ab1bb47c05f03c9

SUMMARY:
The exact accepted GDrive DTO, route and header blobs are present, mode gates run before the wrapped client, pagination and status/body decoding are generally strict, POST is not explicitly retried by adapter code, and private HTTP request/response/client surfaces use fixed redacted formatting. The candidate cannot be accepted because the concrete transport does not enforce the configured overall request deadline, URL normalization permits a path prefix that changes the accepted route, provider root identity is exposed by derived runtime/config Debug, and the committed lockfile does not record the new adapter dependencies.

EXACT_API_FAN_IN:
semantic_changes_found: no
files:
- path: crates/haze-sync-api/src/dto/gdrive.rs
  reviewed_blob: 8098457eee373f63210fbd381c6487a054d7609f
  accepted_blob: 8098457eee373f63210fbd381c6487a054d7609f
  byte_identical: yes
- path: crates/haze-sync-api/src/routes/gdrive.rs
  reviewed_blob: ef7f8dc1d02b3742e76bb99c66ee39703bee3b4b
  accepted_blob: ef7f8dc1d02b3742e76bb99c66ee39703bee3b4b
  byte_identical: yes
- path: crates/haze-sync-api/src/contracts/headers.rs
  reviewed_blob: b146f269056a6ff20cca61591c19cdc14b9085b6
  accepted_blob: b146f269056a6ff20cca61591c19cdc14b9085b6
  byte_identical: yes
module_wiring:
- crates/haze-sync-api/src/dto/mod.rs exposes gdrive without semantic edits to the owner file content
- crates/haze-sync-api/src/routes/mod.rs exposes gdrive without semantic edits to the owner file content

FINDINGS:
- severity: high
  title: Concrete transport does not enforce the configured overall request timeout
  path: crates/haze-gdrive-adapter/src/durable_state.rs
  evidence:
  - UreqHttpTransport::new configures timeout_connect, timeout_read and timeout_write, but never configures ureq AgentBuilder::timeout.
  - In ureq 2.12, timeout_read and timeout_write bound individual socket operations; AgentBuilder::timeout is the overall deadline covering connection and response-body reading.
  - A peer can continue a request beyond policy.request_timeout by making progress within each individual read interval, so the request duration is not bounded as required.
  - Pre-response ureq::Error::Transport values are all mapped to HttpTransportError::Unavailable, so connection/header timeout failures are not preserved as the typed Timeout category.
  impact:
  - A slow or trickling Server connection can block the adapter longer than the declared request policy.
  - Concrete timeout classification differs from the fake-only tests and from the public typed error surface.
  required_fix:
  - Apply policy.request_timeout as the ureq overall request deadline while retaining the separate connect bound.
  - Classify concrete timed-out transport errors as HttpTransportError::Timeout without formatting or exposing the dependency error or URL.
  - Add deterministic in-process transport tests proving an overall timeout, timeout classification and bounded body reading.

- severity: high
  title: Accepted server URL can silently change the exact GDrive route path
  path: crates/haze-gdrive-adapter/src/durable_state.rs
  evidence:
  - validate_and_normalize_server_url validates only scheme, whitespace, query, fragment, a non-empty first path segment used as authority, and absence of a literal at-sign in that segment.
  - It permits a non-root URL path such as https://sync.example.test/prefix.
  - state_request and commit_request concatenate the accepted route constants to that value, producing /prefix/v1/adapters/... rather than the accepted /v1/adapters/... route.
  impact:
  - Configuration accepted at startup can never address the accepted Server routes and may be misclassified later as not-found or malformed behavior.
  - Exact API/Server route compatibility is not fail-closed at construction.
  required_fix:
  - Parse and validate the server endpoint as an origin-only HTTP(S) base with no userinfo, query, fragment or non-root path, unless an owner-approved path-prefix contract is introduced.
  - Add deterministic URL construction tests for a root trailing slash, forbidden path prefixes, malformed authorities and query/fragment/userinfo cases.

- severity: high
  title: Provider root identity leaks through derived config and runtime Debug
  paths:
  - crates/haze-gdrive-adapter/src/config.rs
  - crates/haze-gdrive-adapter/src/runtime.rs
  evidence:
  - AdapterConfig derives Debug and contains drive_root_folder_id as a plain String.
  - AdapterRuntime derives Debug and contains AdapterConfig.
  - Formatting AdapterRuntime or AdapterConfig therefore emits the Google Drive root folder identifier, despite the review contract forbidding provider facts in Debug, Display and log-ready strings.
  - Existing secrecy tests assert token, secret path and adapter identity redaction but do not use a provider-root sentinel.
  impact:
  - Operator diagnostics can disclose a private provider identifier.
  required_fix:
  - Replace derived Debug with explicit fixed/redacted formatting for AdapterConfig and/or AdapterRuntime, including drive_root_folder_id and the raw server endpoint.
  - Add a provider-root sentinel to config/runtime secrecy tests.

- severity: medium
  title: Committed Cargo.lock is stale for the new HTTP client dependencies
  paths:
  - crates/haze-gdrive-adapter/Cargo.toml
  - Cargo.lock
  evidence:
  - Cargo.toml declares haze-sync-api, haze-sync-common, serde_json and ureq dependencies.
  - The exact reviewed Cargo.lock blob 9cfc309d1d18b972b66b35b83c04fcfa79b1057e still records haze-gdrive-adapter with no dependency list.
  - The green workflow runs cargo commands without --locked, so Cargo can repair the lockfile only inside the ephemeral CI checkout.
  impact:
  - The committed dependency graph is not reproducible and a locked build will reject the candidate.
  required_fix:
  - Regenerate and commit Cargo.lock from the final manifest without changing unrelated dependency intent.
  - Re-run complete exact-SHA Component CI after the lockfile-bearing code commit.

PASSED_REVIEW_AREAS:
- required AdapterIdentity uses accepted AdapterId validation and is not inferred from token or provider metadata
- accepted DTO, route and header owner blobs are byte-identical
- bearer and idempotency headers use accepted typed contracts
- after_path uses percent encoding and limit is locally bounded before transport
- mode-aware facade rejects disabled reads and non-mutating commits before delegating to the inner client
- response body, page count and aggregate item count are bounded by policy
- repeated and non-advancing pagination cursors are rejected
- cross-page state format, state version, cursor state, checkpoint and last-operation metadata are compared
- each snapshot is validated and its adapter id must equal configured identity
- 200, 409 and 422 commit outcome shapes are checked against exact allowed key sets and status combinations
- route-error status and code combinations fail closed when unexpected
- transport, response and domain errors expose fixed safe messages instead of dependency errors or response bodies
- redirects are configured as zero in UreqHttpTransport and all returned 3xx responses are rejected
- adapter code performs no explicit compare-and-commit retry; ureq does not replay a non-empty POST body after a response-side connection failure
- no direct DB, provider synchronization, scheduler loop, status-control, CLI, Deployment, sibling or workflow expansion was found

TEST_REVIEW:
fake_only: yes
external_network_or_live_credentials: none found
adequate_coverage:
- accepted GET path/query/auth and snapshot decoding
- accepted POST path/headers/body and all commit response variants
- route error and retry categories represented by tests
- fake transport timeout/unavailable, oversized body and redirect response
- pagination loop/page/item guards and stable collection
- mode-denied zero-call behavior
- redaction for HTTP client/request/response/collection/error surfaces
gaps:
- no concrete Ureq overall-timeout test
- no origin-only base URL edge-case tests
- no provider-root config/runtime secrecy sentinel
- no locked-build evidence for the committed lockfile

CI_EVIDENCE:
workflow: Component CI
run_id: 29507840727
run_number: 2049
job_id: 87653233277
exact_sha: cb9c85169e6f212e11824858501e70b182b26a29
conclusion: success
checks:
- cargo fmt: success
- cargo check: success
- cargo test: success
- cargo clippy: success
- Finalize CI diagnostics: success
- Upload CI diagnostics: skipped because no failures
ci_interpretation: Current tests compile, pass and lint, but do not exercise the concrete overall deadline, path-prefixed base URL, provider-root Debug leak or committed lockfile under --locked behavior.

PR_STATE:
open: yes
draft: yes
merged: no
mergeable: yes

CHANGES_BY_REVIEWER:
product_code_changed: no
tests_changed: no
dependencies_changed: no
docs_changed: no
workflow_changed: no
report_only_changed: yes
ci_skip_used: yes
ci_skip_reason: clean-review report-only commit

BLOCKERS:
- concrete overall request deadline and timeout classification
- exact origin-only route construction
- provider-root Debug redaction
- committed lockfile synchronization

NEXT_RECOMMENDED_AGENT:
fixer-worker

FINAL_VERDICT:
CLEAN_NEEDS_FIX. Exact SHA cb9c85169e6f212e11824858501e70b182b26a29 is green and preserves the accepted API owner blobs, but it does not yet satisfy the bounded concrete HTTP, exact route construction, comprehensive provider-fact secrecy and reproducible dependency requirements. No long-running runtime, deployment or repository merge-readiness claim is made.

PUSHED:
yes
