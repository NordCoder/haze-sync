# Recovery and Mutation Contracts — accept_conflict Transaction

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

## 12. Transactional `accept_conflict`

### 12.1 Purpose and admission

`accept_conflict` promotes the preserved incoming conflict content to one new authoritative revision at the original object path.

It MUST:

- be admitted only in maintenance state `normal`;
- require an authenticated principal authorized to resolve conflicts;
- use a durable idempotency key;
- use explicit optimistic preconditions;
- invoke the current Core `plan_conflict_resolution` policy with `AcceptConflict` after locks are acquired and persist effects that match its returned plan;
- preserve the previous authoritative revision and all older revision history;
- append operation and audit evidence atomically with the resolution;
- leave the conflict open when any required step fails before commit.

It MUST NOT:

- mutate provider state directly;
- replace bytes in place under an existing revision;
- delete the previous current revision or its blob;
- trust client-supplied bytes without hash/size verification;
- mark the conflict resolved before the new authoritative revision and object pointer are durable;
- expose conflict content, SQL, object-store paths, or host paths in results/errors.

### 12.2 Versioned request

The service/API request contract is `haze-sync.accept-conflict.request.v1`.

Required semantic fields are:

```text
schema
conflict_id
resolution: accept_conflict
expected_current_revision_id
expected_incoming_revision_id
expected_content_sha256
expected_size_bytes
```

The protected transport MUST also provide the authenticated principal and idempotency key.

The expected incoming revision/hash/size bind the operator action to the exact preserved content that was reviewed. A request that omits these fields MUST be rejected as `precondition_required` and MUST not mutate state.

The current conflict-resolve route MAY be extended conditionally for `accept_conflict` or a dedicated versioned operation route MAY be introduced by the API owner. In either case, the required semantics and failure categories in this document are mandatory. Metadata-only actions MUST continue to accept their existing action-only request semantics.

### 12.3 Request fingerprint

The fingerprint MUST include:

```text
operation = accept_conflict
conflict_id
expected_current_revision_id
expected_incoming_revision_id
expected_content_sha256
expected_size_bytes
authenticated principal scope
```

It MUST exclude raw content bytes, bearer credentials, the raw idempotency key, database details, and object-store paths.

### 12.4 Object-store preparation and authority rule

The conflict's `incoming_revision_id` MUST reference immutable preserved content metadata.

Before object-store verification, Server MUST perform a non-mutating lookup under the accepted adapter-write idempotency scope. When the same scope/key already has a committed response with the same fingerprint, Server MUST replay it immediately without requiring object-store availability. A different fingerprint MUST fail with `idempotency_conflict`. Absence of a record is only a preliminary observation; the authoritative transaction MUST lock and re-evaluate idempotency to resolve races.

When no committed replay exists, before opening the authoritative database transaction Server MUST verify one of these states:

1. the referenced content-addressed blob is already committed, readable, and matches the expected SHA-256 and size; or
2. when an accepted future boundary supplies bytes, the bytes are written to object-store staging, verified, and atomically finalized under the immutable expected hash before any authoritative database pointer is changed.

Object-store finalization MUST be create-or-verify and idempotent by content hash. It MUST NOT overwrite different bytes.

A finalized blob without a committed database revision is non-authoritative. Database rollback or commit failure MAY leave an unreferenced immutable blob, but MUST NOT leave a falsely authoritative revision. Such blobs are handled by a separate retention/consistency policy, never by automatic deletion in this mutation.

### 12.5 Lock order and atomic transaction

After object-store verification, Server MUST execute one database transaction with this lock order:

