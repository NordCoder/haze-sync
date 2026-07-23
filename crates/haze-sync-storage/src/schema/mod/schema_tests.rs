#[cfg(test)]
mod tests {
    use super::{control_plane, table_names, BASE_MIGRATIONS, INITIAL_MIGRATIONS};

    macro_rules! migration_content {
        ($name:literal) => {
            (
                $name,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../migrations/",
                    $name
                )),
            )
        };
    }

    const MIGRATION_CONTENTS: &[(&str, &str)] = &[
        migration_content!("0001_sync_adapters.sql"),
        migration_content!("0002_content_blobs.sql"),
        migration_content!("0003_sync_objects_file_revisions.sql"),
        migration_content!("0004_operation_log.sql"),
        migration_content!("0005_tombstones_conflicts.sql"),
        migration_content!("0006_cursors_idempotency.sql"),
        migration_content!("0007_gdrive_mapping.sql"),
        migration_content!("0008_worktree_state.sql"),
        migration_content!("0009_audit_events.sql"),
        migration_content!("0010_worktree_durable_state.sql"),
        migration_content!("0011_gdrive_durable_state.sql"),
        migration_content!("0012_operational_control_storage.sql"),
        migration_content!("0013_operational_jobs_audit.sql"),
        migration_content!("0014_gdrive_runtime_authority.sql"),
    ];

    #[test]
    fn migration_metadata_matches_actual_files() {
        assert_eq!(INITIAL_MIGRATIONS.len(), MIGRATION_CONTENTS.len());
        for (metadata_name, (actual_name, contents)) in
            INITIAL_MIGRATIONS.iter().zip(MIGRATION_CONTENTS)
        {
            assert_eq!(metadata_name, actual_name);
            assert!(contents.contains("create table") || contents.contains("alter table"));
        }
    }

    #[test]
    fn base_registry_stops_at_the_authorized_stage10_head() {
        assert_eq!(
            BASE_MIGRATIONS.last(),
            Some(&"0011_gdrive_durable_state.sql")
        );
        assert_eq!(
            INITIAL_MIGRATIONS.last(),
            Some(&control_plane::CURRENT_MIGRATION_HEAD)
        );
        assert_eq!(INITIAL_MIGRATIONS.len(), BASE_MIGRATIONS.len() + 3);
    }

    #[test]
    fn accepted_pre_control_table_set_excludes_only_stage11_tables() {
        for table in control_plane::table_names::ALL {
            assert!(!table_names::PRE_CONTROL_P12.contains(table));
            assert!(table_names::ALL.contains(table));
        }
    }

    #[test]
    fn migration_metadata_is_strictly_ordered() {
        for pair in INITIAL_MIGRATIONS.windows(2) {
            let previous = migration_prefix(pair[0]);
            let next = migration_prefix(pair[1]);
            assert!(
                previous < next,
                "migration filenames must be strictly ordered"
            );
        }
    }

    #[test]
    fn table_name_metadata_matches_final_storage_schema() {
        let mut expected = table_names::ALL.to_vec();
        let mut actual = final_created_tables();
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(expected, actual);
    }

    #[test]
    fn accepted_pre_gdrive_table_set_excludes_only_0011_tables() {
        for table in [
            table_names::GDRIVE_ADAPTER_STATE,
            table_names::GDRIVE_DURABLE_ITEMS,
            table_names::GDRIVE_OPERATIONS,
        ] {
            assert!(!table_names::PRE_STOR_GDA_P11.contains(&table));
            assert!(table_names::ALL.contains(&table));
        }
    }

    fn final_created_tables() -> Vec<&'static str> {
        let mut tables = Vec::new();
        for (_, contents) in MIGRATION_CONTENTS {
            for line in contents.lines().map(str::trim) {
                if let Some(table_name) = line
                    .strip_prefix("drop table ")
                    .and_then(|rest| rest.split_whitespace().next())
                {
                    tables.retain(|candidate| *candidate != table_name.trim_end_matches(';'));
                }
                if let Some(table_name) = line
                    .strip_prefix("create table ")
                    .and_then(|rest| rest.split_whitespace().next())
                {
                    let table_name = table_name.trim_end_matches(';');
                    if !tables.contains(&table_name) {
                        tables.push(table_name);
                    }
                }
            }
        }
        tables
    }

    fn migration_prefix(filename: &str) -> u32 {
        filename
            .split_once('_')
            .and_then(|(prefix, _)| prefix.parse().ok())
            .expect("migration filename must start with a numeric prefix")
    }
}
