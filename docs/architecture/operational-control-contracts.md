# Haze Sync V1 Operational Control Contracts

Status: normative V1 contract  
Contract version: `haze-sync.operational-control.v1`  
Compatibility baseline: `integration/v1-fan-in@59f3cd1f1df6f923083cf42c578e3ecf9f3e49c4`

## 1. Purpose and authority

This document defines the normative operational-control behavior required before Haze Sync may implement maintenance, adapter control, credential administration, or destructive operational work.

The contract covers:

- maintenance and quiescence;
- desired-versus-effective adapter control;
- credential records and lifecycle;
- versioned operational jobs;
- operational idempotency;
- operational audit events;
- downstream ownership and compatibility boundaries.

This document defines behavior only. It does not add HTTP routes, storage schema, runtime code, CLI commands, deployment wiring, or provider integration.

The following terms are normative:

- **MUST** and **MUST NOT** are required for data safety, interoperability, or truthful status.
- **SHOULD** and **SHOULD NOT** are the expected implementation unless a documented safety-equivalent reason exists.
- **MAY** is optional behavior that does not weaken another requirement.

## 2. Compatibility and non-redefinition rules

This contract extends operational control without redefining accepted sync semantics.

Implementations MUST preserve these existing invariants:

1. Core remains the source of truth and the only conflict, revision, and delete-policy arbiter.
2. Adapters submit facts; they do not gain overwrite authority from an operational mode.
3. Every content write or delete retains a known base revision or explicit null-base semantics.
4. Unknown or stale base revisions do not permit silent overwrite.
5. Deletes remain tombstone, trash, and retention oriented; operational control does not introduce immediate hard delete.
6. Conflicts preserve both sides by default and remain separate from maintenance state.
7. Existing adapter mode wire values remain:
   - `disabled`;
   - `read_only`;
   - `import_only`;
   - `export_only`;
   - `bidirectional`;
   - `dry_run`.
8. Existing adapter write idempotency remains adapter-scoped and is not replaced by operational-job idempotency.
9. Existing public health, readiness, sync, conflict, file, change-feed, and read-only administrative behavior remains compatible unless a later owner phase explicitly versions a public contract.
10. The CLI remains an authenticated public Server client. It MUST NOT compensate for missing operational APIs with direct SQL, object-store access, provider-private calls, arbitrary deployment shell access, or secret disclosure.

Operational state MUST NOT be interpreted as proof that replicas are fully synchronized. An applied mode, recent heartbeat, or successful cycle proves only the specific fact named by that field.

## 3. Common control primitives

### 3.1 Durable records and versioning

Every durable operational-control record MUST contain a schema identifier or integer `schema_version`. Readers MUST reject unsupported future schemas rather than guessing.

`schema_version` describes data shape and MUST NOT be used as a compare-and-set token. Every mutable record MUST additionally expose one explicit mutable CAS token whose name identifies its namespace:

```text
maintenance_generation                 global maintenance record
adapter_control_generation             one centrally controlled adapter record
principal_version                      one principal record
credential_set_generation              the credential set owned by one principal
credential_version                     one credential record
job_version                            one operational job record
executor_fence                         one operational-job executor epoch
adapter_inventory_generation           the configured centrally controlled adapter inventory
```

Identifiers MUST be opaque, stable, and non-secret. Operational identifiers SHOULD use the repository's accepted identifier conventions where applicable.

Every generation, version, and fence value above MUST be a non-negative monotonically increasing integer. A value MUST NOT be reused, decremented, or reset after restart, restore, failover, archival, or lease takeover. Exhaustion or overflow MUST fail closed.

The name `maintenance_generation` always means the current value from the singleton maintenance record. `maintenance_control_generation` and an unqualified job field named `expected_control_generation` are not separate namespaces and MUST NOT appear in durable V1 records. An adapter control record copies the exact global value into its `maintenance_generation`; an effective adapter report copies that value into `last_applied_maintenance_generation`; a job that depends on maintenance stores it as `expected_maintenance_generation`.

All timestamps MUST be UTC instants. Comparisons that affect admission, expiry, grace, lease validity, or confirmation MUST use Server-controlled time.

### 3.2 Compare-and-set requirement

Any request that changes maintenance state, adapter desired state, principal or credential lifecycle, or a non-terminal operational job MUST be applied through a durable transaction with the exact current CAS token for every mutable record or set it changes.

Required CAS inputs are:

- maintenance action: `expected_maintenance_generation`;
- adapter desired-state action: `expected_adapter_control_generation`;
- principal enable/disable or credential-set mutation: `expected_principal_version` and `expected_credential_set_generation`;
- single-credential mutation: `expected_credential_version` plus the principal credential-set CAS when set invariants may change;
- job mutation: `expected_job_version`; executor-owned updates additionally require the current `executor_fence` and lease proof.

A stale generation MUST be rejected with safe code `stale_control_generation`. A stale record/set version MUST be rejected with safe code `stale_record_version`. A stale executor epoch MUST be rejected with safe code `stale_executor_fence`. A stale request MUST NOT partially change state, allocate a new generation, acquire a lease, or start an external side effect.

An exact replay identified by the same idempotency scope, key, and request fingerprint returns the existing result and MUST NOT allocate a new generation or version. Replay permits observation of the stored result; it does not authorize a new mutation against stale CAS input.

### 3.3 Safe errors

Operational errors MUST expose only stable categories and safe summaries. They MUST NOT expose:

- bearer credentials, credential verifiers, salts, peppers, or token hashes;
- OAuth access tokens, refresh tokens, client secrets, or provider credentials;
- raw `Idempotency-Key` or confirmation values;
- database URLs, SQL, SQLx/driver errors, or stack traces;
- raw provider requests, responses, cursors, or private provider payloads;
- raw file content or request bodies;
- local absolute paths;
- arbitrary deployment command output.

## 4. Maintenance and quiescence

### 4.1 Canonical state machine

The canonical forward state machine is:

```text
normal
  -> quiescing
  -> quiesced
  -> maintenance
  -> resuming
  -> normal
```

A system MAY omit `maintenance` when a quiesce request is cancelled safely before maintenance work begins:

```text
quiesced -> resuming -> normal
```

No client may write a raw state value. Clients request the actions `quiesce`, `enter_maintenance`, or `resume`; Server owns the actual state transitions.

### 4.2 Maintenance control record

The durable maintenance record MUST contain at least:

```text
schema_version
maintenance_generation
state
transition_operation_id
transition_requested_by
transition_requested_at
state_entered_at
admission_fence_closed
quiescence_evidence_version
quiescence_evidence_id | null
active_maintenance_job_id | null
safe_error_category | null
updated_at
```

`admission_fence_closed` MUST be persisted as `true` in the same transaction that accepts `normal -> quiescing`, before the request is acknowledged as accepted.

`maintenance_generation` MUST increment for every accepted state-changing request. Automatic completion of an already accepted transition, such as `quiescing -> quiesced`, preserves the request's generation and records a new state transition event.

### 4.3 Legal, idempotent, rejected, and invalid transitions

