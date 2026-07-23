# Recovery and Mutation Contracts — Downstream Responsibilities and Invariants

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

## 13. Downstream responsibilities

### 13.1 Storage

Storage follow-up MUST provide:

- the exact Stage 10 repositories for `haze-sync.operational-job.v1`, `job_version`, executor fences/leases, scoped/global execution slots, operational idempotency, confirmation bindings, `haze-sync.audit-event.v1`, adapter inventory/evidence, and recovery evidence;
- lock/read/update primitives needed by atomic `accept_conflict`, including conflict and object/current-revision locking under caller transactions;
- immutable revision insertion and current-pointer update composition under one caller transaction;
- idempotency uniqueness and transaction-safe replay persistence;
- backup/doctor fact providers for schema state, counts, operation sequence, blob metadata, selected-hash verification, and a verified ordinary-restore projection that excludes live control-plane families;
- migration compatibility evidence;
- no automatic hard-delete or orphan-blob cleanup as part of `accept_conflict`.

### 13.2 API

API follow-up MUST define:

- versioned operation/job request, confirmation, status, manifest-summary, and safe error DTOs;
- conditional precondition fields or a dedicated versioned route for `accept_conflict`;
- safe result mapping that preserves existing metadata-only action semantics;
- no raw content, credentials, provider-private state, internal paths, SQL, or helper diagnostics.

### 13.3 Server

Server follow-up MUST implement:

- maintenance-state admission and inventory-complete quiescence evidence using namespace-specific `maintenance_generation`, `adapter_inventory_generation`, and per-adapter `adapter_control_generation` values;
- exact Stage 10 job-CAS/idempotency/confirmation/audit choreography, executor fencing/leases, and global/scoped execution-slot control without a parallel recovery model;
- allowlisted helper invocation and result validation;
- recovery validation startup mode with every inventory adapter held/fenced, preserved live target control-plane authority, and exact job/fence/lease/slot reconciliation from the external recovery-control envelope;
- atomic `accept_conflict` application service using the lock and transaction order above;
- exact safe error/result mapping and ambiguous-commit retry behavior.

### 13.4 CLI

CLI follow-up MUST implement:

- job planning, submission, polling, confirmation, cancellation, and safe evidence display through public Server APIs using `job_version` CAS;
- same-key retry guidance;
- explicit target/manifest/release confirmation display;
- no direct SQL, object-store, provider, host-service, or shell access;
- no raw secret arguments or logs.

### 13.5 Deployment helper

Deployment follow-up MUST implement:

- the fixed typed helper interface and named resource resolution;
- private staging, create-only candidate publication, final-manifest commit-marker semantics, permissions, checksums, readability verification, uncertainty reconciliation, and bounded cleanup;
- PostgreSQL/object-store/optional-Worktree capture and restore with a filtered/isolated product-data PostgreSQL restore that cannot overwrite the live control plane;
- sequential migration invocation and exact result evidence;
- recovery validation launch/stop ordering and the CAS-versioned external recovery-control envelope outside the restore set, including complete adapter inventory and current executor-fence/lease/slot authority;
- no arbitrary shell or unapproved path access.

### 13.6 Worktree

Worktree follow-up MUST provide:

- identity-complete stop/quiesce/effective-state evidence for every configured Worktree instance;
- no in-flight cycle evidence;
- optional artifact inclusion rules based on authority/unreplicated-content status;
- deterministic restore verification counts and selected hashes;
- disabled startup until post-restore acceptance.

### 13.7 GDrive

GDrive follow-up MUST provide:

- identity-complete desired/effective disabled or quiesced generation handling for every configured GDrive instance;
- heartbeat and last-applied-generation evidence;
- stop/restart behavior that cannot submit writes during maintenance;
- no OAuth/provider-secret inclusion in manifests, jobs, helper requests, or audit;
- no claim that provider contents are part of the Haze Sync coordinated backup unless a separate provider-backup contract is accepted.

### 13.8 Acceptance CI

Acceptance CI follow-up MUST prove:

- backup to a private staging destination and final complete manifest;
- checksum/readability/count/selected-hash evidence;
- partial-failure cleanup, non-restorable staging/candidate state, manifest-last publication, and post-publication uncertainty reconciliation;
- idempotent retry and overwrite refusal;
- restore into a separate environment with an empty product-data target and a pre-provisioned isolated live control plane;
- live maintenance/inventory/job-version/executor-fence/lease/slot/idempotency/audit/credential/adapter-control records preserved across PostgreSQL product-data restore, with source control-plane rows unable to reactivate;
- schema/migration/object-store/Worktree verification;
- doctor and preflight result handling;
- failed migration/startup/schema/object/doctor gates remain in maintenance;
- rollback from accepted backup and compatible release set;
- crash/failure injection around job CAS/start, lease/fence allocation, destructive-slot acquisition/retention/release, envelope pending/arm/active/reconcile transitions, PostgreSQL unavailability, backup final-manifest creation, object-store finalization, revision insert, current-pointer update, operation/audit append, idempotency persistence, and commit;
- concurrent `accept_conflict`, stale revision, replay, missing content, hash mismatch, storage failure, ambiguous commit behavior, exact `created_by = incoming_adapter_id`, resolver separation, and `MarkConflictCopyResolved` disposition;
- absence of executable changes from this documentation Candidate.

## 14. V1 safety invariants

1. No recovery operation begins without current inventory-complete quiescence evidence.
2. No destructive restore/rollout/rollback begins without current `job_version`, bound one-time confirmation, executor lease/fence, and global destructive slot.
3. CLI never becomes an infrastructure shell, SQL client, object-store client, or provider-private client.
4. A complete backup is one manifest-bound recovery set from one quiesced window, committed by an immutable final manifest created last.
5. Partial/candidate artifacts are never restorable merely because files exist; a durable complete manifest cannot coexist with a failed-publication claim and requires reconciliation on uncertain delivery.
6. The initial restore target is separate with an empty product-data area; its live control plane and external recovery envelope remain outside the restore set.
7. Ordinary restore never overwrites namespace-specific maintenance/inventory/adapter/principal/credential/job/fence/lease/slot/idempotency/audit authority.
8. Quiescence and recovery envelopes enumerate every configured Worktree/GDrive identity, including desired-disabled instances; aggregate singleton status cannot prove safety.
9. The external recovery envelope grants no independent job authority; it preserves the exact canonical operation, job version, executor fence/lease, destructive slot, inventory, and evidence while PostgreSQL is unavailable.
10. Lease expiry alone never releases a destructive slot, authorizes takeover, or proves an external effect stopped.
11. Adapters remain held or individually fenced until post-operation doctor/preflight and explicit resume.
12. Rollback restores an accepted backup and compatible release set; it does not guess reverse migrations or bypass prior-slot reconciliation.
13. Previous authoritative revisions remain immutable history.
14. Object-store bytes are non-authoritative until referenced by a committed database revision/current pointer.
15. `accept_conflict` is one atomic database transaction after verified immutable content preparation and exactly preserves Core planner provenance/disposition.
16. Idempotency replay returns the original effect; key reuse with a different fingerprint performs no work.
17. Metadata-only conflict actions remain metadata-only with their exact Core conflict-copy dispositions.
18. Public output, manifests, audit, and logs remain free of secrets, content, lease/confirmation material, internal SQL, provider payloads, database URLs, and host paths.
19. Missing, stale, skipped, or uncertain evidence is never reported as success.
