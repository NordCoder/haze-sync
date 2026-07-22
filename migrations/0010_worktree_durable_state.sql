do $$
begin
    if exists (select 1 from worktree_state limit 1) then
        raise exception 'legacy worktree_state rows require explicit operator migration';
    end if;
end
$$;

drop table worktree_state;

create table worktree_instances (
    adapter_id text primary key references sync_adapters(adapter_id),
    root_fingerprint text not null
        check (root_fingerprint ~ '^sha256:[0-9a-f]{64}$'),
    state_format_version integer not null
        check (state_format_version = 1),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

comment on table worktree_instances is 'Versioned durable binding between a Worktree adapter identity and a non-public normalized-root fingerprint.';
comment on column worktree_instances.root_fingerprint is 'SHA-256 fingerprint of an already-normalized configured root. Raw local roots must never be persisted here.';

create table worktree_state (
    adapter_id text not null references worktree_instances(adapter_id),
    path text not null,
    state_kind text not null
        check (state_kind in ('present', 'tombstoned')),
    state_format_version integer not null default 1
        check (state_format_version = 1),
    last_applied_revision_id text not null references file_revisions(revision_id),
    content_sha256 text references content_blobs(sha256),
    observation_schema_version integer,
    observed_size_bytes bigint,
    observed_mtime timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    primary key (adapter_id, path),
    check (
        (state_kind = 'present' and content_sha256 is not null)
        or (state_kind = 'tombstoned' and content_sha256 is null)
    ),
    check (
        (
            observation_schema_version is null
            and observed_size_bytes is null
            and observed_mtime is null
        )
        or (
            state_kind = 'present'
            and observation_schema_version = 1
            and observed_size_bytes is not null
            and observed_size_bytes >= 0
        )
    )
);

comment on table worktree_state is 'Per-instance last-applied Worktree path state. Worktree owns filesystem interpretation; Server owns transaction timing.';
comment on column worktree_state.observation_schema_version is 'Version of bounded reconciliation observation fields. Null means no clean observation is stored.';

create index worktree_state_kind_idx
    on worktree_state(adapter_id, state_kind, path);
create index worktree_state_revision_idx
    on worktree_state(last_applied_revision_id);
create index worktree_state_content_idx
    on worktree_state(content_sha256);

alter table adapter_cursors
    add constraint adapter_cursors_last_core_seq_nonnegative
    check (last_core_seq >= 0);
