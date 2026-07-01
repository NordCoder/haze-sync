create table adapter_cursors (
    adapter_id text primary key references sync_adapters(adapter_id),
    last_core_seq bigint not null default 0,
    external_cursor_json jsonb not null default '{}'::jsonb,
    last_success_at timestamptz,
    updated_at timestamptz not null default now()
);

comment on table adapter_cursors is 'Per-adapter progress cursors. Cursors advance only after successful processing; that behavior is implemented in future phases.';
comment on column adapter_cursors.external_cursor_json is 'Flexible provider cursor metadata such as Google Drive page tokens or scanner watermarks; never store OAuth secrets here.';

create index adapter_cursors_last_core_seq_idx on adapter_cursors(last_core_seq);
create index adapter_cursors_last_success_idx on adapter_cursors(last_success_at);
create index adapter_cursors_updated_at_idx on adapter_cursors(updated_at);

create table idempotency_records (
    adapter_id text not null references sync_adapters(adapter_id),
    idempotency_key text not null,
    request_hash text not null,
    response_json jsonb not null,
    created_at timestamptz not null default now(),
    primary key (adapter_id, idempotency_key)
);

comment on table idempotency_records is 'Retry-safety records for adapter writes. Request comparison and response replay behavior are implemented in future phases.';
comment on column idempotency_records.response_json is 'Safe public response body snapshot only; do not store secrets, raw tokens, or private provider payloads.';

create index idempotency_records_request_hash_idx on idempotency_records(request_hash);
create index idempotency_records_created_at_idx on idempotency_records(created_at);
