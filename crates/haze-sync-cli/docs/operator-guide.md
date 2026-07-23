# Haze Sync CLI operator guide

## Safety model

The CLI is a client of accepted public Server surfaces. It does not connect directly to PostgreSQL, object storage, Google Drive, or private Server state.

Current operational plans are no-write guidance, not execution wrappers. They always report `dry_run` and `writes: none`.

## Configuration

Configuration precedence is:

1. command-line global option;
2. environment variable;
3. selected profile in the config file;
4. safe default.

Example environment configuration:

```bash
export HAZE_SYNC_SERVER_URL='http://127.0.0.1:8080'
export HAZE_SYNC_TOKEN_SOURCE='env:HAZE_SYNC_ADMIN_TOKEN'
export HAZE_SYNC_ADMIN_TOKEN='<operator-supplied-secret>'
```

Do not place raw bearer tokens directly in ordinary CLI arguments. Prefer an environment descriptor, protected file, stdin, or an accepted OS secret source.

## Preflight

Run:

```bash
haze-sync preflight
```

The command reads:

- process health;
- Server readiness;
- admin status;
- adapter list;
- hosted Worktree status.

Success requires all included requests to succeed and Worktree to report ready state, zero failed cycles, and no failed lifecycle.

A ready result begins with:

```text
preflight: ready
```

Any blocked or unavailable required surface returns exit code `1` and a sanitized explanation.

## Operational plans

```bash
haze-sync bootstrap plan
haze-sync recovery plan
haze-sync rollout plan
```

These commands:

- do not load a Server token;
- do not contact the network;
- do not write local or remote state;
- do not enable adapters;
- do not run backup or restore;
- do not mutate a vault.

They provide ordered checkpoints for a later contract-backed operation.

`recovery plan` explicitly states that destructive restore requires operator approval. It must not be treated as proof that backup or restore has been executed.

## JSON output

Select JSON globally:

```bash
haze-sync --output json preflight
haze-sync --output json recovery plan
```

Schema:

```text
haze-sync.cli.output.v1
```

Envelope fields:

```text
schema
command
ok
exit_code
stdout
stderr
```

The process exit code remains authoritative. JSON mode serializes both safe output channels into one stdout envelope so scripts can parse a deterministic document.

## Exit codes

```text
0  success
1  runtime/config/auth/network/readiness/preflight failure
2  usage or parse failure
```

## Worktree commands

```bash
haze-sync worktree status
haze-sync worktree sync-once
```

`sync-once` is not a general force-sync command. It requests the bounded Server-owned operation and succeeds only when the Server exposes it for the configured Worktree mode. In production bidirectional mode, automatic cycles remain the accepted path.

## Current limitations

The CLI does not currently execute:

- GDrive dry-run/import/export;
- adapter mode changes;
- coordinated backup or restore;
- conflict mutation;
- token rotation;
- destructive repair;
- deployment start/stop.

Those operations remain blocked until accepted upstream contracts and separate execution evidence exist.
