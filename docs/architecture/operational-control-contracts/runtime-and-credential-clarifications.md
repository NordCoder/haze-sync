# Haze Sync V1 Runtime and Credential Clarifications

Status: normative V1 addendum  
Parent contract: `docs/architecture/operational-control-contracts.md`  
Parent content baseline: Git blob `ebb0ec000c0b350a3bef17191fece9dabf4612b8`

## 1. Authority and precedence

This document is an inseparable normative child of the parent operational-control contract. It closes three implementability gaps discovered by independent QA:

1. exclusive ownership and ordered effective reporting for standalone GDrive runtimes;
2. one authoritative source for credential authorization roles;
3. complete idempotency for credential create and rotate.

Where this addendum conflicts with the parent contract, this addendum controls. All parent requirements not changed here remain normative.

The following additional safe error codes are reserved:

```text
stale_runtime_epoch
stale_runtime_report
runtime_report_conflict
runtime_lease_conflict
idempotency_in_progress
```

## 2. Standalone runtime ownership and report ordering

### 2.1 Required namespaces

The common control primitives additionally include:

```text
runtime_lease_version       mutable CAS token for one standalone runtime lease
standalone_runtime_epoch    exclusive ownership/fencing epoch for one adapter_id
report_sequence             ordered effective-state publication within one epoch
```

`runtime_lease_version` and `standalone_runtime_epoch` are non-negative monotonic integers. They MUST NOT be reused, decremented, or reset after restart, failover, clean release, expiry, or takeover. `report_sequence` starts from zero for each newly allocated epoch and advances only as defined below.

A stale lease version returns `stale_record_version`. A stale or superseded epoch returns `stale_runtime_epoch`.

### 2.2 Durable runtime lease

Every standalone GDrive `adapter_id` MUST have one durable lease record containing at least:

```text
schema_version
adapter_id
runtime_lease_version
standalone_runtime_epoch
runtime_instance_id | null
lease_token_digest | null
lease_heartbeat_at | null
lease_expires_at | null
last_accepted_report_sequence
last_accepted_report_fingerprint | null
open_mutation_permits
uncertain_external_effects
takeover_state = clear | reconciliation_required
updated_at
```

`runtime_instance_id` is opaque and non-secret. A process MUST generate a fresh value on every process start. A raw lease token is returned once, retained only in bounded process memory or accepted secret transport, and stored only as a non-reversible digest. It MUST NOT be shared with another process or appear in logs, traces, audit metadata, errors, or operator output.

Lease acquisition, renewal, release, and takeover MUST serialize on `expected_runtime_lease_version`.

Acquiring an unowned lease, replacing a cleanly released lease, or taking over an expired lease MUST atomically:

1. increment `runtime_lease_version`;
2. allocate a `standalone_runtime_epoch` greater than every prior epoch for that `adapter_id`;
3. bind the requesting `runtime_instance_id` and lease digest;
4. initialize `last_accepted_report_sequence = 0` and the report fingerprint to null;
5. establish bounded heartbeat and expiry timestamps;
6. append a secret-safe audit event.

An acquisition attempt while another unexpired lease is current returns `runtime_lease_conflict` without changing the lease or epoch. Renewal increments `runtime_lease_version` but preserves the epoch and instance identity.

At most one epoch is authoritative for an `adapter_id`. Bearer authentication alone is insufficient for standalone control reports or mutation admission. Every such request MUST carry:

```text
adapter_id
runtime_instance_id
standalone_runtime_epoch
expected_runtime_lease_version
lease_proof
```

Server rejects a missing, expired, released, superseded, or non-matching lease before accepting a report, checkpoint, cursor advance, or mutation admission. A former or parallel runtime MUST stop scheduling new work after renewal or admission failure and cannot advance authoritative state.

### 2.3 Bounded mutation admission

Before each bounded GDrive provider or Core mutation step, the runtime MUST obtain Server admission bound to:

```text
adapter_id
runtime_instance_id
standalone_runtime_epoch
runtime_lease_version
adapter_control_generation
maintenance_generation
step_id
bounded_expiry
```

