do $$
begin
    if to_regclass(current_schema() || '.gdrive_adapter_state') is not null
        or to_regclass(current_schema() || '.gdrive_durable_items') is not null
        or to_regclass(current_schema() || '.gdrive_operations') is not null
    then
        raise exception 'gdrive durable state structures require explicit operator review';
    end if;
end
$$;

create table gdrive_adapter_state (
    adapter_id text primary key references sync_adapters(adapter_id),
    state_format_version integer not null default 1
        constraint gdrive_adapter_state_format_version_supported check (state_format_version = 1),
    state_version bigint not null default 0
        constraint gdrive_adapter_state_version_nonnegative check (state_version >= 0),
    drive_cursor text constraint gdrive_adapter_cursor_bounded check (drive_cursor is null or (char_length(drive_cursor) between 1 and 8192)),
    drive_cursor_generation bigint not null default 0
        constraint gdrive_adapter_cursor_generation_nonnegative check (drive_cursor_generation >= 0),
    core_export_seq bigint not null default 0
        constraint gdrive_adapter_core_export_seq_nonnegative check (core_export_seq >= 0),
    last_import_operation_id text,
    last_export_operation_id text,
    last_provider_mutation_operation_id text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

comment on table gdrive_adapter_state is 'Versioned adapter-scoped Google Drive progress facts. Server owns transaction choreography and the adapter has no direct database access.';
comment on column gdrive_adapter_state.drive_cursor is 'Opaque provider cursor required for restart correctness. Never render in Debug, status, logs, errors, or reports.';

create table gdrive_durable_items (
    adapter_id text not null references gdrive_adapter_state(adapter_id),
    path text not null,
    drive_file_id text,
    drive_parent_id text,
    drive_name text,
    mime_type text,
    md5_checksum text,
    head_revision_id text,
    drive_version text,
    drive_modified_time timestamptz,
    core_object_id text references sync_objects(object_id),
    core_revision_id text references file_revisions(revision_id),
    core_seq bigint constraint gdrive_durable_items_core_seq_nonnegative check (core_seq is null or core_seq >= 0),
    echo_state text not null default 'none',
    echo_operation_id text,
    echo_provider_version text,
    delete_candidate_first_seen_at timestamptz,
    delete_candidate_last_seen_at timestamptz,
    delete_candidate_generation bigint,
    delete_candidate_blocked boolean not null default false,
    delete_confirmation_audit_id text references audit_events(audit_id),
    last_imported_at timestamptz,
    last_exported_at timestamptz,
    last_seen_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    primary key (adapter_id, path),
    unique (adapter_id, drive_file_id),
    constraint gdrive_durable_items_echo_consistent check (
        (echo_state = 'none' and echo_operation_id is null and echo_provider_version is null)
        or (echo_state = 'pending' and echo_operation_id is not null and echo_provider_version is null)
        or (echo_state = 'confirmed' and echo_operation_id is not null and echo_provider_version is not null)
    ),
    constraint gdrive_durable_items_delete_candidate_consistent check (
        (
            delete_candidate_first_seen_at is null
            and delete_candidate_last_seen_at is null
            and delete_candidate_generation is null
            and delete_candidate_blocked = false
            and delete_confirmation_audit_id is null
        )
        or (
            delete_candidate_first_seen_at is not null
            and delete_candidate_last_seen_at is not null
            and delete_candidate_last_seen_at >= delete_candidate_first_seen_at
            and delete_candidate_generation is not null
            and delete_candidate_generation >= 0
        )
    )
);

comment on table gdrive_durable_items is 'Adapter-scoped caller-confirmed Drive/Core mapping, echo, and delete-candidate facts. Storage does not decide provider or delete policy.';

create index gdrive_durable_items_drive_file_idx
    on gdrive_durable_items(adapter_id, drive_file_id);
create index gdrive_durable_items_core_revision_idx
    on gdrive_durable_items(core_revision_id);
create index gdrive_durable_items_core_seq_idx
    on gdrive_durable_items(adapter_id, core_seq);
create index gdrive_durable_items_delete_candidate_idx
    on gdrive_durable_items(adapter_id, delete_candidate_generation, path)
    where delete_candidate_generation is not null;

create table gdrive_operations (
    adapter_id text not null references gdrive_adapter_state(adapter_id),
    operation_id text not null,
    operation_kind text not null constraint gdrive_operations_kind_supported check (operation_kind in ('import', 'export', 'provider_mutation', 'cursor_checkpoint', 'delete_candidate')),
    facts_hash text not null constraint gdrive_operations_facts_hash_shape check (facts_hash ~ '^[0-9A-Fa-f]{64}$'),
    outcome_kind text not null constraint gdrive_operations_outcome_supported check (outcome_kind = 'committed'),
    committed_state_version bigint not null
        constraint gdrive_operations_state_version_nonnegative check (committed_state_version >= 0),
    mapping_path text,
    core_seq bigint constraint gdrive_operations_core_seq_nonnegative check (core_seq is null or core_seq >= 0),
    drive_version text,
    created_at timestamptz not null default now(),
    primary key (adapter_id, operation_id),
    foreign key (adapter_id, mapping_path)
        references gdrive_durable_items(adapter_id, path)
);

comment on table gdrive_operations is 'Retry-safe typed operation outcomes. Stores fingerprints and committed facts only, never raw provider payloads or request bodies.';

create index gdrive_operations_state_version_idx
    on gdrive_operations(adapter_id, committed_state_version);