1. acquire a transaction-scoped idempotency lock for the authenticated scope/key;
2. evaluate any existing idempotency record;
3. read the conflict once to derive its normalized original path, then acquire the standard transaction-scoped path advisory lock;
4. load the conflict row `FOR UPDATE` and require status `open`;
5. load and lock the current object row and current revision state;
6. load the incoming conflict revision and its content metadata;
7. perform all precondition and consistency validation and invoke Core `plan_conflict_resolution` with the locked open conflict and `AcceptConflict`;
8. require the Core plan to specify `CreateCurrentRevisionFromConflict` and `MarkConflictCopyResolved`, then insert one immutable new file revision exactly from `NewCurrentRevisionPlan`;
9. update `sync_objects.current_revision_id` to the new revision;
10. append the existing V1 `conflict_resolved` operation-log entry with `conflict_id` and the new `revision_id`;
11. append the safe `conflict.accept_conflict.succeeded` audit event;
12. mark the conflict resolved and persist the Core `MarkConflictCopyResolved` disposition, or a transactionally created idempotent materialization work item that encodes that disposition, together with resolver identity and resolution timestamp;
13. persist the safe idempotency response;
14. commit.

All steps from lock acquisition through idempotency-response persistence MUST use the same SQL transaction. The transaction MUST commit only when every required database step succeeds.

The conflict row is reloaded after the path lock because the initial non-locking read is only a path lookup. The locked row is authoritative.

### 12.6 Required validation

Before inserting the new revision, Server MUST verify:

- the conflict exists and is still open;
- its original path matches the locked object path;
- the locked object's current revision equals `expected_current_revision_id`;
- the conflict's recorded current revision and the actual current revision are consistent, or the request fails stale rather than overwriting newer content;
- the conflict's incoming revision equals `expected_incoming_revision_id`;
- the incoming revision/content metadata is the preserved incoming side identified by the conflict and matches the original path expected by the Core planner;
- the incoming revision hash and size equal the conflict metadata and request preconditions;
- the committed object-store blob recomputes to the expected hash and size;
- the Core plan parent revision, content hash, size, `created_by`, and conflict-copy disposition exactly match the locked conflict facts;
- the principal remains authorized and maintenance admission remains `normal` for the current bound `maintenance_generation`.

### 12.7 Core-planner revision provenance and conflict-copy effects

The new revision MUST be created from the current Core `NewCurrentRevisionPlan` without reinterpretation:

```text
new revision id = Server-assigned immutable ID
same object id = locked current object
path = conflict.original_path
parent_revision_id = conflict.current_revision_id = previously current revision id
content_sha256 = conflict.incoming_content_hash
size_bytes = conflict.incoming_size_bytes
created_by = conflict.incoming_adapter_id
created_at = Server transaction time
```

`created_by` records the provenance of the accepted content and MUST NOT be replaced with the authenticated resolver identity. The authenticated resolver remains authoritative for `conflicts.resolved_by`, the resolution audit actor, authorization evidence, and request/idempotency scope.

The previous current revision MUST remain unchanged and queryable as history. The object current pointer MUST change exactly once to the new revision.

For `accept_conflict`, the Core planner's exact conflict-copy disposition is `MarkConflictCopyResolved`:

- the materialized conflict copy MUST no longer be represented as an active kept-both side after successful resolution;
- the disposition MUST be durably attached to the committed resolution result or a transactionally created idempotent post-commit materialization work item;
- physical adapter/Worktree handling MAY occur after database commit, but it MUST apply `MarkConflictCopyResolved`, MUST NOT reinterpret it as `KeepMaterializedConflictCopy` or `NoConflictCopyChange`, and MUST NOT delete authoritative revision/blob history;
- a later materialization failure is a safe degraded follow-up state and MUST NOT create another authoritative revision, change the revision provenance, or silently reopen the committed conflict.

The operation-log entry MUST use `conflict_resolved`, reference the conflict and new revision, and receive the globally monotonic sequence only inside the transaction. The audit event MUST identify the action, conflict, object, old current revision, new current revision, incoming content source adapter, resolver actor, and safe result category. It MUST not contain content or raw paths beyond the accepted normalized vault-path policy.

The conflict MUST be marked resolved only after the new revision, object pointer, operation log, audit event, Core conflict-copy disposition, and idempotency response have all been written successfully in the transaction.
