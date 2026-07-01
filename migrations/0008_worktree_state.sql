create table worktree_state (
    path text primary key,
    last_applied_revision_id text references file_revisions(revision_id),
    last_seen_sha256 text,
    last_seen_mtime timestamptz,
    dirty boolean not null default false,
    last_scanned_at timestamptz,
    last_written_by_adapter boolean not null default false
);

comment on table worktree_state is 'Built-in worktree adapter local materialization state. Scanning, watching, echo guard, and repair behavior are implemented in future phases.';

create index worktree_state_dirty_idx on worktree_state(dirty);
create index worktree_state_last_scanned_idx on worktree_state(last_scanned_at);
create index worktree_state_last_applied_revision_idx on worktree_state(last_applied_revision_id);