| Current state | Request | Result |
|---|---|---|
| `normal` | `quiesce` | Accept; atomically close admission, increment generation, enter `quiescing`. |
| `normal` | `enter_maintenance` | Reject `invalid_control_transition`; maintenance requires completed quiescence. |
| `normal` | `resume` | Idempotent no-op only when expected generation matches; remain `normal`. |
| `quiescing` | exact replay of accepted `quiesce` | Return the existing transition; do not increment generation. |
| `quiescing` | new `quiesce`, `enter_maintenance`, or `resume` | Reject `control_transition_in_progress`. |
| `quiesced` | `quiesce` | Idempotent no-op when expected generation matches; return current evidence identity. |
| `quiesced` | `enter_maintenance` | Accept; increment generation and enter `maintenance`. |
| `quiesced` | `resume` | Accept; increment generation and enter `resuming`. |
| `maintenance` | `quiesce` or `enter_maintenance` | Idempotent no-op when expected generation matches; remain `maintenance`. |
| `maintenance` | `resume` | Accept only when no running maintenance-required job or unresolved external-side-effect outcome exists; increment generation and enter `resuming`. |
| `resuming` | exact replay of accepted `resume` | Return the existing transition; do not increment generation. |
| `resuming` | new `quiesce`, `enter_maintenance`, or `resume` | Reject `control_transition_in_progress`. |

Any raw or unlisted transition is invalid. An invalid transition MUST NOT change the generation.

A request with a stale expected generation is rejected before idempotent state comparison, except an exact stored idempotency replay. This prevents an old operator view from accidentally treating a materially newer state as the requested no-op.

### 4.4 Concurrent requests

Maintenance control has one durable serialization point.

- Concurrent requests with different idempotency keys race on the expected generation; at most one is accepted.
- Concurrent requests with the same key and identical fingerprint return one operation identity.
- Reuse of the same key with a different action, expected generation, or target parameters returns `idempotency_conflict`.
- A later request MUST read the current state and generation rather than infer completion from a timed-out client response.
- A disconnected caller MUST NOT cause Server to roll back an accepted transition.

### 4.5 Mutation admission matrix

The maintenance fence is enforced at the Server application-service boundary, not only in HTTP routing. Hosted adapters, HTTP routes, and future internal callers MUST share the same admission decision.

Legend:

- **allow** — request may proceed subject to its normal auth and sync policy;
- **read only** — no authoritative or replica mutation may be created;
- **drain** — no new work is admitted; work accepted before the fence follows the quiescence rules;
- **reject** — return a safe maintenance error without mutation.

| Operation class | `normal` | `quiescing` | `quiesced` | `maintenance` | `resuming` |
|---|---|---|---|---|---|
| Public file PUT/DELETE | allow | reject new; drain accepted work | reject | reject | reject |
| Conflict-resolution mutation | allow | reject new; drain accepted work | reject | reject | reject |
| Adapter cursor/state commit that asserts completed sync work | allow | reject new; drain accepted work | reject | reject | reject |
| Exact replay of an already terminal public mutation | allow | allow read-only replay | allow read-only replay | reject | reject |
| Public metadata/content reads | allow | allow | allow | reject unless a job-specific immutable snapshot contract authorizes the read | reject |
| Change-feed reads | allow | allow | allow from the stable pre-maintenance log | reject | reject |
| Hosted Worktree automatic/manual mutating cycles | allow by effective mode | stop scheduling; cancel/drain current | reject | reject | reject until resume control is applied |
| Standalone GDrive mutating cycles | allow by effective mode | apply control hold; cancel/drain current | reject | reject | reject until resume control is applied |
| Obsidian plugin local edits | device-owned; Server admits API writes | device-owned; Server rejects new API writes | device-owned; Server rejects writes | device-owned; Server rejects writes | device-owned; Server rejects writes |
| Administrative status/control reads | allow | allow | allow | allow when control storage is readable | allow |
| Credential create/rotate | allow | reject | allow | allow | reject |
| Credential revoke | allow | allow | allow | allow | allow |
| Operational-job planning/dry-run metadata | allow if kind permits | allow only if it cannot delay drain | allow | allow | reject new plans |
| Maintenance-required operational-job execution | reject | reject | reject until `maintenance` | allow with confirmation and generation checks | reject |
| Health/liveness | report process liveness | report process liveness | report process liveness | report process liveness | report process liveness |
| Readiness | normal dependency/readiness rules | not ready | not ready | not ready | not ready |
| Shutdown and safe operator control | allow | allow | allow | allow | allow |

The public maintenance rejection SHOULD use HTTP `503 Service Unavailable` with safe code `maintenance_in_progress` or a more specific transitional code. It MUST NOT claim that a rejected mutation was queued unless a later public API explicitly implements a durable queue.

`GET /health` remains a liveness signal. It SHOULD remain successful while the process is alive even when maintenance is active. `GET /ready` MUST report not-ready in every state except `normal`.

### 4.6 Quiescing behavior

On entry to `quiescing`, Server MUST:

1. persist the closed admission fence;
2. stop scheduling new hosted Worktree cycles and manual mutations;
3. publish a new per-adapter control generation that includes the maintenance hold;
4. request cooperative cancellation of in-flight hosted adapter work;
5. continue tracking all mutations admitted before the fence;
6. wait for every authoritative transaction to commit completely or roll back;
7. wait for required external adapter acknowledgements or establish an explicit fence;
8. persist versioned quiescence evidence before entering `quiesced`.

A timeout MUST NOT reopen admission and MUST NOT produce `quiesced`. The state remains `quiescing`, readiness remains false, and the safe error category records the incomplete class. The same transition operation may be retried or continued after restart.

### 4.7 In-flight drain and cancellation

A mutation counts as in flight from successful admission until its authoritative transaction and required durable side effects have reached one of these outcomes:

- complete success with all associated checkpoints persisted;
- safe idempotent replay of a previously completed result;
- complete rollback with no success/checkpoint claim;
- explicit failed outcome whose partial external effects are recorded for an operational job.

For Worktree:

- cooperative cancellation MUST be checked between phases and bounded items;
- one atomic filesystem operation already in progress MAY finish;
- no later phase may begin after cancellation is observed;
- an open SQL transaction commits only on complete success and otherwise rolls back;
- a cancelled or partial cycle MUST NOT advance the authoritative cursor or claim accepted path state.

For GDrive:

- no new provider or Core mutation may begin after the hold generation is applied;
- an in-flight provider operation must finish, be cancelled, or become an explicitly recorded uncertain external effect;
- a cursor or mapping checkpoint advances only after the corresponding provider/Core effect is confirmed;
- rate-limit backoff or process disconnection is not drain evidence.

For public mutations and Obsidian requests:

- requests admitted before the fence may finish under normal Core policy;
- requests arriving after the fence are rejected;
- Obsidian local edits may continue on the device and are not evidence that the authoritative system is unquiesced;
- after resume, queued device facts still use normal base/null-base and conflict semantics.

### 4.8 Quiescence evidence

A `quiesced` claim requires one immutable evidence record containing at least:

```text
schema_version
quiescence_evidence_id
maintenance_generation
adapter_inventory_generation
admission_fence_closed_at
server_instance_id
server_started_at
active_authoritative_mutations = 0
open_authoritative_transactions = 0
adapter_instances[]
obsidian_authoritative_mutation_gate = closed
captured_at
```

`adapter_instances` MUST contain exactly one identity-complete entry for every centrally controlled adapter instance present in the durable adapter inventory captured at `adapter_inventory_generation`. The inventory includes every configured hosted Worktree instance and every configured standalone GDrive instance, including desired-disabled instances. `adapter_inventory_generation` increments whenever such an instance is added, removed, replaced, or changes stable control identity/kind. Duplicate, missing, unknown, or subsequently added/removed adapter identities invalidate the evidence.

