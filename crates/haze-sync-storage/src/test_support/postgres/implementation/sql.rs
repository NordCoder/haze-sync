fn same_table_set(actual: &[String], expected: &[&str]) -> bool {
    let mut actual = actual.iter().map(String::as_str).collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    actual.sort_unstable();
    expected.sort_unstable();
    actual == expected
}

fn owned_storage_table_names_sql() -> String {
    let names = table_names::ALL
        .iter()
        .map(|table_name| format!("'{table_name}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "select table_name from information_schema.tables \
         where table_schema = current_schema() \
           and table_type = 'BASE TABLE' \
           and table_name in ({names}) \
         order by table_name"
    )
}

fn clean_storage_tables_sql() -> String {
    format!(
        "truncate table {tables} restart identity cascade; \
         insert into maintenance_control (singleton_id) values (1); \
         insert into adapter_inventory_state (singleton_id) values (1); \
         insert into operational_execution_slots (slot_id, slot_kind) \
         values ('global-destructive', 'global_destructive')",
        tables = table_names::ALL.join(", "),
    )
}
