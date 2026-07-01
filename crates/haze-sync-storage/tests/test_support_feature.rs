#![cfg(feature = "test-support")]

use haze_sync_storage::test_support::{
    TestDatabaseUrl, TestDatabaseUrlSource, TestNamespace, STORAGE_TEST_MIGRATIONS,
};

#[test]
fn test_support_feature_exports_safe_non_db_helpers_and_embedded_migrations() {
    let namespace = TestNamespace::new("fan-in");
    let path = namespace.vault_path("daily note.md");
    let url = TestDatabaseUrl::parse_explicit(
        TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
        "postgres://haze_sync:placeholder@localhost:5432/haze_sync_fanin_test",
    )
    .expect("safe test database URL should parse");

    assert!(namespace.id().contains("fan-in"));
    assert!(path.starts_with("_haze_tests/"));
    assert_eq!(url.database_name(), "haze_sync_fanin_test");
    assert!(url.redacted().contains("<redacted>"));
    assert_eq!(STORAGE_TEST_MIGRATIONS.len(), 9);
    assert_eq!(STORAGE_TEST_MIGRATIONS[0].name, "0001_sync_adapters.sql");
}
