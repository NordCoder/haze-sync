# Recovery and Mutation Contracts — Idempotency, Credentials, and Audit

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

### 3.3 Exact specialization of operational idempotency

Operational recovery idempotency is the Stage 10 operational-idempotency contract, not the existing adapter-write idempotency table and not a third recovery-specific mechanism.

The exact unique scope is:

```text
(requester_principal_id, operation_kind, idempotency_key_digest)
```

`idempotency_key_digest` MUST be HMAC-SHA-256 or a stronger keyed digest using a Server-managed operational-idempotency pepper. The raw key MUST NOT be persisted, logged, audited, copied into helper requests, included in manifests/envelopes, or exposed publicly.

The request fingerprint MUST include every semantic field that can change the result, including kind, target scope, `execution_scope_digest`, backup identity, release set, requested artifact set, expected target identity, dry-run state, confirmation requirement, `expected_maintenance_generation`, every expected adapter-control generation, and any required requester/CAS versions. It MUST exclude credentials, confirmation values, raw file bytes, provider payloads, host paths, database URLs, and the raw idempotency key.

For the same operational scope and key digest:

- the same fingerprint MUST return the existing `operation_id` and its current or terminal state;
- a different fingerprint MUST return `idempotency_conflict` and MUST perform no new work;
- concurrent duplicates MUST reserve at most one operational job and MUST NOT start parallel executors;
- replay of a terminal job MUST return its terminal result and MUST NOT rerun it;
- replay of a non-terminal job MUST return the existing job and MUST NOT acquire another lease/slot;
- an ambiguous client timeout MUST be recovered by retrying the identical request with the same raw key so it resolves to the same digest and operation.

A new job request with stale `maintenance_generation`, any stale `(adapter_id, adapter_control_generation)`, stale requester/CAS record version, or invalid adapter-inventory snapshot MUST be rejected before job creation. An exact replay remains readable after generations change, but it is historical evidence only and MUST NOT restart execution.

Immediately before `planned` or `awaiting_confirmation` enters `running`, Server MUST compare `expected_job_version`, the exact `expected_maintenance_generation`, every expected adapter-control generation, the current adapter-inventory/quiescence identities, and the applicable execution slot. A generation mismatch moves the job to `failed` as `stale_control_generation`; a job-version or slot-version mismatch rejects the stale caller as `stale_record_version`; neither path performs an external effect. Lease acquisition then creates the new `executor_fence` defined above.

`accept_conflict` is not an operational recovery job. It continues to use the accepted authenticated adapter/public-write idempotency model and Core replay decision (`NewRequest`, `ReplaySameRequest`, or `ConflictDifferentRequest`) inside its single mutation transaction. Its public different-fingerprint category is also `idempotency_conflict`, but its storage scope remains the accepted adapter-write scope rather than the operational-job digest scope.

### 3.4 Credentials and helper secrets

Operator authentication and helper infrastructure credentials are separate concerns.

- CLI MUST authenticate to Server through the accepted credential mechanism.
- CLI requests MUST contain deployment-configured identifiers such as `backup_destination_id`, `target_environment_id`, and optional opaque `credential_ref`; they MUST NOT contain raw database URLs, passwords, OAuth tokens, vault secrets, provider credentials, or host filesystem paths.
- Server and audit records MUST persist only safe identifiers and references.
- A deployment-owned helper MAY resolve an approved local credential reference from a secrets manager, restricted file, inherited descriptor, or other deployment-owned secret source.
- Secret values MUST NOT be placed in process arguments, shell command strings, environment dumps, helper responses, manifests, logs, or public errors.
- Plaintext secret material MUST NOT cross back to Server or CLI.

### 3.5 Exact specialization of audit

Recovery job events use the Stage 10 append-only schema `haze-sync.audit-event.v1`. Recovery-specific evidence MAY add bounded safe metadata, but MUST NOT create a parallel audit event model or rename the canonical identity/version/fence fields.

Every operational-job transition, job CAS rejection, lease/fence/slot acquisition or release, destructive confirmation, recovery-envelope authority handoff, and uncertain-outcome reconciliation MUST append an audit event. When state and audit share one storage authority, they MUST commit atomically. `accept_conflict` MUST append its mutation audit event in the same database transaction as the authoritative revision change.

Recovery audit events use the canonical fields, including:

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

Recovery `safe_metadata` MAY contain safe values such as backup ID, target environment ID, release-set ID, adapter-inventory generation, quiescence-evidence ID, execution-slot ID/version, recovery-envelope ID/version, conflict/object/revision IDs, manifest digest, bounded counts, result category, and timestamps.

Audit metadata MUST NOT contain file content, conflict bytes, credentials, credential verifier material, token hashes, lease tokens/digests, confirmation digests, raw idempotency keys, raw database URLs, provider payloads, internal SQL, local absolute paths, arbitrary shell text, stack traces, or vault content.
