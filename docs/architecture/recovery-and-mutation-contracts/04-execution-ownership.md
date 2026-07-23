# Recovery and Mutation Contracts — Execution Ownership

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

## 4. Canonical recovery execution ownership

The canonical chain is:

```text
CLI
  -> authenticated versioned Server operation request
  -> durable `haze-sync.operational-job.v1` validation and reservation
  -> deployment-owned recovery helper
  -> typed PostgreSQL/object-store/optional-Worktree artifact results
  -> Server job/audit/evidence finalization
  -> safe CLI result
```

### 4.1 CLI ownership

CLI MUST:

- submit only versioned structured requests to an accepted public administrative/operation boundary;
- send an idempotency key through the accepted protected transport field/header;
- render safe job status, confirmation requirements, manifest identity, and result categories;
- require an explicit operator confirmation workflow for destructive operations;
- default planning and inspection commands to no-write behavior.

CLI MUST NOT:

- execute SQL or invoke `pg_dump`, `pg_restore`, migration tools, or database reset commands directly;
- read or mutate object-store files directly;
- access provider-private APIs or private adapter state;
- mutate deployment services directly;
- accept or construct arbitrary shell commands;
- pass secret values as flags, positional arguments, output, or logs;
- claim backup, restore, upgrade, or rollback success from a plan-only response.

### 4.2 Server/operation ownership

Server MUST:

- authenticate and authorize the operator;
- validate the versioned request and named deployment resources;
- enforce maintenance-state admission;
- create/replay the exact Stage 10 operational job under the operational-idempotency contract;
- require and validate destructive confirmation where applicable;
- invoke only an allowlisted helper operation;
- receive typed progress/result evidence;
- persist canonical job and `haze-sync.audit-event.v1` state, or reconcile it from the external recovery-control envelope when the product PostgreSQL target is unavailable;
- expose safe status and result categories.

Server MUST NOT:

- accept an arbitrary executable path, shell fragment, SQL fragment, host path, or provider command from CLI;
- treat helper exit code alone as proof of success;
- mark a job successful before manifest and verification evidence are durable and re-readable;
- enable adapters or resume public mutation automatically after restore, upgrade, or rollback.

### 4.3 Deployment-owned helper

The recovery helper is the only component authorized to perform validated host-level backup/restore work.

The helper MUST be invoked through a fixed deployment configuration and a typed operation enum. Acceptable transports include a fixed executable receiving JSON on standard input or an authenticated local RPC boundary. The transport MUST NOT evaluate a shell command string.

The helper MUST:

- accept only schema-validated V1 requests;
- resolve named destinations/targets through deployment configuration;
- enforce path allowlists and target identity;
- enforce required filesystem permissions;
- run only allowlisted backup, verification, restore, and migration primitives;
- produce typed progress and final result records;
- sanitize all errors before returning them;
- preserve a local detailed log only when it is access-controlled and secret-safe.

The helper MUST NOT:

- execute arbitrary operator-provided shell;
- expose host paths to CLI/public API;
- read provider-private data unless a separate provider-specific contract explicitly authorizes it;
- use automatic destructive fallback;
- infer a successful job from partial artifacts.
