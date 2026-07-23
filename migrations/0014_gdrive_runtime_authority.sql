-- Stage 11 standalone GDrive runtime authority and immutable/no-delete guards.

create table gdrive_runtime_authorities (
    adapter_id text primary key references adapter_inventory(adapter_id),
    schema_version integer not null default 1 check (schema_version = 1),
    runtime_lease_version bigint not null default 0 check (runtime_lease_version >= 0),
    standalone_runtime_epoch bigint not null default 0 check (standalone_runtime_epoch >= 0),
    runtime_instance_id text,
    lease_token_digest text check (lease_token_digest is null or lease_token_digest ~ '^[0-9a-f]{64}$'),
    lease_heartbeat_at timestamptz,
    lease_expires_at timestamptz,
    last_accepted_report_sequence bigint not null default 0 check (last_accepted_report_sequence >= 0),
    last_accepted_report_fingerprint text,
    open_mutation_permits bigint not null default 0 check (open_mutation_permits >= 0),
    uncertain_external_effects bigint not null default 0 check (uncertain_external_effects >= 0),
    takeover_state text not null default 'clear' check (takeover_state in ('clear', 'reconciliation_required')),
    updated_at timestamptz not null default now(),
    check ((runtime_instance_id is null and lease_token_digest is null and lease_heartbeat_at is null and lease_expires_at is null)
        or (runtime_instance_id is not null and lease_token_digest is not null and lease_heartbeat_at is not null and lease_expires_at is not null)),
    check (lease_expires_at is null or lease_expires_at > lease_heartbeat_at),
    check (last_accepted_report_fingerprint is null or last_accepted_report_fingerprint ~ '^[0-9a-f]{64}$')
);

create table gdrive_runtime_reports (
    adapter_id text not null references gdrive_runtime_authorities(adapter_id),
    standalone_runtime_epoch bigint not null check (standalone_runtime_epoch >= 1),
    report_sequence bigint not null check (report_sequence >= 1),
    schema_version integer not null default 1 check (schema_version = 1),
    runtime_instance_id text not null,
    accepted_runtime_lease_version bigint not null check (accepted_runtime_lease_version >= 1),
    report_fingerprint text not null check (report_fingerprint ~ '^[0-9a-f]{64}$'),
    adapter_control_generation bigint not null check (adapter_control_generation >= 0),
    maintenance_generation bigint not null check (maintenance_generation >= 0),
    in_flight boolean not null,
    open_mutation_permits bigint not null check (open_mutation_permits >= 0),
    checkpoint_summary jsonb not null,
    safe_error_category text,
    safe_error_code text,
    reported_at timestamptz not null,
    accepted_at timestamptz not null default now(),
    primary key (adapter_id, standalone_runtime_epoch, report_sequence)
);

create table gdrive_mutation_permits (
    permit_id text primary key,
    schema_version integer not null default 1 check (schema_version = 1),
    adapter_id text not null references gdrive_runtime_authorities(adapter_id),
    runtime_instance_id text not null,
    standalone_runtime_epoch bigint not null check (standalone_runtime_epoch >= 1),
    runtime_lease_version bigint not null check (runtime_lease_version >= 1),
    adapter_control_generation bigint not null check (adapter_control_generation >= 0),
    maintenance_generation bigint not null check (maintenance_generation >= 0),
    step_id text not null,
    permit_proof_digest text not null check (permit_proof_digest ~ '^[0-9a-f]{64}$'),
    bounded_expires_at timestamptz not null,
    state text not null check (state in ('open', 'closed', 'uncertain')),
    terminal_outcome text,
    checkpoint_summary jsonb,
    opened_at timestamptz not null default now(),
    closed_at timestamptz,
    unique (adapter_id, standalone_runtime_epoch, step_id),
    check ((state = 'open' and closed_at is null) or (state <> 'open' and closed_at is not null))
);

create index gdrive_mutation_permits_open_idx
    on gdrive_mutation_permits(adapter_id, standalone_runtime_epoch)
    where state = 'open';

create table gdrive_uncertain_effects (
    effect_id text primary key,
    schema_version integer not null default 1 check (schema_version = 1),
    adapter_id text not null references gdrive_runtime_authorities(adapter_id),
    standalone_runtime_epoch bigint not null check (standalone_runtime_epoch >= 1),
    permit_id text references gdrive_mutation_permits(permit_id),
    step_id text not null,
    effect_identity_digest text not null check (effect_identity_digest ~ '^[0-9a-f]{64}$'),
    state text not null check (state in ('unresolved', 'reconciled', 'externally_fenced')),
    external_fence_id text,
    safe_metadata jsonb not null default '{}'::jsonb,
    discovered_at timestamptz not null default now(),
    resolved_at timestamptz,
    check ((state = 'unresolved' and resolved_at is null) or (state <> 'unresolved' and resolved_at is not null)),
    check ((state = 'externally_fenced' and external_fence_id is not null) or state <> 'externally_fenced')
);

