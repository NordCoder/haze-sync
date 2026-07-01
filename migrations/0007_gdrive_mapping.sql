create table gdrive_mapping (
    path text primary key,
    drive_file_id text unique,
    drive_parent_id text,
    drive_name text,
    mime_type text,
    md5_checksum text,
    head_revision_id text,
    drive_version text,
    drive_modified_time timestamptz,
    core_revision_id text references file_revisions(revision_id),
    core_seq bigint,
    last_imported_at timestamptz,
    last_exported_at timestamptz,
    last_seen_at timestamptz,
    delete_candidate_at timestamptz
);

comment on table gdrive_mapping is 'Google Drive file ID and metadata mapping for vault paths. Drive API fetching, echo guards, and delete-candidate policy are implemented in future phases.';
comment on column gdrive_mapping.drive_file_id is 'External Google Drive file identifier. This is provider metadata, not a credential.';

create index gdrive_mapping_file_id_idx on gdrive_mapping(drive_file_id);
create index gdrive_mapping_parent_idx on gdrive_mapping(drive_parent_id);
create index gdrive_mapping_core_revision_idx on gdrive_mapping(core_revision_id);
create index gdrive_mapping_core_seq_idx on gdrive_mapping(core_seq);
create index gdrive_mapping_delete_candidate_idx on gdrive_mapping(delete_candidate_at);
