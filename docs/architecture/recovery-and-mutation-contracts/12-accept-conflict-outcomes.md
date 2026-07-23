# Recovery and Mutation Contracts — accept_conflict Outcomes and Compatibility

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

### 12.8 Concurrency and replay

Concurrent resolution of the same conflict:

- the path/conflict locks MUST serialize attempts;
- exactly one attempt may commit;
- a later different-key attempt observes resolved state and returns `conflict_already_resolved`;
- the authoritative revision MUST not be duplicated.

Stale expected current revision:

- return `stale_current_revision`;
- include only safe expected/actual revision identifiers when public policy permits;
- leave the conflict open and state unchanged.

Same idempotency key and same fingerprint:

- replay the committed safe response, including the same new revision id and operation sequence;
- perform no object-store or database effect again.

Same idempotency key and different fingerprint:

- return `idempotency_conflict`;
- perform no mutation.

Already-resolved replay with the original key:

- idempotency replay wins and returns the original success.

Already-resolved request with a new key:

- return `conflict_already_resolved`;
- do not infer that the caller requested the same historical action unless durable audit metadata proves it.

### 12.9 Failure behavior

Missing conflict or object:

- return `conflict_not_found` or `object_not_found` as appropriate;
- perform no mutation.

Missing incoming revision/blob:

- return `conflict_content_unavailable`;
- leave the conflict open;
- do not create a new revision or alter the object pointer.

Invalid hash or size:

- return `conflict_content_mismatch`;
- leave state unchanged;
- do not expose bytes.

Object-store read/verification failure:

- return `storage_unavailable` or `conflict_content_unavailable` according to whether the failure is transient or an integrity absence;
- do not begin or commit authoritative database effects.

Database/repository failure before commit:

- roll back the entire transaction;
- leave the conflict open and current revision unchanged;
- return a sanitized retryable/non-retryable category.

Commit outcome ambiguity:

- return `commit_outcome_unknown` with instruction to retry the identical request using the same idempotency key;
- MUST NOT advise a new key;
- the retry MUST resolve through the durable idempotency record if commit succeeded.

### 12.10 Metadata-only action compatibility

The exact current Core dispositions are:

```text
accept_current  -> MetadataOnlyCurrentUnchanged + MarkConflictCopyResolved
accept_conflict -> CreateCurrentRevisionFromConflict + MarkConflictCopyResolved
keep_both       -> MetadataOnlyCurrentUnchanged + KeepMaterializedConflictCopy
mark_resolved   -> MetadataOnlyCurrentUnchanged + NoConflictCopyChange
```

The accepted metadata-only actions retain these semantics:

`accept_current`:

- current revision unchanged;
- no new file revision;
- conflict resolved;
- Core disposition is `MarkConflictCopyResolved`; the conflict copy MUST be marked resolved/superseded according to accepted materialization behavior;
- `conflict_resolved` operation and audit evidence recorded.

`keep_both`:

- current revision unchanged;
- no new file revision;
- materialized conflict copy remains the second copy;
- conflict resolved;
- `conflict_resolved` operation and audit evidence recorded.

`mark_resolved`:

- current revision unchanged;
- no new file revision;
- no required conflict-copy materialization change;
- conflict metadata closed;
- `conflict_resolved` operation and audit evidence recorded.

Implementations MAY strengthen transaction, lock, idempotency, and audit integrity for these actions, but MUST NOT silently convert them into content mutation or revision creation.

### 12.11 Safe result and error categories

The service-layer success result contract is `haze-sync.accept-conflict.result.v1` and contains only:

```text
schema
outcome: accepted | replayed
conflict_id
resolution: accept_conflict
previous_current_revision_id
new_current_revision_id
operation_seq
audit_event_id
replayed
```

Public API mapping MUST provide stable categories equivalent to:

| Category | Meaning | Recommended HTTP class |
|---|---|---|
| `accepted` | transaction committed | 200 |
| `replayed` | same-key committed result replayed | 200 |
| `precondition_required` | required expected values absent | 422 |
| `invalid_request` | malformed identifier/hash/size/action | 400 or 422 |
| `conflict_not_found` | conflict does not exist | 404 |
| `object_not_found` | referenced object is absent | 409 |
| `conflict_already_resolved` | new-key attempt after resolution | 409 |
| `stale_current_revision` | current revision changed | 409 |
| `idempotency_conflict` | same idempotency scope/key, different request fingerprint | 409 |
| `conflict_content_unavailable` | preserved revision/blob absent or unreadable | 409 |
| `conflict_content_mismatch` | hash/size/precondition mismatch | 409 or 422 |
| `maintenance_rejected` | mutation admission is not `normal` | 409 or 503 |
| `storage_unavailable` | transient storage dependency failure | 503 |
| `commit_outcome_unknown` | client must retry same key | 503 |
| `internal_error` | sanitized non-specific failure | 500 |

Errors MUST NOT include file content, credentials, raw idempotency keys, internal SQL, SQLx errors, provider payloads, object-store paths, database URLs, host paths, or stack traces.
