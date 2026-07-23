#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL and schema-validation evidence"]
async fn current_schema_validation_components_are_exact() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;

    let mut actual = owned_storage_table_names(&mut transaction).await.unwrap();
    let mut expected = crate::schema::table_names::ALL
        .iter()
        .map(|table| (*table).to_owned())
        .collect::<Vec<_>>();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected, "current owned table set differs");

    validate_stage10_schema(&mut transaction)
        .await
        .expect("accepted Stage 10 schema must remain compatible");

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

    validate_current_schema(&mut transaction)
        .await
        .expect("composed current schema validation must pass");
    transaction.rollback().await.unwrap();
}