Each entry MUST contain at least:

```text
adapter_id
adapter_kind
control_authority
adapter_control_generation
maintenance_generation
desired_enabled
desired_mode
last_applied_adapter_control_generation | null
last_applied_maintenance_generation | null
runtime_lifecycle
connection_state
in_flight = false
drain_proof = acknowledged | externally_fenced
external_fence_id | null
checkpoint_summary
captured_at
```

For `drain_proof = acknowledged`, both last-applied generations MUST equal the desired record values, the current maintenance hold MUST be applied, and `in_flight` MUST be false. For `drain_proof = externally_fenced`, `external_fence_id` MUST identify immutable evidence that prevents that exact `adapter_id` from starting Core or replica mutations; a fence for one adapter identity MUST NOT satisfy another.

`checkpoint_summary` MUST identify the last durable operation/cursor generations needed to detect later drift for that adapter identity. It MUST NOT contain raw provider cursors, credentials, or file content.

A standalone GDrive instance is quiesced only when one of these is true:

1. that exact `adapter_id` has acknowledged its current `adapter_control_generation` and the current `maintenance_generation`, and reported no in-flight provider/Core mutation; or
2. an operator has established a versioned external fence for that exact `adapter_id` by stopping its process or exclusive runtime lease, revoking every Haze Sync credential that could mutate as that principal, and waiting until any bounded credential/cache validity has expired.

A missing heartbeat, stale status, disconnected process, desired-disabled state, or fence for a different adapter by itself is not drain evidence.

Evidence becomes invalid before maintenance entry if the global `maintenance_generation`, `adapter_inventory_generation`, any included adapter desired record, any included external fence, or the admission fence changes. Server MUST re-read and compare all of those identities transactionally before accepting `quiesced -> maintenance` or starting a maintenance-required job.

Obsidian devices are not centrally frozen and are not members of `adapter_instances`. Quiescence evidence asserts only that Server rejects their authoritative mutations. Destructive recovery MUST NOT assume that remote Obsidian vaults stopped changing or constitute a current backup.

### 4.9 Entry into maintenance

`quiesced -> maintenance` is allowed only when:

- quiescence evidence exists for the current maintenance generation;
- the admission fence is still closed;
- no drift invalidates the evidence;
- no conflicting control transition is active.

Before any destructive job starts, Server MUST revalidate the evidence and persist the job's expected maintenance generation. A mismatch returns `stale_control_generation` and performs no destructive side effect.

### 4.10 Resume behavior

On entry to `resuming`, Server MUST keep the admission fence closed.

Server MUST:

1. verify there is no running maintenance-required job and no unresolved uncertain external outcome;
2. validate durable schema and required dependencies;
3. publish new adapter control generations that remove the maintenance hold while preserving desired settings;
4. start the hosted Worktree runtime when desired enabled;
5. wait for every desired-enabled centrally controlled adapter identity to apply its current adapter control and maintenance generations;
6. verify no required runtime is failed or disconnected;
7. reopen admission and enter `normal` atomically.

If an enabled standalone GDrive adapter cannot apply the resume generation, the system remains `resuming`. An operator may explicitly disable that adapter through a new desired-state generation, after which resume may continue. Obsidian devices are not part of the resume acknowledgement set.

A resume timeout MUST leave the fence closed and the state `resuming`. It MUST NOT claim normal readiness.

### 4.11 Restart and recovery by state

On every Server start, the durable maintenance record MUST be loaded before mutation routes or hosted adapters become available.

| Persisted state | Required restart behavior |
|---|---|
| `normal` | Apply normal admission only after required dependencies are ready. Reconcile adapter desired/effective generations before reporting readiness. |
| `quiescing` | Keep the fence closed, do not schedule new work, re-enumerate in-flight durable work, reissue the current hold generation, and continue evidence collection. Never infer quiescence from process restart. |
| `quiesced` | Keep the fence closed and adapters held. Revalidate evidence before allowing maintenance entry; regenerate evidence if process identity or checkpoints changed. |
| `maintenance` | Keep the fence closed. Recover operational jobs according to their job contract. Never auto-resume. |
| `resuming` | Keep the fence closed, reapply the resume generation, and continue readiness checks. Enter `normal` only after all resume conditions are true. |

A missing, unreadable, contradictory, or unsupported maintenance record MUST fail closed: mutation admission remains closed and readiness remains false.

## 5. Adapter desired and effective control

### 5.1 Separation of concerns

Adapter control has three separate concepts:

1. **Desired state** — the operator's current requested enablement and mode.
2. **Effective state** — what a runtime last proved it applied.
3. **Mutation admission** — whether the global maintenance fence currently allows authoritative or replica mutation.

Desired state MUST NOT be overwritten merely because maintenance temporarily blocks work. Maintenance is an overlay expressed in the adapter control snapshot and generation.

Runtime enablement is distinct from credential/principal enablement. Setting `desired_enabled = false` stops sync work but MUST NOT implicitly revoke the credential needed by a standalone GDrive process to poll control and report status. Credential revocation is a separate explicit action.

### 5.2 Desired control record

Each centrally controlled adapter record MUST contain at least:

```text
schema_version
adapter_id
adapter_kind
control_authority
desired_enabled
desired_mode
adapter_control_generation
maintenance_generation
maintenance_hold
updated_by
updated_at
```

Invariants:

- `desired_enabled = false` requires `desired_mode = disabled`.
- `desired_enabled = true` requires a non-`disabled` mode valid for the adapter kind.
- Changing enablement, mode, or maintenance hold increments `adapter_control_generation`.
- `maintenance_generation` MUST equal the singleton maintenance generation whose hold value was used to build this snapshot; it is not independently incremented by the adapter record.
- Rewriting identical desired values with the current `expected_adapter_control_generation` is an idempotent no-op.
- A stale adapter control generation is rejected.

### 5.3 Effective runtime record

Each runtime status record MUST contain at least:

```text
schema_version
adapter_id
effective_mode | null
runtime_lifecycle
last_applied_adapter_control_generation
last_applied_maintenance_generation
heartbeat_at | null
last_success_at | null
last_cycle_started_at | null
last_cycle_finished_at | null
in_flight
connection_state
safe_error_category | null
safe_error_code | null
reported_at
```

`effective_mode = null` means no effective mode has ever been proven. It MUST NOT be rendered as `disabled`, ready, or successful.

Allowed `runtime_lifecycle` values are:

```text
stopped
starting
idle
running
draining
backing_off
failed
```

`last_applied_adapter_control_generation` MUST NOT exceed the desired `adapter_control_generation`; `last_applied_maintenance_generation` MUST NOT exceed the singleton `maintenance_generation`.

`heartbeat_at` proves only recent control-plane contact. `last_success_at` proves only that one complete cycle succeeded at that time. Neither proves zero replica lag, complete reconciliation, or readiness.

### 5.4 Connection state

Allowed `connection_state` values are:

```text
connected
stale
disconnected
```

For standalone runtimes, the default heartbeat target is 15 seconds, the default stale threshold is 45 seconds, and the default disconnected threshold is 120 seconds. Deployments MAY configure stricter bounded thresholds, but MUST preserve:

