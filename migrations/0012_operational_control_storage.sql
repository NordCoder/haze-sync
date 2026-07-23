-- Stage 11 durable operational-control storage foundation.
-- This migration is forward-only and intentionally does not run product policy.

create table maintenance_control (
    singleton_id smallint primary key default 1 check (singleton_id = 1),
    schema_version integer not null default 1 check (schema_version = 1),
    maintenance_generation bigint not null default 0 check (maintenance_generation >= 0),
    state text not null default 'normal' check (state in ('normal', 'quiescing', 'quiesced', 'maintenance', 'resuming')),
    transition_operation_id text,
    transition_requested_by text,
    transition_requested_at timestamptz,
    state_entered_at timestamptz not null default now(),
    admission_fence_closed boolean not null default false,
    admission_fence_closed_at timestamptz,
    quiescence_evidence_version bigint not null default 0 check (quiescence_evidence_version >= 0),
    quiescence_evidence_id text,
    active_maintenance_job_id text,
    safe_error_category text,
    updated_at timestamptz not null default now(),
    check ((admission_fence_closed and admission_fence_closed_at is not null) or not admission_fence_closed)
);

insert into maintenance_control (singleton_id) values (1);

create table adapter_inventory_state (
    singleton_id smallint primary key default 1 check (singleton_id = 1),
    schema_version integer not null default 1 check (schema_version = 1),
    adapter_inventory_generation bigint not null default 0 check (adapter_inventory_generation >= 0),
    updated_at timestamptz not null default now()
);

insert into adapter_inventory_state (singleton_id) values (1);

create table adapter_inventory (
    adapter_id text primary key,
    schema_version integer not null default 1 check (schema_version = 1),
    adapter_kind text not null,
    control_authority text not null,
    inventory_generation bigint not null check (inventory_generation >= 0),
    configured_at timestamptz not null default now(),
    retired_at timestamptz,
    check (adapter_id <> '' and adapter_kind <> '' and control_authority <> '')
);

create index adapter_inventory_generation_idx on adapter_inventory(inventory_generation, adapter_id);

create table adapter_desired_controls (
    adapter_id text primary key references adapter_inventory(adapter_id),
    schema_version integer not null default 1 check (schema_version = 1),
    desired_enabled boolean not null,
    desired_mode text not null,
    adapter_control_generation bigint not null check (adapter_control_generation >= 0),
    maintenance_generation bigint not null check (maintenance_generation >= 0),
    maintenance_hold boolean not null,
    updated_by text not null,
    updated_at timestamptz not null default now(),
    check ((not desired_enabled and desired_mode = 'disabled') or (desired_enabled and desired_mode <> 'disabled'))
);

create table adapter_effective_controls (
    adapter_id text primary key references adapter_inventory(adapter_id),
    schema_version integer not null default 1 check (schema_version = 1),
    effective_mode text,
    runtime_lifecycle text not null check (runtime_lifecycle in ('stopped', 'starting', 'idle', 'running', 'draining', 'backing_off', 'failed')),
    last_applied_adapter_control_generation bigint check (last_applied_adapter_control_generation >= 0),
    last_applied_maintenance_generation bigint check (last_applied_maintenance_generation >= 0),
    heartbeat_at timestamptz,
    last_success_at timestamptz,
    last_cycle_started_at timestamptz,
    last_cycle_finished_at timestamptz,
    in_flight boolean not null,
    connection_state text not null check (connection_state in ('connected', 'stale', 'disconnected')),
    safe_error_category text,
    safe_error_code text,
    runtime_instance_id text,
    standalone_runtime_epoch bigint check (standalone_runtime_epoch >= 0),
    report_sequence bigint check (report_sequence >= 0),
    runtime_lease_version bigint check (runtime_lease_version >= 0),
    runtime_lease_expires_at timestamptz,
    checkpoint_summary jsonb not null default '{}'::jsonb,
    reported_at timestamptz not null,
    updated_at timestamptz not null default now(),
    check ((runtime_instance_id is null and standalone_runtime_epoch is null and report_sequence is null and runtime_lease_version is null)
        or (runtime_instance_id is not null and standalone_runtime_epoch is not null and report_sequence is not null and runtime_lease_version is not null))
);

