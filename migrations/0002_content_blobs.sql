create table content_blobs (
    sha256 text primary key,
    size_bytes bigint not null check (size_bytes >= 0),
    object_store_path text not null,
    created_at timestamptz not null default now()
);

comment on table content_blobs is 'Immutable content-addressed blob metadata keyed by SHA-256. Blob write and verification behavior are implemented in future phases.';
comment on column content_blobs.object_store_path is 'Relative or configured object-store path for the immutable blob bytes; not a user-supplied vault path.';

create index content_blobs_created_at_idx on content_blobs(created_at);
