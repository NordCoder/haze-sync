create table operation_log (
    seq bigserial primary key,
    op_id text not null unique,
    adapter_id text not null references sync_adapters(adapter_id),
    kind text not null,
    path text not null,
    revision_id text references file_revisions(revision_id),
    tombstone_id text,
    conflict_id text,
    created_at timestamptz not null default now()
);

comment on table operation_log is 'Append-only operation log consumed by adapters. Operation creation and cursor advancement behavior are implemented in future phases.';
comment on column operation_log.seq is 'Global adapter-consumable operation sequence. The primary key index supports changes-since-sequence scans.';

create index operation_log_adapter_idx on operation_log(adapter_id);
create index operation_log_path_idx on operation_log(path);
create index operation_log_created_at_idx on operation_log(created_at);