create table quiescence_evidence (
    quiescence_evidence_id text primary key,
    schema_version integer not null default 1 check (schema_version = 1),
    evidence_version bigint not null check (evidence_version >= 1),
    maintenance_generation bigint not null check (maintenance_generation >= 0),
    adapter_inventory_generation bigint not null check (adapter_inventory_generation >= 0),
    admission_fence_closed_at timestamptz not null,
    server_instance_id text not null,
    server_started_at timestamptz not null,
    active_authoritative_mutations bigint not null check (active_authoritative_mutations = 0),
    open_authoritative_transactions bigint not null check (open_authoritative_transactions = 0),
    obsidian_authoritative_mutation_gate text not null check (obsidian_authoritative_mutation_gate = 'closed'),
    evidence_digest text not null check (evidence_digest ~ '^[0-9a-f]{64}$'),
    captured_at timestamptz not null
);

create table quiescence_evidence_invalidations (
    invalidation_id text primary key,
    quiescence_evidence_id text not null references quiescence_evidence(quiescence_evidence_id),
    maintenance_generation bigint not null check (maintenance_generation >= 0),
    invalidation_reason text not null,
    invalidated_at timestamptz not null default now(),
    check (invalidation_id <> '' and invalidation_reason <> '')
);

create index quiescence_evidence_invalidations_evidence_idx
    on quiescence_evidence_invalidations(quiescence_evidence_id, invalidated_at);

create table quiescence_adapter_snapshots (
    quiescence_evidence_id text not null references quiescence_evidence(quiescence_evidence_id),
    adapter_id text not null,
    adapter_kind text not null,
    control_authority text not null,
    adapter_control_generation bigint not null check (adapter_control_generation >= 0),
    maintenance_generation bigint not null check (maintenance_generation >= 0),
    desired_enabled boolean not null,
    desired_mode text not null,
    last_applied_adapter_control_generation bigint,
    last_applied_maintenance_generation bigint,
    runtime_lifecycle text not null,
    connection_state text not null,
    in_flight boolean not null check (not in_flight),
    drain_proof text not null check (drain_proof in ('acknowledged', 'externally_fenced')),
    external_fence_id text,
    checkpoint_summary jsonb not null,
    captured_at timestamptz not null,
    primary key (quiescence_evidence_id, adapter_id),
    check ((drain_proof = 'acknowledged' and external_fence_id is null)
        or (drain_proof = 'externally_fenced' and external_fence_id is not null))
);

create table quiescence_runtime_snapshots (
    quiescence_evidence_id text not null,
    adapter_id text not null,
    runtime_instance_id text not null,
    runtime_lease_version bigint not null check (runtime_lease_version >= 0),
    standalone_runtime_epoch bigint not null check (standalone_runtime_epoch >= 0),
    last_accepted_report_sequence bigint not null check (last_accepted_report_sequence >= 0),
    last_accepted_report_fingerprint text not null check (last_accepted_report_fingerprint ~ '^[0-9a-f]{64}$'),
    open_mutation_permits bigint not null check (open_mutation_permits = 0),
    uncertain_external_effects bigint not null check (uncertain_external_effects = 0),
    takeover_state text not null check (takeover_state = 'clear'),
    runtime_lease_expires_at timestamptz,
    external_fence_id text,
    captured_at timestamptz not null,
    primary key (quiescence_evidence_id, adapter_id),
    foreign key (quiescence_evidence_id, adapter_id)
        references quiescence_adapter_snapshots(quiescence_evidence_id, adapter_id)
);

create table principals (
    principal_id text primary key,
    schema_version integer not null default 1 check (schema_version = 1),
    principal_kind text not null default 'adapter',
    role text not null,
    principal_enabled boolean not null,
    principal_version bigint not null check (principal_version >= 1),
    credential_set_generation bigint not null check (credential_set_generation >= 1),
    created_at timestamptz not null,
    updated_at timestamptz not null,
    disabled_at timestamptz,
    check (principal_id <> '' and role <> '' and principal_kind <> '')
);

