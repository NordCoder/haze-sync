use super::*;
use crate::test_support::{connect_required_test_database_from_env, unique_test_id};
use sqlx::{Executor, PgPool};

const BASE_MIGRATIONS: &[&str] = &[
    include_str!("../../../../../../migrations/0001_sync_adapters.sql"),
    include_str!("../../../../../../migrations/0002_content_blobs.sql"),
    include_str!("../../../../../../migrations/0003_sync_objects_file_revisions.sql"),
    include_str!("../../../../../../migrations/0004_operation_log.sql"),
    include_str!("../../../../../../migrations/0005_tombstones_conflicts.sql"),
    include_str!("../../../../../../migrations/0006_cursors_idempotency.sql"),
    include_str!("../../../../../../migrations/0007_gdrive_mapping.sql"),
    include_str!("../../../../../../migrations/0008_worktree_state.sql"),
    include_str!("../../../../../../migrations/0009_audit_events.sql"),
    include_str!("../../../../../../migrations/0010_worktree_durable_state.sql"),
    include_str!("../../../../../../migrations/0011_gdrive_durable_state.sql"),
];
const CONTROL_MIGRATIONS: &[&str] = &[
    include_str!("../../../../../../migrations/0012_operational_control_storage.sql"),
    include_str!("../../../../../../migrations/0013_operational_jobs_audit.sql"),
    include_str!("../../../../../../migrations/0014_gdrive_runtime_authority.sql"),
];

async fn begin_isolated_schema(pool: &PgPool) -> Transaction<'_, Postgres> {
    let mut transaction = pool.begin().await.expect("begin isolated transaction");
    let schema = unique_test_id("control-storage").replace('-', "_");
    (&mut *transaction)
        .execute(format!("create schema {schema}").as_str())
        .await
        .expect("create isolated schema");
    (&mut *transaction)
        .execute(format!("set local search_path to {schema}").as_str())
        .await
        .expect("select isolated schema");
    transaction
}

async fn apply_base(transaction: &mut Transaction<'_, Postgres>) {
    for migration in BASE_MIGRATIONS {
        (&mut **transaction)
            .execute(*migration)
            .await
            .expect("base migration should apply");
    }
}

async fn apply_control(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    for migration in CONTROL_MIGRATIONS {
        (&mut **transaction).execute(*migration).await?;
    }
    Ok(())
}

async fn apply_all(transaction: &mut Transaction<'_, Postgres>) {
    apply_base(transaction).await;
    apply_control(transaction)
        .await
        .expect("control migrations should apply");
}

async fn seed_legacy_and_existing_storage(transaction: &mut Transaction<'_, Postgres>) {
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash, enabled) values \
         ('worktree-a','Worktree A','worktree_adapter',$1,true), \
         ('gdrive-a','GDrive A','gdrive_adapter',$2,false)",
    )
    .bind("1".repeat(64))
    .bind("2".repeat(64))
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into content_blobs (sha256,size_bytes,object_store_path) values ($1,7,'objects/blob')",
    )
    .bind("a".repeat(64))
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into sync_objects (object_id,path,kind,updated_by) values \
         ('object-a','Notes/a.md','file','worktree-a')",
    )
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into file_revisions (revision_id,object_id,path,content_sha256,size_bytes,created_by) \
         values ('revision-a','object-a','Notes/a.md',$1,7,'worktree-a')",
    )
    .bind("a".repeat(64))
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query("update sync_objects set current_revision_id = 'revision-a' where object_id = 'object-a'")
        .execute(&mut **transaction)
        .await
        .unwrap();
    sqlx::query(
        "insert into operation_log (op_id,adapter_id,kind,path,revision_id) \
         values ('operation-a','worktree-a','file_put','Notes/a.md','revision-a')",
    )
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into adapter_cursors (adapter_id,last_core_seq,external_cursor_json) \
         values ('gdrive-a',1,'{\"cursor\":\"opaque\"}'::jsonb)",
    )
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into idempotency_records (adapter_id,idempotency_key,request_hash,response_json) \
         values ('worktree-a','existing-key',$1,'{\"ok\":true}'::jsonb)",
    )
    .bind("b".repeat(64))
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into audit_events (audit_id,actor_adapter_id,event_type,path,revision_id,metadata) \
         values ('audit-a','worktree-a','file.put','Notes/a.md','revision-a','{\"safe\":true}'::jsonb)",
    )
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into worktree_instances (adapter_id,root_fingerprint,state_format_version) \
         values ('worktree-a',$1,1)",
    )
    .bind(format!("sha256:{}", "c".repeat(64)))
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into worktree_state (adapter_id,path,state_kind,last_applied_revision_id, \
         content_sha256,observation_schema_version,observed_size_bytes) \
         values ('worktree-a','Notes/a.md','present','revision-a',$1,1,7)",
    )
    .bind("a".repeat(64))
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query("insert into gdrive_adapter_state (adapter_id) values ('gdrive-a')")
        .execute(&mut **transaction)
        .await
        .unwrap();
    sqlx::query(
        "insert into gdrive_durable_items (adapter_id,path,drive_file_id,core_object_id, \
         core_revision_id,core_seq) values ('gdrive-a','Notes/a.md','drive-file-a', \
         'object-a','revision-a',1)",
    )
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into gdrive_operations (adapter_id,operation_id,operation_kind,facts_hash, \
         outcome_kind,committed_state_version,mapping_path,core_seq) \
         values ('gdrive-a','gdrive-operation-a','import',$1,'committed',1,'Notes/a.md',1)",
    )
    .bind("d".repeat(64))
    .execute(&mut **transaction)
    .await
    .unwrap();
}