```text
heartbeat target < stale threshold < disconnected threshold
```

The Server receipt time is authoritative for freshness. An adapter-reported clock MUST NOT make a stale report appear current.

A hosted Worktree runtime uses the same vocabulary, but Server may derive it directly from the joined host task rather than network heartbeats.

### 5.5 Safe error categories

Effective status MAY expose only a bounded category and stable safe code. The V1 category vocabulary is:

```text
auth
invalid_config
control_rejected
dependency_unavailable
provider_unavailable
rate_limited
storage_unavailable
cancelled
unsafe_delete_blocked
internal
```

Absence of an error is represented by null, not by a fabricated success message.

### 5.6 Worktree control

Worktree is hosted by Server. Server owns application of desired control and joined task lifecycle; the Worktree component continues to own scheduler, filesystem, cancellation, and mode semantics.

Valid desired modes are:

- `disabled` with `desired_enabled = false`;
- `read_only`;
- `import_only`;
- `export_only`;
- `bidirectional`;
- `dry_run`.

`read_only` and `export_only` retain their accepted existing meanings even when their current capabilities are equivalent. They MUST NOT be silently rewritten to a different wire value.

`dry_run` is planning-only. It MUST NOT start automatic cycles or mutate Core, cursors, Worktree durable state, materialized files, trash, or echo state. A dry-run cycle is started only by an explicit operational job.

Worktree transition rules:

- Any mode may transition to `disabled`; Server drains/cancels the current cycle first.
- `disabled` may transition to any valid non-disabled mode after durable identity binding succeeds.
- A transition between two enabled modes applies at a cycle boundary. A cycle already admitted under the old generation may complete only when doing so is safe; otherwise it is cancelled.
- Effective mode changes only after the host has applied the generation.
- A failed bind, host start, or cycle leaves desired state unchanged but reports a truthful failed effective lifecycle.

### 5.7 GDrive control

GDrive remains a separately running adapter process. It polls a private authenticated control surface and reports effective state to Server.

Valid desired modes are:

- `disabled` with `desired_enabled = false`;
- `import_only`;
- `export_only`;
- `bidirectional`;
- `dry_run`.

New GDrive control requests using `read_only` MUST be rejected as invalid. `export_only` is the canonical mode for Core-to-GDrive-only behavior.

`dry_run` is planning-only and explicit. It may read provider metadata and Core metadata needed to produce a safe count/manifest, but it MUST NOT mutate Core, Drive, mappings, cursors, echo state, or delete state.

The standalone process MUST continue polling and heartbeating while desired-disabled unless its process is intentionally stopped. Desired-disabled means the sync worker is stopped, not that control communication is stopped.

On startup or reconnect, GDrive MUST:

1. fetch the complete current control snapshot;
2. compare its last applied adapter control and maintenance generations;
3. fail closed for mutation until the current adapter control and maintenance generations are applied;
4. report effective mode, lifecycle, and both last-applied generations;
5. resume cycles only when maintenance hold is false and the desired mode permits the direction.

A disconnected or stale GDrive adapter MUST NOT be represented as successfully disabled, ready, or synchronized unless a separate fence proves that fact.

### 5.8 Obsidian ownership boundary

Obsidian plugin mode remains device-owned. Server MUST NOT present a central desired mode as if it had changed plugin-local settings or stopped the device runtime.

Server may record last-observed device facts such as adapter identity, credential status, last seen time, and last reported safe mode, but those facts are observations, not centrally applied effective state.

During any non-`normal` maintenance state, Server gates Obsidian mutations exactly like other public writes. The plugin SHOULD queue/retry safe local facts after normal service resumes and MUST preserve base/null-base semantics. Local plugin changes during maintenance are not lost merely because Server rejected their immediate submission.

### 5.9 Retry and eventual consistency

Desired control is durable and authoritative; effective state is eventually consistent.

- A runtime retries fetching/applying the current adapter control and maintenance generations with bounded backoff.
- Applying the pair `(adapter_control_generation, maintenance_generation)` is idempotent.
- An older adapter control or maintenance generation received after a newer one is rejected and MUST NOT roll back effective state.
- A runtime restart does not reset either durable last-applied generation.
- Server retains desired state through runtime disconnection.
- A control change is not complete until effective state reports the accepted adapter control and maintenance generations or a safe error/fence is recorded.
- Operator output MUST show desired and effective values side by side when they differ.

## 6. Credential lifecycle

### 6.1 Principal and credential separation

A principal represents an adapter or administrator identity and authorization role. A credential is one revocable verification record for that principal.

Principal enablement and adapter desired enablement are distinct:

- `principal_enabled = false` rejects every credential for the principal;
- adapter `desired_enabled = false` stops sync work but does not revoke authentication;
- credential revoke invalidates only that credential;
- provider OAuth material is not a Haze Sync credential and remains in provider-specific secret storage.

The mutable principal record MUST contain at least `principal_id`, `role`, `principal_enabled`, `principal_version`, `credential_set_generation`, and lifecycle timestamps. `principal_version` increments for any principal field change. `credential_set_generation` increments in the same transaction as every create, rotate, revoke, grace transition, or other mutation that changes which credentials may authenticate for that principal. Principal and credential-set CAS values are never inferred from timestamps.

### 6.2 Credential record

The versioned credential record MUST contain at least:

```text
schema_version
credential_id
credential_version
principal_id
issuance_operation_id | null
credential_kind
role
status
verifier_scheme
verifier_material
created_at
not_before
expires_at | null
grace_expires_at | null
rotated_from_credential_id | null
revoked_at | null
revoked_by | null
last_used_at | null
```

Allowed status values are:

```text
active
grace
revoked
expired
```

`credential_version` starts at `1` and increments on every durable mutation of status, verifier scheme/material, lifecycle boundary, revocation metadata, or other authorization-relevant field. `issuance_operation_id` is required for newly issued V1 credentials and may be null only for migrated legacy credentials.

Public/operator DTOs MUST omit `verifier_material`, salts, parameter strings that reveal verifier material, and any secret-management metadata. Safe output may include credential ID, principal ID, role, credential version, status, and lifecycle timestamps.

### 6.3 Token format and one-time plaintext delivery

New V1 bearer credentials MUST use an opaque format that permits lookup by a public credential ID, equivalent to:

```text
hs1.<credential_id>.<secret>
```

The secret MUST contain at least 256 bits generated by a cryptographically secure random source and encoded without embedded whitespace.

The complete plaintext credential:

- is included only in the initial create or rotate response generated by the committing request, whether or not the client successfully receives that response;
- MUST NOT be persisted, logged, traced, audited, placed in an idempotency response snapshot, or returned by later lookup/replay;
- MUST NOT be included in URLs, job summaries, artifacts, or error details;
- exists only in bounded process memory between generation and response disposal;
- cannot be recovered if the one-time response is lost; the operator must rotate or create another credential.

Create and rotate MUST require an idempotency key. The durable idempotency result stores `issuance_operation_id`, `credential_id`, safe lifecycle metadata, and terminal outcome, but never plaintext or reversible secret material. An exact replay after commit returns that same safe result with `plaintext_available = false` and safe code `credential_secret_not_replayable`; it MUST NOT create another credential, repeat rotation, or return a newly generated replacement secret. Recovery from a lost response is a new explicitly authorized rotate or revoke request with a new idempotency key and current CAS values.