create table credentials (
    credential_id text primary key,
    schema_version integer not null default 1 check (schema_version = 1),
    credential_version bigint not null check (credential_version >= 1),
    principal_id text not null references principals(principal_id),
    issuance_operation_id text,
    credential_kind text not null,
    status text not null check (status in ('active', 'grace', 'revoked', 'expired')),
    verifier_scheme text not null check (verifier_scheme in ('argon2id_v1', 'legacy_sha256_v0')),
    verifier_material text not null,
    legacy_migrated boolean not null default false,
    created_at timestamptz not null,
    not_before timestamptz not null,
    expires_at timestamptz,
    grace_expires_at timestamptz,
    rotated_from_credential_id text references credentials(credential_id),
    revoked_at timestamptz,
    revoked_by text,
    last_used_at timestamptz,
    updated_at timestamptz not null default now(),
    check (credential_id <> '' and credential_kind <> '' and verifier_material <> ''),
    check (verifier_scheme <> 'legacy_sha256_v0' or legacy_migrated),
    check ((status = 'grace' and grace_expires_at is not null) or status <> 'grace'),
    check ((status = 'revoked' and revoked_at is not null) or status <> 'revoked')
);

create unique index credentials_one_active_per_principal_uq
    on credentials(principal_id) where status = 'active';
create unique index credentials_one_grace_per_principal_uq
    on credentials(principal_id) where status = 'grace';
create unique index credentials_legacy_sha256_lookup_uq
    on credentials(verifier_material) where verifier_scheme = 'legacy_sha256_v0';
create index credentials_principal_idx on credentials(principal_id, credential_id);

create table credential_issuance_idempotency (
    requester_principal_id text not null references principals(principal_id),
    idempotency_key_digest text not null check (idempotency_key_digest ~ '^[0-9a-f]{64}$'),
    schema_version integer not null default 1 check (schema_version = 1),
    request_fingerprint text not null check (request_fingerprint ~ '^[0-9a-f]{64}$'),
    issuance_operation_id text not null,
    action text not null check (action in ('create', 'rotate')),
    target_principal_id text not null references principals(principal_id),
    affected_credential_ids jsonb not null default '[]'::jsonb,
    committed_outcome text not null,
    safe_result jsonb not null,
    plaintext_available boolean not null default false check (not plaintext_available),
    committed_at timestamptz not null default now(),
    compacted_at timestamptz,
    primary key (requester_principal_id, idempotency_key_digest)
);

-- Validate accepted legacy identities before any compatibility rows are created.
do $$
begin
    if exists (
        select 1 from sync_adapters
        where adapter_id = '' or role = '' or token_hash !~ '^[0-9A-Fa-f]{64}$'
    ) then
        raise exception 'legacy adapter credential input is malformed' using errcode = '23514';
    end if;
    if exists (
        select 1 from sync_adapters
        group by lower(token_hash)
        having count(*) > 1
    ) then
        raise exception 'legacy adapter credential input is ambiguous' using errcode = '23505';
    end if;
end
$$;

insert into principals (
    principal_id, principal_kind, role, principal_enabled,
    principal_version, credential_set_generation, created_at, updated_at, disabled_at
)
select adapter_id, 'adapter', role, enabled, 1, 1, created_at, created_at,
    case when enabled then null else created_at end
from sync_adapters;

insert into credentials (
    credential_id, credential_version, principal_id, issuance_operation_id,
    credential_kind, status, verifier_scheme, verifier_material,
    legacy_migrated, created_at, not_before, updated_at
)
select
    'legacy-' || md5(adapter_id),
    1,
    adapter_id,
    null,
    'bearer',
    'active',
    'legacy_sha256_v0',
    lower(token_hash),
    true,
    created_at,
    created_at,
    created_at
from sync_adapters;
