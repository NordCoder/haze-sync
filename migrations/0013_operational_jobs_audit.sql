-- Stage 11 canonical operational-job, execution-slot, evidence, and audit storage.

create table operational_jobs (
    operation_id text primary key,
    schema_version integer not null default 1 check (schema_version = 1),
    job_version bigint not null check (job_version >= 1),
    kind text not null,
    maintenance_required boolean not null,
    destructive boolean not null,
    requester_principal_id text not null references principals(principal_id),
    requester_credential_id text references credentials(credential_id),
    idempotency_scope text not null,
    idempotency_key_digest text not null check (idempotency_key_digest ~ '^[0-9a-f]{64}$'),
    request_fingerprint text not null check (request_fingerprint ~ '^[0-9a-f]{64}$'),
    state text not null check (state in ('planned', 'awaiting_confirmation', 'running', 'succeeded', 'failed', 'cancelled')),
    dry_run boolean not null,
    confirmation_required boolean not null,
    confirmation_digest text check (confirmation_digest is null or confirmation_digest ~ '^[0-9a-f]{64}$'),
    confirmation_expires_at timestamptz,
    confirmation_consumed_at timestamptz,
    expected_maintenance_generation bigint check (expected_maintenance_generation >= 0),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    started_at timestamptz,
    completed_at timestamptz,
    cancel_requested_at timestamptz,
    safe_summary jsonb not null default '{}'::jsonb,
    safe_error_category text,
    artifact_manifest_id text,
    artifact_manifest_digest text check (artifact_manifest_digest is null or artifact_manifest_digest ~ '^[0-9a-f]{64}$'),
    checkpoint jsonb,
    execution_scope_digest text not null check (execution_scope_digest ~ '^[0-9a-f]{64}$'),
    executor_id text,
    executor_fence bigint not null default 0 check (executor_fence >= 0),
    lease_token_digest text check (lease_token_digest is null or lease_token_digest ~ '^[0-9a-f]{64}$'),
    lease_heartbeat_at timestamptz,
    lease_expires_at timestamptz,
    destructive_execution_slot_id text,
    retry_of_operation_id text references operational_jobs(operation_id),
    check ((not confirmation_required and confirmation_digest is null and confirmation_expires_at is null)
        or confirmation_required),
    check ((state in ('succeeded', 'failed', 'cancelled') and completed_at is not null)
        or state not in ('succeeded', 'failed', 'cancelled'))
);

create table operational_job_adapter_generations (
    operation_id text not null references operational_jobs(operation_id),
    adapter_id text not null,
    adapter_control_generation bigint not null check (adapter_control_generation >= 0),
    primary key (operation_id, adapter_id)
);

create table operational_execution_slots (
    slot_id text primary key,
    schema_version integer not null default 1 check (schema_version = 1),
    slot_version bigint not null default 0 check (slot_version >= 0),
    slot_kind text not null check (slot_kind in ('global_destructive', 'scoped')),
    execution_scope_digest text check (execution_scope_digest is null or execution_scope_digest ~ '^[0-9a-f]{64}$'),
    operation_id text references operational_jobs(operation_id),
    maintenance_generation bigint check (maintenance_generation >= 0),
    executor_fence bigint check (executor_fence >= 0),
    blocked_uncertain boolean not null default false,
    acquired_at timestamptz,
    released_at timestamptz,
    updated_at timestamptz not null default now(),
    check ((slot_kind = 'global_destructive' and execution_scope_digest is null)
        or (slot_kind = 'scoped' and execution_scope_digest is not null)),
    check ((operation_id is null and acquired_at is null) or operation_id is not null)
);

insert into operational_execution_slots (slot_id, slot_kind)
values ('global-destructive', 'global_destructive');

create unique index operational_execution_slots_one_global_uq
    on operational_execution_slots(slot_kind) where slot_kind = 'global_destructive';

alter table operational_jobs
    add constraint operational_jobs_execution_slot_fk
    foreign key (destructive_execution_slot_id) references operational_execution_slots(slot_id);

create unique index operational_execution_slots_scope_active_uq
    on operational_execution_slots(execution_scope_digest)
    where slot_kind = 'scoped' and operation_id is not null;

create table operational_idempotency (
    requester_principal_id text not null references principals(principal_id),
    operation_kind text not null,
    idempotency_key_digest text not null check (idempotency_key_digest ~ '^[0-9a-f]{64}$'),
    schema_version integer not null default 1 check (schema_version = 1),
    request_fingerprint text not null check (request_fingerprint ~ '^[0-9a-f]{64}$'),
    operation_id text not null references operational_jobs(operation_id),
    created_at timestamptz not null default now(),
    primary key (requester_principal_id, operation_kind, idempotency_key_digest)
);

create table operational_job_evidence (
    evidence_id text primary key,
    schema_version integer not null default 1 check (schema_version = 1),
    operation_id text not null references operational_jobs(operation_id),
    job_version bigint not null check (job_version >= 1),
    evidence_kind text not null,
    evidence_schema text not null,
    evidence_version bigint not null check (evidence_version >= 1),
    evidence_digest text not null check (evidence_digest ~ '^[0-9a-f]{64}$'),
    status text not null check (status in ('not_run', 'skipped', 'failed', 'blocked_uncertain', 'passed')),
    safe_metadata jsonb not null,
    source_environment_id text,
    target_environment_id text,
    helper_attempt_id text,
    created_at timestamptz not null default now()
);

create index operational_job_evidence_operation_idx
    on operational_job_evidence(operation_id, job_version, evidence_kind);

create table operational_audit_events (
    audit_id text primary key,
    schema_version integer not null default 1 check (schema_version = 1),
    occurred_at timestamptz not null,
    event_type text not null,
    outcome text not null,
    actor_principal_id text,
    actor_credential_id text,
    actor_role text,
    actor_origin text not null,
    request_id text,
    correlation_id text,
    target_type text not null,
    target_id text,
    operation_id text,
    maintenance_generation bigint,
    adapter_control_generation bigint,
    principal_version bigint,
    credential_set_generation bigint,
    credential_version bigint,
    job_version bigint,
    executor_fence bigint,
    runtime_lease_version bigint,
    standalone_runtime_epoch bigint,
    runtime_report_sequence bigint,
    issuance_operation_id text,
    previous_state text,
    next_state text,
    safe_error_category text,
    artifact_manifest_id text,
    safe_metadata jsonb not null default '{}'::jsonb,
    corrects_audit_id text references operational_audit_events(audit_id),
    check (maintenance_generation is null or maintenance_generation >= 0),
    check (adapter_control_generation is null or adapter_control_generation >= 0),
    check (principal_version is null or principal_version >= 0),
    check (credential_set_generation is null or credential_set_generation >= 0),
    check (credential_version is null or credential_version >= 0),
    check (job_version is null or job_version >= 0),
    check (executor_fence is null or executor_fence >= 0),
    check (runtime_lease_version is null or runtime_lease_version >= 0),
    check (standalone_runtime_epoch is null or standalone_runtime_epoch >= 0),
    check (runtime_report_sequence is null or runtime_report_sequence >= 0)
);

create index operational_audit_events_operation_idx
    on operational_audit_events(operation_id, occurred_at, audit_id);
create index operational_audit_events_target_idx
    on operational_audit_events(target_type, target_id, occurred_at);