Server issues admission only when the runtime lease is current, the maintenance fence permits the operation, the desired mode permits its direction, and no unresolved takeover state blocks work. Issuance atomically records one open mutation permit and an in-flight effective state before the runtime may start the provider/Core step.

A permit authorizes only the named bounded step. It MUST NOT authorize arbitrary provider operations, another epoch, another adapter, or work after its bounded expiry. The runtime reports the terminal outcome and durable checkpoint under a later ordered report; Server then closes the permit atomically with that accepted report.

A superseded runtime cannot obtain a new permit. Expiry or epoch takeover does not prove that a previously admitted provider operation stopped. Any permit without a confirmed terminal outcome becomes an uncertain external effect and blocks acknowledged quiescence, clean release, and normal takeover until reconciled or covered by an explicit external fence.

### 2.4 Lease takeover and release

Lease expiry alone is not drain evidence and MUST NOT authorize optimistic takeover.

Takeover allocates a new epoch and fences the old epoch at Server. The new runtime starts with:

```text
takeover_state = reconciliation_required
effective_mode = null
open_mutation_permits = 0
uncertain_external_effects >= 0
```

It has no mutation or quiescence authority until durable checkpoints, open permits, and uncertain external effects from the prior epoch are reconciled, or an explicit external fence for that exact `adapter_id` is established.

A clean release is accepted only after the current epoch's latest report proves `in_flight = false`, all mutation permits are closed, no uncertain external effect remains, and the final checkpoint is durable. Release increments `runtime_lease_version`, clears the runtime identity and lease digest, and preserves the epoch history.

### 2.5 Ordered effective reports

The standalone effective record additionally contains:

```text
runtime_instance_id
standalone_runtime_epoch
report_sequence
runtime_lease_version
runtime_lease_expires_at
```

Within one epoch, reports use positive `report_sequence` values beginning at `1` and increasing by exactly one. A report fingerprint MUST cover all effective fields, both applied control generations, in-flight state, open-permit count, checkpoint summary, and safe error state.

Server accepts a report only when:

1. the lease, runtime identity, and epoch are current;
2. `expected_runtime_lease_version` is current;
3. `report_sequence = last_accepted_report_sequence + 1`;
4. the report is internally consistent with durable permits and checkpoints.

Acceptance MUST atomically:

1. replace the effective record;
2. update the lease record's latest sequence and fingerprint;
3. close any terminally reported permits and persist resulting checkpoints;
4. increment `runtime_lease_version`;
5. return the new lease version.

An exact duplicate of the latest sequence and fingerprint is an idempotent replay even when it carries the pre-acceptance lease version. It returns the current effective state and current lease version without another mutation. The same sequence with a different fingerprint returns `runtime_report_conflict`. A lower or skipped sequence returns `stale_runtime_report` and requires a lease/effective-state refetch.

Server receipt order, not adapter clock time, determines report order. Delayed same-epoch reports cannot overwrite later reports. Reports from an older epoch are rejected regardless of their control generation or timestamp.

A runtime MUST obtain mutation admission, thereby recording in-flight work, before starting a bounded mutation. It MUST publish a later ordered terminal outcome before `in_flight = false` can be accepted. `heartbeat_at` alone cannot change in-flight state or prove drain.

### 2.6 Quiescence evidence

For every standalone GDrive entry, acknowledged quiescence evidence MUST include:

```text
runtime_instance_id
standalone_runtime_epoch
runtime_lease_version
last_accepted_report_sequence
open_mutation_permits = 0
uncertain_external_effects = 0
```

Those values MUST match the current durable lease and latest accepted effective record at evidence capture. The latest report for that epoch must apply the current adapter-control and maintenance generations and prove `in_flight = false`.

A stale, duplicate, non-latest, or prior-epoch report can never satisfy quiescence. Evidence is invalidated by a new epoch, lease takeover, a later accepted report, a newly opened permit, a newly discovered uncertain effect, or any parent-contract invalidation condition.

## 3. Credential role authority

### 3.1 Sole source of authorization

The principal record's current `role` is the sole authorization authority. A credential is only a revocable verifier for its `principal_id`; it MUST NOT store, retain, or independently authorize a role.