### 6.4 Verification material at rest

New credentials MUST store only non-reversible verification material.

The required V1 verifier scheme is `argon2id_v1` with:

- a unique random salt of at least 16 bytes;
- memory cost of at least 64 MiB;
- at least 3 iterations;
- parallelism of at least 1;
- a standard encoded verifier representation;
- constant-time result comparison.

Deployments MAY increase these parameters. They MUST NOT reduce them without a versioned security review.

For `hs1` credentials, lookup uses `credential_id` to select one record and then verifies the secret. The secret or its digest MUST NOT be used as a public lookup key. The private legacy compatibility lookup is defined separately in section 6.9.

### 6.5 Create

Credential creation MUST:

1. authenticate and authorize the operator;
2. require `expected_principal_version`, `expected_credential_set_generation`, and an idempotency key;
3. validate that the target principal and role exist;
4. reject creation if the principal already has an active credential, directing the caller to rotate;
5. generate the credential and verifier in memory;
6. set `credential_version = 1` and V1 `not_before` to the Server commit instant; the V1 admin surface MUST NOT accept caller-selected future activation;
7. persist the credential, increment `credential_set_generation`, finalize the safe idempotency result, and append the audit event in one transaction;
8. return plaintext exactly once after durable commit and discard it after the response attempt.

If the response cannot be delivered after commit, the credential remains valid but unrecoverable. Exact replay returns only the committed credential identity and safe metadata. The caller must use an explicit rotate/revoke action; Server MUST NOT replay plaintext or silently issue another credential.

### 6.6 Rotate and grace

Rotation requires the current `expected_principal_version`, `expected_credential_set_generation`, expected version of every credential whose status will change, and an idempotency key. Rotation is atomic:

1. generate and persist one new `active` credential with `credential_version = 1` and `not_before` equal to the commit instant;
2. move the previous active credential to `grace` and increment its `credential_version`;
3. assign a bounded `grace_expires_at`;
4. revoke any older grace credential and increment its `credential_version` in the same transaction;
5. increment the principal `credential_set_generation` exactly once;
6. finalize the safe idempotency result and record rotation and any forced grace revocation in audit;
7. return only the new plaintext credential on the first successful response attempt.

The default grace period is 15 minutes. The maximum accepted grace period is 24 hours. A requested zero grace period revokes the previous credential immediately.

A principal MUST have at most one `active` and one `grace` credential after a transaction commits.

A grace credential has the same role as before but is accepted only until `grace_expires_at`, `expires_at`, explicit revoke, or principal disable, whichever occurs first.

### 6.7 Revoke and expiry

Revocation is explicit, durable, and immediate for any request admitted after the revoke transaction commits.

- Revoking requires `expected_credential_version` and current principal/credential-set CAS values when it can change set invariants. An exact idempotency replay is a no-op; an already revoked or expired credential with matching current CAS is also an idempotent no-op.
- Revoking a credential does not delete its record or audit history.
- Revoking the active credential does not automatically promote a grace credential.
- `not_before`, `expires_at`, and `grace_expires_at` are evaluated from Server time before authorization. A credential is unusable while `now < not_before` and expired when an applicable expiry boundary has passed, even if an asynchronous status-maintenance task has not yet updated the row.
- Positive authentication caches MUST validate a revocation/principal generation or otherwise be invalidated so committed revocation is not bypassed by stale cache state.

Credential revoke is permitted in every maintenance state when control storage is available.

### 6.8 Authentication lookup behavior

Authentication MUST evaluate, in order:

1. token shape and version, selecting exactly one of the `hs1` or legacy paths;
2. credential record existence through the selected private lookup path;
3. principal existence and `principal_enabled`;
4. Server time at or after `not_before`;
5. credential status and effective `expires_at`/`grace_expires_at` boundary;
6. verifier result;
7. role authorization for the requested operation.

Malformed, unknown, non-matching, not-yet-valid, revoked, expired, grace-expired, or disabled-principal credentials MUST produce the same generic public authentication failure. Public output MUST NOT reveal which check failed or whether legacy lookup was attempted.

`last_used_at` MAY be updated asynchronously, but failure to update it MUST NOT change an otherwise valid authentication decision.

### 6.9 Legacy single-token compatibility

The current `sync_adapters.enabled` and `sync_adapters.token_hash` model is migration input, not the final credential model.

A downstream migration MUST:

- map `sync_adapters.enabled` to principal authorization enablement, not adapter desired runtime enablement;
- create a principal for each accepted adapter identity/role with initialized principal and credential-set CAS values;
- create one synthetic `credential_id` and credential record for the existing SHA-256 value using verifier scheme `legacy_sha256_v0`;
- preserve the legacy SHA-256 value only as private verification/lookup material;
- allow legacy credentials only for compatibility lookup;
- prohibit creation or rotation into `legacy_sha256_v0`;
- require the next rotation to issue an `argon2id_v1` credential;
- never expose the legacy hash in public/operator output.

A token without the `hs1.` prefix enters the legacy path only while legacy compatibility is enabled. A token with the `hs1.` prefix that is malformed MUST fail generically and MUST NOT fall back to legacy lookup. Server computes the accepted legacy SHA-256 form in bounded memory and performs one private exact lookup against `legacy_sha256_v0` records. The raw token and computed digest MUST NOT be logged, audited, cached as public metadata, or returned. Zero matches, multiple matches, disabled principal, lifecycle rejection, or verifier mismatch fail generically. The legacy path MUST NOT scan or attempt Argon2 verification across unrelated credential rows.

After rotation, the legacy credential follows the ordinary bounded grace or immediate-revoke rule and MUST NOT be promoted again. Exact replay of the rotation returns the same new credential identity without plaintext. Legacy compatibility MUST NOT weaken generic failure behavior, CAS, idempotency, audit, or revocation-cache requirements.

## 7. Operational jobs

### 7.1 Job model

Every operational job MUST use schema `haze-sync.operational-job.v1` and contain at least:

```text
schema_version
operation_id
job_version
kind
maintenance_required
destructive
requester_principal_id
requester_credential_id | null
idempotency_scope
idempotency_key_digest
request_fingerprint
state
dry_run
confirmation_required
confirmation_digest | null
confirmation_expires_at | null
expected_maintenance_generation | null
expected_adapter_control_generations[]
created_at
updated_at
started_at | null
completed_at | null
cancel_requested_at | null
safe_summary
safe_error_category | null
artifact_manifest_id | null
artifact_manifest_digest | null
checkpoint | null
execution_scope_digest
executor_id | null
executor_fence
lease_token_digest | null
lease_heartbeat_at | null
lease_expires_at | null
destructive_execution_slot_id | null
```

Each `expected_adapter_control_generations` entry MUST contain exactly `adapter_id` and `adapter_control_generation`; duplicate adapter IDs are invalid. `job_version` starts at `1`. `executor_fence` starts at `0` before the first lease and is incremented only by a successful lease acquisition or takeover.

Raw idempotency keys, confirmation values, credentials, file content, provider payloads, or arbitrary command output MUST NOT be stored in the job record.

### 7.2 Kind registry

The V1 operational kind registry reserves:

```text
adapter_dry_run
adapter_reconcile
backup
restore
repair
retention_cleanup
deployment_rollout
deployment_rollback
```

