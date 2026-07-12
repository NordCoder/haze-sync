# W1-STOR-P10-SRV-CONTRACT-HOLD — Await Server clean acceptance

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: none
Phase: STOR-P10-SRV-CONTRACT-HOLD

This is a hold notice, not an executable worker prompt.

## Storage status

The complete STOR-P10 Storage-local implementation is clean and fully evidenced at code-bearing SHA:

`66b6a1f554aae1d1b774cc88560d46dd140c7a54`

Authoritative Component CI:

- run id: `29185466870`;
- run number: `1833`;
- conclusion: `success`;
- Rust workspace: success;
- Storage PostgreSQL verification: success;
- strict ignored-test command: success;
- all five mandatory STOR-P10 evidence checks: success;
- both diagnostics finalizers: success.

The final Storage review confirmed migration 0010 safety, the direct savepoint-backed migration guard test, repository and cursor transaction semantics, secrecy, bounded snapshots, and formatter-only nature of the last correction.

## Why acceptance is held

The final reviewer inspected the stale Server copy carried by `component/storage` and found:

`haze-sync-storage = { path = "../haze-sync-storage", features = ["test-support"] }`

inside Server normal dependencies. Storage contract explicitly states that production crates must not enable `test-support` in normal dependencies.

This is a Server-owned cross-component contract issue and cannot be corrected from the Storage component branch.

## Current Server-owner evidence

The current Server candidate already contains the correct dependency split at exact code-bearing SHA:

`647dce7b624d67663632808906896cb6745ea7e7`

At that SHA:

- normal `[dependencies]` uses `haze-sync-storage = { path = "../haze-sync-storage" }` with no `test-support` feature;
- `[dev-dependencies]` enables `haze-sync-storage` with `features = ["test-support"]`;
- Component CI run `29186058268`, run number `1835`, completed successfully;
- PostgreSQL services, fmt, check, isolated Server tests, isolated Storage tests, remaining workspace tests, clippy and diagnostics finalizer all passed.

The correction is therefore implemented and green in the Server owner branch, but it has not yet received mandatory SRV-P7B2 clean-code acceptance.

## Hold condition

Do not launch a Storage worker from this notice.

STOR-P10 remains `CLEAN_BLOCKED_BY_CONTRACT` until:

1. `SRV-P7B2` receives `CLEAN_ACCEPT` on an exact Server code-bearing SHA that preserves the correct Storage dependency gating;
2. the accepted Server report explicitly confirms that `test-support` is absent from the normal production dependency graph and present only in dev/test scope;
3. the Orchestrator reactivates Storage for final cross-branch accepted-SHA confirmation.

## Downstream block

`SRV-P7B3` must not begin until WT-P10, STOR-P10 and SRV-P7B2 are all `CLEAN_ACCEPT` and their exact accepted SHAs are synchronized into the Server integration scope.