The `role` field listed in the parent credential record is withdrawn from the normative V1 model. Existing or migrated credential-role values, if temporarily present during migration, are non-authoritative compatibility data and MUST NOT be consulted for authentication, authorization, cache keys, audit actor-role derivation, or operator decisions.

Public credential output MAY expose `effective_role`, but it MUST be derived at read time from the current principal and clearly represented as principal state rather than credential state.

### 3.2 Role changes

A principal role change requires:

```text
expected_principal_version
expected_credential_set_generation
new_role
idempotency_key
```

The change MUST atomically:

1. validate the new role and principal type;
2. update the principal role;
3. increment `principal_version`;
4. increment `credential_set_generation`;
5. invalidate positive authentication and authorization caches;
6. append an audit event containing only safe old/new role identifiers and resulting versions.

Existing `active` and `grace` credentials remain lifecycle-valid unless explicitly revoked, but every request admitted after commit authorizes only under the new principal role. Grace overlaps secrets, not privileges: no credential retains the old role, no grace credential preserves broader authority, and no credential is promoted or reissued implicitly.

A deployment MAY require reauthentication by explicitly revoking current credentials, but revocation is a separate CAS-protected lifecycle decision. Role change by itself MUST NOT silently revoke, rotate, promote, or create credentials.

### 3.3 Authentication and caches

Authentication MUST load the current principal after credential lookup and evaluate, in order:

1. principal existence and enablement;
2. current `principal_version`, `credential_set_generation`, and role;
3. credential lifecycle boundaries and verifier;
4. requested-operation authorization using only the current principal role.

Authentication and authorization caches MUST be keyed or validated by both `principal_version` and `credential_set_generation`. A committed role change therefore takes effect immediately for active and grace credentials. Generic authentication failure behavior remains unchanged and MUST NOT reveal principal, credential, or role state.

## 4. Credential issuance idempotency

### 4.1 Scope and key storage

Credential create and rotate use a dedicated namespace separate from adapter-write and operational-job idempotency. The unique scope is exactly:

```text
(idempotency_namespace = credential_issuance,
 requester_principal_id,
 idempotency_key_digest)
```

`idempotency_key_digest` MUST use HMAC-SHA-256 or a stronger keyed digest with a Server-managed credential-issuance pepper. The raw key MUST NOT be persisted, logged, traced, audited, or returned. The target principal and action are intentionally not part of the uniqueness scope; reusing one key by the same requester for another target or action is an idempotency conflict.

### 4.2 Canonical fingerprint

The request fingerprint uses versioned deterministic serialization of exactly:

```text
issuance_contract_version
action = create | rotate
target_principal_id
credential_kind
requested_expires_at | null
expected_principal_version
expected_credential_set_generation
affected_credentials[]
requested_grace_seconds | null
```

For create:

- `affected_credentials` is empty;
- `requested_grace_seconds` is null.

For rotate:

- `affected_credentials` contains every credential whose lifecycle may change as `(credential_id, expected_credential_version, current_status)`;
- entries are sorted by `credential_id`;
- the bounded grace request is included.

The principal role is bound by `target_principal_id` plus `expected_principal_version`; it is not copied into the credential or fingerprint as separate authority. Transport request IDs, raw keys, plaintext or verifier material, response-delivery state, implicit clocks, provider payloads, and unordered object representation MUST NOT affect the fingerprint.

### 4.3 Reservation and transaction order

Processing order is normative:

1. authenticate and authorize the requester;
2. compute the uniqueness scope and canonical fingerprint;
3. look up the durable issuance record before evaluating current target CAS values;
4. if the scope exists with the same fingerprint, return its stored safe result and never return plaintext;
5. if the scope exists with a different fingerprint, return `idempotency_conflict` without checking or changing target credentials;
6. if the scope is absent, validate principal, credential-set, and every affected-credential CAS value and lifecycle invariant;
7. in one transaction, win the unique scope reservation, allocate one `issuance_operation_id`, apply every credential/principal-set change, persist the committed safe result, and append audit events;
8. only the request whose transaction committed may include generated plaintext in its initial response attempt.

The unique reservation and credential mutation commit or roll back together. V1 has no durable partially reserved issuance state. Candidate secrets and verifiers MAY be generated before the transaction, but an aborted or losing transaction MUST discard them and MUST NOT return plaintext.