A component may support only a subset. Unsupported kinds MUST be rejected explicitly; they MUST NOT be approximated with direct shell, SQL, provider, or filesystem actions.

`adapter_dry_run` is non-mutating apart from job/audit records. `restore`, `repair`, `retention_cleanup`, and `deployment_rollback` are maintenance-required and confirmation-required. `backup`, `adapter_reconcile`, and `deployment_rollout` MUST declare their mutation and maintenance requirements in their versioned executor contract before implementation.

### 7.3 States and legal transitions

Allowed job states are:

```text
planned
awaiting_confirmation
running
succeeded
failed
cancelled
```

Legal transitions are:

| Current | Next | Requirement |
|---|---|---|
| create | `planned` | Request validated and idempotency record reserved. |
| `planned` | `awaiting_confirmation` | Plan and immutable confirmation inputs persisted. |
| `planned` | `running` | Confirmation not required; all expected generations, job version, and execution slot revalidated. |
| `planned` | `cancelled` | No external side effect has started. |
| `planned` | `failed` | Planning, generation, manifest, or pre-execution validation failed while the transition owner held current job CAS and before any external side effect. |
| `awaiting_confirmation` | `running` | One-time confirmation valid and current generations revalidated. |
| `awaiting_confirmation` | `cancelled` | Operator cancels or confirmation expires. |
| `awaiting_confirmation` | `failed` | Stored plan/manifest became invalid, required generation changed, or restart recovery proves the job cannot safely start. |
| `running` | `succeeded` | All required effects and durable checkpoints confirmed. |
| `running` | `failed` | Safe terminal failure recorded; no success claim. |
| `running` | `cancelled` | Cancellation completed and no further effects can start. |

Every transition MUST compare and increment `job_version`; executor-owned transitions also require the current executor fence. Terminal states are immutable. A retry creates a new operation with a new idempotency key and may reference the prior operation ID.

### 7.4 Dry run, artifact manifest, and confirmation

`dry_run = true` means the executor performs no product, provider, cursor, mapping, deployment, or filesystem mutation. Job/audit persistence is allowed.

A destructive job MUST first produce an immutable artifact/plan manifest containing safe identities, counts, expected generations, scope, and a SHA-256 digest. The manifest MUST NOT contain secrets or raw file content unless stored in an explicitly authorized protected backup artifact outside public/operator output.

Confirmation MUST be bound to:

```text
operation_id
kind
request_fingerprint
expected_maintenance_generation
expected_adapter_control_generations
job_version
artifact_manifest_digest
confirmation_expiry
```

The confirmation value is returned once, stored only as a non-reversible digest, usable once, and valid for at most 15 minutes. A stale generation, changed manifest, changed request, expired confirmation, or reused confirmation is rejected without starting work.

### 7.5 Execution and action boundary

Operational jobs MUST be executed by Server or a specifically authorized deployment/recovery component through a named, versioned, bounded action contract.

An executor MUST NOT accept arbitrary shell text, raw SQL, provider credentials, or unbounded filesystem paths from CLI/API input. Each executable action MUST declare:

- exact kind and version;
- bounded target scope;
- required maintenance state;
- dry-run and confirmation behavior;
- checkpoint and cancellation semantics;
- artifact outputs;
- safe error categories.

The CLI and connectors invoke those public/named actions. They do not become the execution authority.

#### 7.5.1 Executor lease and fencing

A job enters `running` only by atomically:

1. comparing `expected_job_version` and all expected control generations;
2. acquiring the applicable execution slot;
3. assigning `executor_id`;
4. incrementing `executor_fence` to a value never previously used by that job;
5. storing only a non-reversible `lease_token_digest` plus bounded heartbeat/expiry timestamps;
6. incrementing `job_version` and appending the start audit event.

The executor holds the raw lease token only in bounded runtime memory or accepted secret transport. Every heartbeat, checkpoint, cancellation acknowledgement, lease release, and terminal transition MUST compare `(operation_id, expected_job_version, executor_fence, lease proof)`. A stale executor receives `stale_executor_fence` and MUST stop before starting another side effect.

Before each bounded external step, the executor MUST re-read or strongly validate its current fence and lease. Provider/deployment operations SHOULD use an idempotency or fencing value derived from `(operation_id, step_id, executor_fence)` when the external system supports it. If an external action cannot be fenced or queried, lease expiry MUST NOT authorize takeover while its outcome is uncertain; the job remains running-recoverable or failed-uncertain until explicit reconciliation.

Lease renewal extends time only; it does not change `executor_fence`. Lease takeover after expiry MUST allocate a larger fence and is allowed only when the durable checkpoint proves no prior external step is in flight, or when the next executor can query/deduplicate the exact step outcome. A former executor can never write a checkpoint or terminal result after takeover.

#### 7.5.2 Destructive and overlapping-job concurrency

Every executor contract MUST classify the job as `destructive`, `maintenance_required`, and provide a canonical `execution_scope_digest`.

The durable global destructive execution slot MUST contain at least `slot_id`, `slot_version`, `operation_id | null`, `maintenance_generation | null`, `executor_fence | null`, `blocked_uncertain`, and timestamps. Acquisition, release, or reconciliation of the slot requires `expected_slot_version` and increments `slot_version`.

- At most one `running` job with `destructive = true` or `maintenance_required = true` may exist globally.
- Transition to `running` for such a job MUST acquire one durable global destructive execution slot in the same transaction as the job lease and bind it to `operation_id`, `expected_maintenance_generation`, and `executor_fence`.
- A competing start is rejected with `operation_in_progress`; it MUST NOT wait invisibly or start partial work.
- The slot is released only after a terminal outcome proves no further side effect can start. A failed job with an uncertain external effect retains or blocks the slot until a separate reconciler records a safe resolution.
- Non-destructive jobs MAY run concurrently only when their versioned contracts prove disjoint execution scopes. Jobs with the same scope digest, or scopes that cannot be proven disjoint, MUST be serialized.

Planning and awaiting-confirmation records may coexist, but their generations and manifests are revalidated when the execution slot is acquired; they do not reserve execution authority merely by existing.

### 7.6 Cancellation

A cancel request on `running` sets `cancel_requested_at`; state remains `running` until the executor proves no further side effects can start.

- Cancellation is cooperative and checked between bounded steps.
- An atomic side effect already in progress may finish.
- The executor MUST persist the resulting checkpoint before terminal cancellation.
- If an external effect is uncertain, the job MUST NOT become `cancelled` or `succeeded`; it remains recoverable `running` or becomes `failed` with an uncertainty category and keeps maintenance closed.

### 7.7 Restart recovery

On restart:

- terminal jobs remain unchanged;
- `planned` jobs remain planned only when their persisted plan inputs are valid; otherwise they transition to `failed` through CAS before any side effect;
- `awaiting_confirmation` jobs remain pending while their plan, manifest, and generations remain valid; expiry becomes `cancelled`, while invalid or stale persisted inputs become `failed`;
- `running` jobs may resume or transfer execution only from a durable checkpoint with idempotent, externally queryable, or externally fenced side effects and a newly acquired executor fence;
- a non-resumable or ambiguous running job becomes `failed` with safe category `restart_recovery_required` and blocks resume from maintenance until reconciled;
- a lease expiry alone does not prove an external action stopped and does not release a destructive execution slot.

