create table tombstones (
    tombstone_id text primary key,
    path text not null,
    deleted_revision_id text references file_revisions(revision_id),
    deleted_by text not null references sync_adapters(adapter_id),
    deleted_at timestamptz not null default now(),
    retention_until timestamptz not null,
    restored_at timestamptz
);

comment on table tombstones is 'Safe delete markers with retention. Delete guard, restoration, and physical cleanup behavior are implemented in future phases.';

create index tombstones_path_idx on tombstones(path);
create index tombstones_retention_idx on tombstones(retention_until);
create index tombstones_deleted_at_idx on tombstones(deleted_at);
create index tombstones_restored_idx on tombstones(restored_at);

create table conflicts (
    conflict_id text primary key,
    original_path text not null,
    base_revision_id text references file_revisions(revision_id),
    current_revision_id text not null references file_revisions(revision_id),
    incoming_revision_id text not null references file_revisions(revision_id),
    incoming_adapter_id text not null references sync_adapters(adapter_id),
    policy_applied text not null,
    materialized_path text not null,
    status text not null,
    created_at timestamptz not null default now(),
    resolved_at timestamptz,
    resolved_by text references sync_adapters(adapter_id)
);

comment on table conflicts is 'Records preserved conflict versions. Resolution policy and materialization behavior are implemented in future Core phases.';

create index conflicts_status_idx on conflicts(status);
create index conflicts_original_path_idx on conflicts(original_path);
create index conflicts_created_at_idx on conflicts(created_at);
create index conflicts_incoming_adapter_idx on conflicts(incoming_adapter_id);

alter table operation_log
    add constraint operation_log_tombstone_fk
    foreign key (tombstone_id) references tombstones(tombstone_id),
    add constraint operation_log_conflict_fk
    foreign key (conflict_id) references conflicts(conflict_id);
