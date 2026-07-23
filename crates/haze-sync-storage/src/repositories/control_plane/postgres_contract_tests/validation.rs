#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL and schema-validation evidence"]
async fn current_schema_validation_components_are_exact() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;

    let table_names = crate::schema::table_names::ALL
        .iter()
        .map(|table| (*table).to_owned())
        .collect::<Vec<_>>();
    let mut actual = sqlx::query_scalar::<_, String>(
        "select table_name from information_schema.tables where table_schema = current_schema() \
         and table_type = 'BASE TABLE' and table_name = any($1) order by table_name",
    )
    .bind(&table_names)
    .fetch_all(&mut *transaction)
    .await
    .unwrap();
    let mut expected = table_names;
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected, "current owned table set differs");

    for (table, expected_columns) in [
        (
            "worktree_instances",
            &[
                "adapter_id",
                "root_fingerprint",
                "state_format_version",
                "created_at",
                "updated_at",
            ][..],
        ),
        (
            "worktree_state",
            &[
                "adapter_id",
                "path",
                "state_kind",
                "state_format_version",
                "last_applied_revision_id",
                "content_sha256",
                "observation_schema_version",
                "observed_size_bytes",
                "observed_mtime",
                "created_at",
                "updated_at",
            ][..],
        ),
        (
            "gdrive_adapter_state",
            &[
                "adapter_id",
                "state_format_version",
                "state_version",
                "drive_cursor",
                "drive_cursor_generation",
                "core_export_seq",
                "last_import_operation_id",
                "last_export_operation_id",
                "last_provider_mutation_operation_id",
                "created_at",
                "updated_at",
            ][..],
        ),
        (
            "gdrive_durable_items",
            &[
                "adapter_id",
                "path",
                "drive_file_id",
                "drive_parent_id",
                "drive_name",
                "mime_type",
                "md5_checksum",
                "head_revision_id",
                "drive_version",
                "drive_modified_time",
                "core_object_id",
                "core_revision_id",
                "core_seq",
                "echo_state",
                "echo_operation_id",
                "echo_provider_version",
                "delete_candidate_first_seen_at",
                "delete_candidate_last_seen_at",
                "delete_candidate_generation",
                "delete_candidate_blocked",
                "delete_confirmation_audit_id",
                "last_imported_at",
                "last_exported_at",
                "last_seen_at",
                "created_at",
                "updated_at",
            ][..],
        ),
        (
            "gdrive_operations",
            &[
                "adapter_id",
                "operation_id",
                "operation_kind",
                "facts_hash",
                "outcome_kind",
                "committed_state_version",
                "mapping_path",
                "core_seq",
                "drive_version",
                "created_at",
            ][..],
        ),
    ] {
        let actual_columns = sqlx::query_scalar::<_, String>(
            "select column_name from information_schema.columns where table_schema = current_schema() \
             and table_name = $1 order by ordinal_position",
        )
        .bind(table)
        .fetch_all(&mut *transaction)
        .await
        .unwrap();
        assert_eq!(
            actual_columns.iter().map(String::as_str).collect::<Vec<_>>(),
            expected_columns,
            "accepted Stage 10 columns differ for {table}",
        );
    }

    for constraint in [
        "adapter_cursors_last_core_seq_nonnegative",
        "gdrive_adapter_state_version_nonnegative",
        "gdrive_adapter_cursor_generation_nonnegative",
        "gdrive_adapter_core_export_seq_nonnegative",
        "gdrive_durable_items_echo_consistent",
        "gdrive_durable_items_delete_candidate_consistent",
    ] {
        let exists: bool = sqlx::query_scalar(
            "select exists (select 1 from pg_constraint \
             where connamespace = current_schema()::regnamespace and conname = $1)",
        )
        .bind(constraint)
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
        assert!(exists, "accepted Stage 10 constraint is missing: {constraint}");
    }

    let maintenance_count: i64 = sqlx::query_scalar(
        "select count(*)::bigint from maintenance_control where singleton_id = 1",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    let inventory_count: i64 = sqlx::query_scalar(
        "select count(*)::bigint from adapter_inventory_state where singleton_id = 1",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    let global_slot_count: i64 = sqlx::query_scalar(
        "select count(*)::bigint from operational_execution_slots \
         where slot_id = 'global-destructive' and slot_kind = 'global_destructive'",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(
        (maintenance_count, inventory_count, global_slot_count),
        (1, 1, 1),
        "required control singleton rows differ",
    );
    transaction.rollback().await.unwrap();
}
