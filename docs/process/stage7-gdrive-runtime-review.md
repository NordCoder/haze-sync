# Haze Sync V1 — Stage 7 GDrive Runtime Review

## Scope

Stage 7 addresses the first bounded part of `R6-1 — GDrive runtime closure` from the Stage 6 completion review.

Accepted scope:

- a cooperative long-running adapter process;
- deterministic full-scan and incremental-cycle scheduling;
- stable cycle identity across retry and restart metadata;
- bounded retry backoff;
- durable-checkpoint enforcement for mutation-capable modes;
- safe shutdown signaling;
- an opt-in, secret-backed, non-root GDrive deployment service;
- disabled-mode Compose acceptance.

Explicitly excluded from this stage:

- live Google OAuth authorization or refresh acceptance;
- concrete provider-to-Core mutation-cycle composition;
- live import-only, export-only, or bidirectional execution;
- Drive watch-channel registration and renewal;
- Google sandbox or real-vault rollout.

## Verdict

```text
STAGE_7_RUNTIME_SCHEDULER: PASS
STAGE_7_DISABLED_DEPLOYMENT: PASS
R6_1_GDRIVE_RUNTIME_CLOSURE: PARTIAL
V1_COMPLETION: BLOCKED
ROLLOUT_READINESS: BLOCKED
PR_68_STATE: KEEP_DRAFT
```

## Runtime behavior accepted

The adapter runtime now owns:

- lifecycle states and cooperative termination;
- first-cycle full scan scheduling;
- periodic full-scan cadence with incremental cycles between scans;
- stable cycle IDs reused after retryable failures;
- bounded exponential backoff;
- resume metadata for monotonic cycle identity after restart;
- aggregated safe operation counters;
- a hard requirement that successful mutation-capable cycles report a committed durable checkpoint.

Disabled mode remains an inert long-running service and does not call Google or Core.

Enabled production modes use a guarded executor until concrete provider/Core composition is published. The guard returns a safe fatal `CompositionUnavailable` error. It must not report a false successful synchronization cycle.

## Deployment behavior accepted

The opt-in GDrive Compose override provides:

- a separate non-root image running as UID/GID `10002`;
- Rust 1.88 release build with `cargo build --release --locked`;
- adapter and OAuth values delivered through mounted Compose secrets;
- read-only root filesystem;
- tmpfs for temporary data;
- all Linux capabilities dropped;
- `no-new-privileges`;
- dependency on a healthy Haze Sync Server;
- disabled mode as the default;
- explicit stop grace period and process health monitoring.

The base Compose topology is unchanged. GDrive is available only through the explicit profile/override.

## Verification evidence

The isolated Stage 7 verifier succeeded for candidate `d5f9654df6582ed2b0e109b2e67a7137aed0a946` in workflow run `29967588887`.

It performed:

- `cargo fmt --all`;
- canonical `Cargo.lock` regeneration;
- `cargo check --workspace --locked`;
- `cargo test --workspace --exclude haze-sync-server --locked`;
- `cargo clippy --workspace --all-targets --locked -- -D warnings`;
- `cargo test -p haze-gdrive-adapter --locked`;
- `cargo clippy -p haze-gdrive-adapter --all-targets --locked -- -D warnings`.

The verifier published only byte-preserving content to an isolated branch. Final exact-head Integration, Node, and Hardening workflow evidence is recorded in PR #68 after publication.

## Remaining R6-1 work

Stage 7 does not close `R6-1`. The remaining work is:

1. compose `GoogleDriveHttpClient`, OAuth refresh, durable-state client, full scan, change feed, Core export, echo guard, and delete guard into the cycle executor;
2. persist and restore provider/Core checkpoints through live successful cycles;
3. accept import-only, export-only, dry-run, and bidirectional behavior through fake-provider end-to-end tests;
4. implement watch registration/renewal with polling fallback;
5. execute dedicated Google test-folder OAuth and provider acceptance;
6. add secret-safe provider readiness and doctor evidence for enabled mode.

## Release disposition

- Keep PR #68 open and draft.
- Do not describe Stage 7 as full GDrive runtime completion.
- Do not enable a mutation-capable GDrive mode in deployment.
- Do not perform Google sandbox or real-vault rollout under this stage.
