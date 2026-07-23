# Stage 9 Operational CLI Review

Date: 2026-07-23  
Integration branch: `integration/v1-fan-in`

## Verdict

```text
STAGE_9_CLI_PREFLIGHT: PASS
STAGE_9_CLI_JSON_V1: PASS
STAGE_9_CLI_GUARDED_PLANS: PASS
STAGE_9_LIVE_CLI_TOPOLOGY: PASS
R6_3_OPERATIONAL_CLI_CLOSURE: PARTIAL
LIVE_BACKUP_RESTORE_WRAPPERS: BLOCKED_BY_UPSTREAM_CONTRACT
LIVE_ADAPTER_MODE_MUTATION: BLOCKED_BY_UPSTREAM_CONTRACT
LIVE_GDRIVE_OPERATION_COMMANDS: BLOCKED_BY_UPSTREAM_CONTRACT
V1_COMPLETION: BLOCKED
ROLLOUT_READINESS: BLOCKED
PR_68_STATE: KEEP_DRAFT
```

Stage 9 closes the safe CLI surface that can be implemented against accepted public Server contracts. It does not claim full R6-3 closure because the repository does not expose accepted public contracts for coordinated backup/restore, adapter enable/disable or mode mutation, GDrive execution, conflict mutation, token rotation, or destructive recovery.

## Accepted implementation

Initial integration commit:

```text
e4bc6d7712bacf5cc60129a481b2289ac9c4b676
feat(cli): integrate Stage 9 operational safeguards
```

Verified permanent code head before this review:

```text
230439f21607b384256bea9fa22c0eba99b1f376
style(cli): apply verified rustfmt output
```

Accepted files:

- `crates/haze-sync-cli/src/commands.rs`
- `crates/haze-sync-cli/src/main.rs`
- `crates/haze-sync-cli/src/operations.rs`
- `crates/haze-sync-cli/src/output.rs`
- `crates/haze-sync-cli/docs/component-contract.md`
- `crates/haze-sync-cli/docs/operator-guide.md`
- `.github/scripts/local-vertical-slice.sh`
- `.github/workflows/integration-hardening-ci.yml`

No direct database, object-store, provider, or private Server access was added to the CLI.

## Accepted command surface

Stage 9 adds:

```text
haze-sync preflight
haze-sync bootstrap plan
haze-sync recovery plan
haze-sync rollout plan
```

Existing status, doctor, adapter, and Worktree commands remain available.

### Preflight

`preflight` uses the authenticated public Server client and aggregates:

- `GET /health`;
- `GET /ready`;
- `GET /v1/admin/status`;
- `GET /v1/admin/adapters`;
- `GET /v1/admin/worktree/status`.

A successful verdict requires:

- every required request to succeed;
- ready Server and dependency state;
- ready Worktree state;
- zero failed Worktree cycles;
- no failed Worktree lifecycle.

Unavailable or unsafe state returns process exit code `1` with sanitized partial evidence.

### Stable JSON output

Machine-readable output uses:

```text
haze-sync.cli.output.v1
```

Exact envelope fields:

```text
schema
command
ok
exit_code
stdout
stderr
```

The envelope preserves the authoritative process exit code and sanitizes both output channels. Parse errors can also use the JSON envelope when JSON was selected explicitly.

### Guarded operational plans

Bootstrap, recovery, and rollout plans are offline no-write commands. Each states:

```text
dry_run
writes: none
```

`recovery plan` additionally states that destructive restore requires explicit operator approval.

The plans do not:

- read tokens or config files;
- access the network;
- change Server, PostgreSQL, object storage, provider state, adapter modes, deployment state, or vault files;
- claim that backup, restore, rollout, or rollback was executed.

## Live evidence

The permanent local vertical-slice job now validates:

```text
CLI config/token source
→ authenticated CLI preflight
→ human and JSON output assertions
→ guarded no-write plan assertions
→ PostgreSQL + Server
→ automatic Worktree export/import
→ production Obsidian client readback/change feed
```

It verifies:

- locked CLI build;
- human `preflight: ready`;
- exact JSON v1 field set and command identity;
- `ok=true`, `exit_code=0`, and empty JSON stderr on success;
- recovery plan no-write and explicit-approval text;
- all three plan commands remain no-write;
- CLI and Server outputs do not contain token markers;
- existing Worktree and Obsidian vertical-slice behavior remains green.

## Verification evidence

### Isolated Stage 9 verifier

Candidate head:

```text
3899add83edc05ca455e65201309eaf249f6e3fc
```

Runs:

```text
Integration Hardening CI  29996877315  success
Integration Node CI       29996877392  success
```

Hardening jobs:

- Locked Rust workspace — success;
- Deployment Compose integration — success;
- Worktree + Obsidian + CLI local vertical slice — success.

### Permanent exact-head verification

Verified head:

```text
230439f21607b384256bea9fa22c0eba99b1f376
```

Runs:

```text
Integration CI            29998956949  success
Integration Node CI       29998956966  success
Integration Hardening CI  29998956777  success
Component CI              29998957092  skipped
```

The skipped legacy Component CI is not counted as evidence.

Integration CI passed:

- `cargo fmt`;
- workspace check, tests, and clippy;
- strict Storage PostgreSQL tests and migration evidence;
- full Server PostgreSQL tests, private cursor evidence, build, health/readiness, and graceful shutdown.

Hardening passed:

- locked metadata/check/test/clippy;
- deployment Compose smoke;
- Worktree + Obsidian + CLI live local vertical slice.

## Formatting correction

The first permanent exact-head run exposed only `cargo fmt --check` differences. Check, tests, clippy, Storage, Server, Node, and live topology were otherwise successful.

A temporary read-only GitHub Actions helper ran `cargo fmt --all`, verified that only these files changed, and exported them as an artifact:

- `crates/haze-sync-cli/src/commands.rs`;
- `crates/haze-sync-cli/src/main.rs`;
- `crates/haze-sync-cli/src/operations.rs`.

The artifact was imported through Git blobs. The helper workflow was deleted in the same permanent formatting commit and is absent from the verified tree.

## Remaining R6-3 boundaries

Full R6-3 still requires accepted upstream contracts and later implementation/evidence for:

1. coordinated backup and restore execution;
2. adapter enable/disable and mode mutation;
3. GDrive dry-run/import/export execution;
4. conflict-resolution mutation;
5. token creation and rotation;
6. destructive repair or cleanup;
7. packaging, release publication, signing, and checksum consumption.

The CLI must not bypass these missing contracts through direct SQL, object-store access, provider internals, or deployment shell mutation.

## Cleanup and review state

- Temporary verifier workflow files are absent from the permanent integration tree.
- Permanent Integration Hardening CI is restricted to pull requests targeting `main` and has `contents: read`.
- No force update or history rewrite was used.
- Temporary PR #79 must be closed without merge after final exact-head evidence is recorded.
- PR #68 remains open and draft.
- This stage does not establish V1 completion or rollout readiness.