create index gdrive_uncertain_effects_unresolved_idx
    on gdrive_uncertain_effects(adapter_id, standalone_runtime_epoch)
    where state = 'unresolved';

create function haze_sync_reject_immutable_mutation() returns trigger
language plpgsql as $$
begin
    raise exception 'immutable control-plane row cannot be changed' using errcode = '55000';
end
$$;

create function haze_sync_reject_control_delete() returns trigger
language plpgsql as $$
begin
    raise exception 'durable control-plane row cannot be deleted' using errcode = '55000';
end
$$;


create trigger quiescence_evidence_immutable
before update or delete on quiescence_evidence
for each row execute function haze_sync_reject_immutable_mutation();

create trigger quiescence_evidence_invalidations_immutable
before update or delete on quiescence_evidence_invalidations
for each row execute function haze_sync_reject_immutable_mutation();

create trigger quiescence_adapter_snapshots_immutable
before update or delete on quiescence_adapter_snapshots
for each row execute function haze_sync_reject_immutable_mutation();

create trigger quiescence_runtime_snapshots_immutable
before update or delete on quiescence_runtime_snapshots
for each row execute function haze_sync_reject_immutable_mutation();

create trigger credential_issuance_idempotency_immutable
before update or delete on credential_issuance_idempotency
for each row execute function haze_sync_reject_immutable_mutation();

create trigger operational_idempotency_immutable
before update or delete on operational_idempotency
for each row execute function haze_sync_reject_immutable_mutation();

create trigger operational_job_evidence_immutable
before update or delete on operational_job_evidence
for each row execute function haze_sync_reject_immutable_mutation();

create trigger operational_audit_events_append_only
before update or delete on operational_audit_events
for each row execute function haze_sync_reject_immutable_mutation();

create trigger gdrive_runtime_reports_immutable
before update or delete on gdrive_runtime_reports
for each row execute function haze_sync_reject_immutable_mutation();

create trigger maintenance_control_no_hard_delete
before delete on maintenance_control
for each row execute function haze_sync_reject_control_delete();

create trigger adapter_inventory_state_no_hard_delete
before delete on adapter_inventory_state
for each row execute function haze_sync_reject_control_delete();

create trigger adapter_inventory_no_hard_delete
before delete on adapter_inventory
for each row execute function haze_sync_reject_control_delete();

create trigger adapter_desired_controls_no_hard_delete
before delete on adapter_desired_controls
for each row execute function haze_sync_reject_control_delete();

create trigger adapter_effective_controls_no_hard_delete
before delete on adapter_effective_controls
for each row execute function haze_sync_reject_control_delete();

create trigger principals_no_hard_delete
before delete on principals
for each row execute function haze_sync_reject_control_delete();

create trigger credentials_no_hard_delete
before delete on credentials
for each row execute function haze_sync_reject_control_delete();

create trigger operational_jobs_no_hard_delete
before delete on operational_jobs
for each row execute function haze_sync_reject_control_delete();

create trigger operational_execution_slots_no_hard_delete
before delete on operational_execution_slots
for each row execute function haze_sync_reject_control_delete();

create trigger gdrive_runtime_authorities_no_hard_delete
before delete on gdrive_runtime_authorities
for each row execute function haze_sync_reject_control_delete();

create trigger gdrive_mutation_permits_no_hard_delete
before delete on gdrive_mutation_permits
for each row execute function haze_sync_reject_control_delete();

create trigger gdrive_uncertain_effects_no_hard_delete
before delete on gdrive_uncertain_effects
for each row execute function haze_sync_reject_control_delete();

create trigger operational_job_adapter_generations_immutable
before update or delete on operational_job_adapter_generations
for each row execute function haze_sync_reject_immutable_mutation();

create function haze_sync_reject_terminal_job_mutation() returns trigger
language plpgsql as $$
begin
    if old.state in ('succeeded', 'failed', 'cancelled') and to_jsonb(new) is distinct from to_jsonb(old) then
        raise exception 'terminal operational job is immutable' using errcode = '55000';
    end if;
    return new;
end
$$;

create trigger operational_jobs_terminal_immutable
before update on operational_jobs
for each row execute function haze_sync_reject_terminal_job_mutation();

comment on table maintenance_control is 'Singleton passive maintenance state; Server owns transition policy.';
comment on table adapter_desired_controls is 'Durable desired adapter controls; Storage does not choose legal modes.';
comment on table adapter_effective_controls is 'Caller-reported effective adapter control facts.';
comment on table credentials is 'Private credential verifier records. verifier_material must never be publicly rendered.';
comment on table operational_jobs is 'Canonical haze-sync.operational-job.v1 persistence.';
comment on table operational_audit_events is 'Append-only haze-sync.audit-event.v1 persistence.';
comment on table gdrive_runtime_authorities is 'Standalone GDrive lease, epoch, ordered-report, permit, and takeover authority.';
