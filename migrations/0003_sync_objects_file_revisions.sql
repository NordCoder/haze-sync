create table sync_objects (
    object_id text primary key,
    path text not null unique,
    kind text not null,
    current_revision_id text,
    deleted_at timestamptz,
    updated_at timestamptz not null default now(),
    updated_by text not null references sync_adapters(adapter_id)
);

comment on table sync_objects is 'Logical vault objects. V1 uses path as the practical identity while retaining object_id for future move support; Core apply policy is implemented in future phases.';

create index sync_objects_path_idx on sync_objects(path);
create index sync_objects_deleted_idx on sync_objects(deleted_at);
create index sync_objects_updated_at_idx on sync_objects(updated_at);

create table file_revisions (
    revision_id text primary key,
    object_id text not null references sync_objects(object_id),
    path text not null,
    parent_revision_id text references file_revisions(revision_id),
    content_sha256 text not null references content_blobs(sha256),
    size_bytes bigint not null check (size_bytes >= 0),
    created_by text not null references sync_adapters(adapter_id),
    created_at timestamptz not null default now()
);

comment on table file_revisions is 'Immutable file revisions. Conflict, delete, and current-revision policies are implemented by future Core phases.';

create index file_revisions_object_idx on file_revisions(object_id);
create index file_revisions_path_idx on file_revisions(path);
create index file_revisions_content_sha256_idx on file_revisions(content_sha256);
create index file_revisions_created_at_idx on file_revisions(created_at);

alter table sync_objects
    add constraint sync_objects_current_revision_fk
    foreign key (current_revision_id) references file_revisions(revision_id);
