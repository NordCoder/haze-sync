create table audit_events (
    audit_id text primary key,
    actor_adapter_id text references sync_adapters(adapter_id),
    event_type text not null,
    path text,
    revision_id text references file_revisions(revision_id),
    metadata jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now()
);

comment on table audit_events is 'Append-oriented audit event records for operational visibility. Event production policy is implemented in future phases.';
comment on column audit_events.metadata is 'Safe structured metadata only; do not store raw file contents, tokens, OAuth secrets, or private provider payloads.';

create index audit_events_created_at_idx on audit_events(created_at);
create index audit_events_path_idx on audit_events(path);
create index audit_events_event_type_idx on audit_events(event_type);
create index audit_events_actor_adapter_idx on audit_events(actor_adapter_id);