### 4.4 Concurrency, replay, and conflict

Concurrent requests with the same scope and fingerprint serialize behind one winner. At most one create or rotation commits.

After the winner commits, every duplicate returns the same safe result with:

```text
issuance_operation_id
committed outcome
affected credential identities
safe lifecycle metadata
plaintext_available = false
safe_code = credential_secret_not_replayable
```

A duplicate MUST NOT generate a replacement secret, repeat rotation, change grace state, or increment any principal/credential version. If bounded waiting ends before the winning transaction becomes visible, the loser returns `idempotency_in_progress` and may retry the same key; it MUST NOT attempt another issuance.

The same scope with a different fingerprint returns `idempotency_conflict`. A precondition or CAS failure before successful reservation does not consume the key. Once a committed issuance record exists, later role, CAS, lifecycle, or expiry changes do not alter replay; the record is historical evidence and never re-executes.

### 4.5 Retention

A committed issuance record MUST remain while the target principal exists and for at least the full credential and audit retention period after principal deletion.

Detailed safe result fields MAY be compacted only to a non-secret tombstone retaining:

```text
uniqueness scope
request fingerprint
issuance_operation_id
action
target_principal_id
affected credential IDs
committed outcome
plaintext_available = false
```

The uniqueness tombstone MUST NOT be evicted if eviction would permit a previously committed create or rotate to execute as new within any supported credential, principal, audit, or recovery retention window.

## 5. Audit, ownership, and verification amendments

### 5.1 Audit

The operational audit model additionally permits these nullable safe fields:

```text
runtime_lease_version
standalone_runtime_epoch
runtime_report_sequence
issuance_operation_id
```

Adapter audit events MUST cover lease acquisition, renewal, release, expiry, takeover, mutation-permit issue/closure, report acceptance/replay, stale report rejection, report conflict, and takeover reconciliation.

Credential audit events MUST cover principal role changes and issuance-idempotency commit, safe replay, conflict, and in-progress outcomes. Audit MUST retain the parent prohibitions on raw lease tokens, idempotency keys, plaintext credentials, verifier material, and provider payloads.

### 5.2 Component ownership

Storage owns passive repositories and transactional/CAS primitives for runtime leases, epochs, report ordering, mutation permits, uncertain effects, sole principal-role state, and retained issuance-idempotency records or tombstones.

API owns passive validated DTOs and safe errors for runtime lease/report/mutation admission, principal role changes, and credential issuance idempotency. API does not own runtime orchestration or authorization decisions.

Server owns exclusive runtime lease/fencing, permit admission, ordered report acceptance, quiescence validation, current-principal-role authorization, cache invalidation, and issuance-idempotency orchestration.

GDrive owns lease renewal, stop-on-fence behavior, bounded permit use, provider execution, checkpoint safety, and ordered effective reporting. It MUST NOT treat possession of a bearer credential or provider OAuth token as runtime ownership.

CLI may request principal role changes and credential issuance only through public Server APIs with current CAS values and idempotency keys. It MUST NOT retry a lost one-time secret by silently changing the key or issuing a second rotation.

### 5.3 Required downstream verification

Downstream implementation evidence MUST prove:

- two parallel GDrive processes cannot both acquire or retain authority for one `adapter_id`;
- old-epoch reports, checkpoints, cursor advances, and mutation admissions are rejected;
- delayed, duplicated, conflicting, skipped, and reordered same-epoch reports cannot overwrite the latest accepted state;
- takeover with an uncertain prior provider effect cannot claim readiness or quiescence;
- acknowledged quiescence uses only the latest report from the current epoch and requires zero open permits and uncertain effects;
- principal role changes immediately govern every active and grace credential without relying on credential-stored role;
- authorization caches cannot preserve an old role after committed version changes;
- same-key concurrent create/rotate produces at most one committed credential or rotation;
- exact issuance replay never redelivers or regenerates plaintext;
- different-fingerprint key reuse conflicts;
- issuance tombstone retention prevents a committed request from later executing as new;
- no executable, workflow, deployment, migration, manifest, lockfile, README, or shared-index behavior is changed by this documentation amendment.