A job MUST NOT be marked succeeded solely because the process restarted after issuing an external request.

### 7.8 Control-plane preservation during restore

Ordinary product-data restore MUST NOT overwrite:

- current maintenance state and generation;
- adapter desired/effective control records;
- principals and credentials;
- operational jobs and idempotency records;
- audit events.

Restoring those records requires a separate, explicitly confirmed control-plane recovery procedure whose own state is stored outside the restore set. This prevents a restore from reopening admission, reviving revoked credentials, or forgetting an in-flight destructive operation.

## 8. Operational idempotency

### 8.1 Scope and storage

Operational-job idempotency is separate from existing adapter write idempotency.

The unique scope is:

```text
(requester_principal_id, operation_kind, idempotency_key_digest)
```

The digest MUST be HMAC-SHA-256 or a stronger keyed digest using a server-managed operational-idempotency pepper. The raw key MUST NOT be persisted or logged.

The request fingerprint MUST cover canonical safe request metadata including kind, target scope, dry-run flag, confirmation requirement, and expected generations. It MUST exclude credentials, raw idempotency keys, confirmation values, provider payloads, and file bytes.

### 8.2 Replay and conflict behavior

- First use reserves one operation identity transactionally.
- Same scope/key and same fingerprint returns the existing operation and current state.
- Same scope/key with a different fingerprint returns `idempotency_conflict`.
- Concurrent duplicates create at most one job.
- Replaying a terminal job returns its terminal result; it never reruns the job.
- Replaying a non-terminal job returns the existing operation; it never starts a parallel executor.

### 8.3 Generation checks

A new job request with a stale maintenance generation, any stale adapter control generation, or stale requester/CAS record version is rejected before job creation.

An exact replay of an existing job remains readable even if any current generation changed, but it is historical evidence only and MUST NOT restart execution.

Immediately before `planned` or `awaiting_confirmation` enters `running`, Server MUST compare `expected_job_version`, the exact `expected_maintenance_generation`, every `(adapter_id, adapter_control_generation)` entry, and the applicable execution slot. A generation mismatch moves the job to `failed` with safe code `stale_control_generation`; a job-version mismatch rejects the caller with `stale_record_version`; neither path performs an external effect. Lease acquisition then creates a new `executor_fence` as defined in section 7.5.1.

### 8.4 Retention

Idempotency records MUST remain available while their job is non-terminal and for at least as long as the job result and its audit references are retained. Eviction MUST NOT permit a previously completed destructive request to be replayed as new within the supported audit/operation retention window.

## 9. Audit contract

### 9.1 Event model

Operational audit events use schema `haze-sync.audit-event.v1` and are append-only.

Each event MUST contain at least:

```text
audit_id
schema_version
occurred_at
event_type
outcome
actor_principal_id | null
actor_credential_id | null
actor_role | null
actor_origin
request_id | null
correlation_id | null
target_type
target_id | null
operation_id | null
maintenance_generation | null
adapter_control_generation | null
principal_version | null
credential_set_generation | null
credential_version | null
job_version | null
executor_fence | null
previous_state | null
next_state | null
safe_error_category | null
artifact_manifest_id | null
safe_metadata
```

Corrections are represented by a new event referencing the prior audit ID. Existing events MUST NOT be rewritten to conceal an earlier action or error.

### 9.2 Required event families

Maintenance and control MUST emit events for:

- transition requested, accepted, rejected, completed, timed out, and resume failed;
- admission fence closed and reopened;
- quiescence evidence created or invalidated;
- external fence established or removed.

Adapter control MUST emit events for:

- desired state changed or rejected;
- generation applied;
- safe effective failure;
- transition to stale or disconnected;
- explicit external fence.

Credential lifecycle MUST emit events for:

- credential created;
- rotation and grace start;
- grace expiry;
- explicit revoke;
- principal enable/disable;
- rejected administrative credential action.

Operational jobs MUST emit events for:

- job created;
- plan/artifact finalized;
- confirmation issued, accepted, expired, or rejected;
- start and cancellation request;
- succeeded, failed, or cancelled;
- restart recovery and uncertain external effect.

### 9.3 Atomicity and truthful outcomes

A durable control, credential, or job state transition and its audit event MUST be committed in the same transaction when they share one storage authority.

Before an external side effect, the executor MUST persist an intent/start transition and audit event. If that persistence fails, the external side effect MUST NOT start.

After an external side effect, the executor MUST persist the resulting checkpoint/state and audit outcome before reporting success. If result persistence is unavailable, the job remains non-terminal or failed-uncertain; it MUST NOT be reported succeeded.

### 9.4 Allowed metadata

Safe metadata may include:

- opaque internal identifiers;
- adapter kind and safe mode values;
- counts and booleans;
- control and schema versions;
- artifact IDs and SHA-256 digests;
- bounded safe error codes;
- vault-relative paths only when an existing sync audit contract requires path context.

### 9.5 Prohibited audit content

Audit events MUST NOT contain:

- plaintext Haze Sync or provider credentials;
- token hashes, verifier material, salts, peppers, or confirmation digests;
- raw idempotency keys;
- OAuth material or secret-file paths;
- raw provider payloads, raw external cursors, or provider response bodies;
- raw file content or unredacted request bodies;
- database URLs, SQL, stack traces, or driver errors;
- local absolute paths;
- arbitrary shell commands or complete command output.

## 10. Downstream ownership and required follow-up surfaces

### 10.1 Storage

Storage owns future migrations and passive repositories for:

- maintenance state and generation;
- quiescence evidence and adapter-inventory snapshots;
- adapter desired/effective control records;
- principals, credential-set generations, credential records, and issuance idempotency results;
- operational jobs, executor fences, global/scoped execution slots, leases, checkpoints, confirmations, and idempotency records;
- append-only operational audit events.

Storage MUST provide transaction/CAS primitives for every named version/generation, complete adapter-inventory snapshots, and caller-owned locking. It MUST NOT decide maintenance transitions, adapter modes, credential authorization policy, job confirmation policy, or external side effects.

Storage migration work MUST preserve existing adapter, cursor, idempotency, GDrive, Worktree, and audit data. It MUST implement the legacy credential migration described in this contract.

### 10.2 API

API owns versioned DTOs, validated input shapes, stable safe errors, and passive route-contract helpers for:

- maintenance status and action requests;
- adapter desired status, effective report, and desired/effective views;
- principal and credential lifecycle;
- operational job create/read/confirm/cancel;
- safe audit queries where authorized.

The public error vocabulary MUST include equivalents of:

```text
maintenance_in_progress
control_transition_in_progress
invalid_control_transition
stale_control_generation
stale_record_version
stale_executor_fence
idempotency_conflict
credential_secret_not_replayable
operation_in_progress
confirmation_required
confirmation_expired
operation_not_cancellable
unsupported_operation_kind
```

Authentication failure remains generic and MUST NOT reveal credential status.

API remains passive. It MUST NOT start runtimes, query the database directly, execute jobs, or perform provider/deployment actions.

### 10.3 Server

Server owns:

- loading control state before mutation admission;
- the shared application-service admission fence;
- maintenance transition orchestration and quiescence evidence;
- hosted Worktree control application and joined lifecycle;
- private GDrive control polling/report surfaces;
- authentication lookup and credential policy execution;
- operational-job orchestration, confirmations, leases, and checkpoints;
- transactionally consistent operational audit production;
- health/readiness mapping.

