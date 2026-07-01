create table sync_adapters (
    adapter_id text primary key,
    display_name text not null,
    role text not null,
    token_hash text not null,
    enabled boolean not null default true,
    created_at timestamptz not null default now(),
    last_seen_at timestamptz
);

comment on table sync_adapters is 'Registered Haze Sync adapters. Core authorization and adapter policy behavior are implemented in future phases.';
comment on column sync_adapters.token_hash is 'Server-side token hash only. Plaintext adapter tokens must never be stored.';

create index sync_adapters_role_idx on sync_adapters(role);
create index sync_adapters_enabled_idx on sync_adapters(enabled);