Server MUST continue delegating sync conflict/delete/revision/idempotency decisions to Core and persistence primitives to Storage.

### 10.4 CLI

CLI may later expose operator commands only through accepted public Server APIs.

It may:

- read desired/effective/maintenance/job status;
- request quiesce, maintenance entry, and resume with `expected_maintenance_generation`;
- create, rotate, and revoke credentials with principal/credential-set/credential CAS values, idempotency keys, and one-time secret handling;
- create dry-run plans, display safe manifests, confirm, poll, or cancel jobs using `job_version` and never treating a lease expiry as completion.

It MUST NOT:

- query or mutate PostgreSQL directly;
- inspect object-store internals;
- call Google Drive or other providers directly;
- accept arbitrary deployment shell or SQL;
- print plaintext credentials after their one-time response;
- claim success without a terminal Server job result.

Existing offline plan commands remain no-write and MUST NOT be relabeled as executed operations.

### 10.5 Worktree

Worktree owns scheduler and filesystem behavior needed to:

- apply Server-provided generations and maintenance hold;
- stop new scheduling;
- cooperatively cancel/drain one in-flight cycle;
- preserve cursor and path-state safety;
- report safe lifecycle, counts, generation, heartbeat/last-success facts;
- run explicit non-mutating dry-run jobs.

Worktree MUST NOT write control tables directly or decide that global quiescence is complete.

### 10.6 GDrive

GDrive owns provider/runtime behavior needed to:

- poll private desired control while enabled or disabled;
- apply only the current adapter control and maintenance generations;
- stop new provider/Core mutations under hold;
- drain/cancel in-flight work safely;
- report effective mode, lifecycle, heartbeat, success, generation, and safe errors;
- preserve provider cursor/mapping safety;
- run explicit non-mutating dry-run jobs.

GDrive MUST NOT use direct database access by default, expose OAuth material, or claim quiescence merely because its process is silent.

### 10.7 Obsidian plugin

Obsidian retains device-owned settings and mode. It consumes public maintenance errors, queues or retries local facts safely, and preserves normal base/conflict semantics after resume.

The plugin MUST NOT represent Server maintenance as proof that its local vault stopped changing.

### 10.8 Deployment and recovery components

Deployment/recovery owns the implementation of explicitly authorized named actions such as process fencing, backup artifact handling, restore, rollout, and rollback.

It MUST:

- use versioned bounded actions rather than arbitrary shell supplied by CLI/API;
- keep provider and deployment secrets in accepted secret storage;
- return safe structured evidence and artifact manifest identities;
- support cancellation/checkpoint semantics declared by each action;
- respect maintenance generations and quiescence evidence;
- never revive revoked credentials or overwrite current control-plane records during ordinary product restore.

GitHub Actions or Connector-backed execution may host these named actions, but the job record and Server control state remain authoritative for operator-visible status. A workflow dispatch alone is not success evidence.

## 11. Compatibility evidence on the authorized baseline

The following exact baseline surfaces constrain downstream implementation:

| Surface | Preserved contract |
|---|---|
| `crates/haze-sync-common/src/adapter.rs` | Stable adapter role and mode wire values. |
| `crates/haze-sync-core/docs/component-contract.md` | Core-only sync policy, base revision safety, tombstone/conflict semantics, adapter-scoped write idempotency. |
| `crates/haze-sync-core/src/idempotency/mod.rs` | Same-request replay and different-request conflict; safe stored response boundary. |
| `crates/haze-sync-api/docs/component-contract.md` | Passive DTO/error/auth boundary and secret-safe output. |
| `crates/haze-sync-api/src/auth/mod.rs` | Current redacting bearer/hash primitives are compatibility inputs, not the final credential lifecycle. |
| `crates/haze-sync-server/docs/component-contract.md` | Server application-service/runtime authority and hosted Worktree lifecycle. |
| `crates/haze-sync-worktree/docs/component-contract.md` | Worktree modes, no-overlap scheduling, cooperative cancellation, and non-mutating dry run. |
| `crates/haze-gdrive-adapter/docs/component-contract.md` | Separate provider runtime, mode enforcement, safe retries/errors, and no direct DB by default. |
| `apps/haze-obsidian-plugin/docs/component-contract.md` | Device-owned plugin runtime and settings; public Server API only. |
| `migrations/0001_sync_adapters.sql` | Legacy single token hash and enabled flag require explicit migration. |
| `migrations/0006_cursors_idempotency.sql` | Existing adapter cursor/write-idempotency records remain separate from operational jobs. |
| `migrations/0009_audit_events.sql` | Existing append-oriented safe audit metadata is extended, not made secret-bearing. |
| `docs/process/stage9-operational-cli-review.md` | Existing CLI plans are no-write and operational mutation remains blocked until contracts and public APIs exist. |

No requirement in this document authorizes a current component to bypass those boundaries before its owner phase implements and verifies the new contract.

## 12. Required downstream verification

A downstream implementation is not accepted until evidence proves all applicable requirements below.

### Maintenance

- legal, invalid, stale, duplicate, and concurrent transition behavior;
- admission fence applied in HTTP and non-HTTP application services;
- drain/cancel and timeout behavior without cursor/checkpoint overclaim;
- restart in every persisted state;
- readiness false outside `normal` and health remaining truthful;
- immutable, adapter-inventory-complete quiescence evidence covering every Worktree/GDrive `adapter_id`, plus the Obsidian mutation-gate boundary;
- destructive jobs blocked without current evidence and maintenance generation.

### Adapter control

- desired/effective values remain separately visible;
- monotonic, namespace-explicit adapter and maintenance generations under concurrency and restart;
- Worktree hosted application and GDrive polling/reporting;
- disabled runtime remains distinct from credential revoke;
- stale/disconnected representation and no fabricated readiness;
- valid/invalid mode matrix and non-mutating `dry_run`;
- resume blocked until every desired-enabled controlled adapter identity applies its current adapter control and maintenance generations.

### Credentials

- one-time plaintext delivery, idempotent safe replay without secret redelivery, and explicit lost-response recovery;
- Argon2id verification and secret-safe formatting/output;
- create, rotate, bounded grace, existing-grace replacement, revoke, `not_before`, and expiry;
- maximum one active plus one grace credential;
- immediate committed revocation despite caches;
- generic lookup failure behavior across exact `hs1` ID lookup and private legacy raw-token compatibility lookup;
- legacy SHA-256 compatibility through explicit private lookup and only until rotation/bounded grace;
- audit with no secret or verifier material.

### Operational jobs and audit

- exact state-transition matrix including pre-running failure transitions, CAS increments, and terminal immutability;
- dry-run no-write proof;
- bound one-time confirmation and expiry;
- idempotent duplicate/concurrent behavior and fingerprint conflict;
- stale generation rejection at create and start;
- restart recovery from checkpoints, executor fencing/takeover, destructive-slot retention, and uncertain-outcome handling;
- named bounded action execution, global destructive concurrency control, and no arbitrary shell/SQL/provider bypass;
- append-only, transactionally consistent, secret-safe audit events;
- ordinary restore preserving the live operational control plane.

### Scope

Verification MUST also prove that implementation changes remain in the owning components and that no component silently takes Core, API, Storage, Server, adapter, CLI, or deployment authority from another component.
