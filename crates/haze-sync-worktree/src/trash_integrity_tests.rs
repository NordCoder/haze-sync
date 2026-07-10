use crate::hashing::content_hash_for_bytes;
use crate::*;
use haze_sync_common::{RevisionId, VaultPath};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

static NEXT_TEST_ROOT_ID: AtomicU64 = AtomicU64::new(0);

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let id = NEXT_TEST_ROOT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "haze-sync-trash-integrity-{name}-{}-{id}",
            std::process::id()
        ));
        remove_dir_if_exists(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn config(&self) -> WorktreeConfig {
        WorktreeConfig::new(self.path.clone()).unwrap()
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        remove_dir_if_exists(&self.path);
    }
}

fn remove_dir_if_exists(path: &Path) {
    match fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => panic!("failed to clear trash-integrity root: {error}"),
    }
}

fn path(input: &str) -> VaultPath {
    VaultPath::parse(input).unwrap()
}

fn revision(input: &str) -> RevisionId {
    RevisionId::parse(input).unwrap()
}

fn materialize_record(
    root: &TempRoot,
    name: &str,
    bytes: &[u8],
) -> (WorktreeConfig, WorktreeTrashManager, WorktreeTrashRecord) {
    let config = root.config();
    let vault_path = path(name);
    let local_path = config.vault_path_to_local(&vault_path).unwrap();
    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&local_path, bytes).unwrap();
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(
        vault_path.clone(),
        revision("rev_present"),
        content_hash_for_bytes(bytes),
    );
    let manager = WorktreeTrashManager::new(
        config.clone(),
        WorktreeTrashPolicy::new(Duration::from_secs(60)).unwrap(),
    );
    let outcome = manager
        .materialize_tombstone(
            &mut state,
            WorktreeTombstoneMaterializationRequest {
                vault_path,
                tombstone_revision_id: revision("rev_tombstone"),
                tombstoned_at: SystemTime::UNIX_EPOCH + Duration::from_secs(20),
            },
        )
        .unwrap();
    let WorktreeTombstoneMaterializationOutcome::Trashed(record) = outcome else {
        panic!("expected trash record");
    };
    (config, manager, record)
}

#[test]
fn unavailable_root_is_not_misclassified_as_already_absent() {
    let root = TempRoot::new("missing-root");
    let config = root.config();
    let manager = WorktreeTrashManager::new(
        config,
        WorktreeTrashPolicy::new(Duration::from_secs(60)).unwrap(),
    );
    let vault_path = path("missing.md");
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(
        vault_path.clone(),
        revision("rev_present"),
        content_hash_for_bytes(b"present"),
    );
    fs::remove_dir_all(&root.path).unwrap();

    let error = manager
        .materialize_tombstone(
            &mut state,
            WorktreeTombstoneMaterializationRequest {
                vault_path: vault_path.clone(),
                tombstone_revision_id: revision("rev_tombstone"),
                tombstoned_at: SystemTime::UNIX_EPOCH + Duration::from_secs(10),
            },
        )
        .unwrap_err();

    assert_eq!(error, WorktreeTrashError::RootUnavailable);
    assert!(matches!(
        state.path_state(&vault_path),
        Some(WorktreeAppliedPathState::Present(applied))
            if applied.revision_id == revision("rev_present")
    ));
}

#[test]
fn tampered_restore_metadata_fields_fail_record_id_integrity_check() {
    let root = TempRoot::new("tampered-metadata");
    let (_, manager, record) = materialize_record(&root, "note.md", b"content");
    let metadata_path = root.path.join(record.metadata_relative_path());
    let metadata = String::from_utf8(fs::read(&metadata_path).unwrap()).unwrap();

    fs::write(&metadata_path, metadata.replace("size=7", "size=8")).unwrap();
    assert_eq!(
        manager.load_record(&record.record_id).unwrap_err(),
        WorktreeTrashError::InvalidMetadata
    );

    fs::write(
        &metadata_path,
        metadata.replace("retention_until_unix_seconds=80", "retention_until_unix_seconds=81"),
    )
    .unwrap();
    assert_eq!(
        manager.load_record(&record.record_id).unwrap_err(),
        WorktreeTrashError::InvalidMetadata
    );
}

#[test]
fn missing_or_changed_retained_bytes_are_rejected() {
    let root = TempRoot::new("retained-bytes");
    let (_, manager, record) = materialize_record(&root, "note.md", b"content");
    let retained_path = root.path.join(record.trash_relative_path());

    fs::write(&retained_path, b"contEnt").unwrap();
    assert_eq!(
        manager.load_record(&record.record_id).unwrap_err(),
        WorktreeTrashError::RetainedFileMismatch {
            vault_path: path("note.md")
        }
    );

    fs::remove_file(&retained_path).unwrap();
    assert_eq!(
        manager.load_record(&record.record_id).unwrap_err(),
        WorktreeTrashError::RetainedFileMissing {
            vault_path: path("note.md")
        }
    );
}

#[test]
fn extreme_metadata_timestamps_fail_without_panicking() {
    let root = TempRoot::new("extreme-time");
    let config = root.config();
    let metadata_dir = config.metadata_dir().join("trash");
    fs::create_dir_all(&metadata_dir).unwrap();
    let record_id = WorktreeTrashRecordId::parse(&"0".repeat(64)).unwrap();
    let metadata_path = metadata_dir.join(format!("{}.meta", record_id.as_str()));
    fs::write(
        metadata_path,
        format!(
            "version=1\nrecord_id={}\npath_hex=6e6f74652e6d64\ntombstone_revision_hex=7265765f746f6d6273746f6e65\ncontent_hash={}\nsize=0\nretained_at_unix_seconds={}\nretention_until_unix_seconds={}\n",
            record_id.as_str(),
            "0".repeat(64),
            u64::MAX - 1,
            u64::MAX,
        ),
    )
    .unwrap();
    let manager = WorktreeTrashManager::new(
        config,
        WorktreeTrashPolicy::new(Duration::from_secs(60)).unwrap(),
    );

    assert_eq!(
        manager.load_record(&record_id).unwrap_err(),
        WorktreeTrashError::InvalidMetadata
    );
}

#[cfg(unix)]
#[test]
fn symlinked_metadata_parent_is_rejected_before_file_open() {
    use std::os::unix::fs::symlink;

    let root = TempRoot::new("metadata-parent");
    let outside = TempRoot::new("metadata-outside");
    let (config, manager, record) = materialize_record(&root, "note.md", b"content");
    let metadata_dir = config.metadata_dir().join("trash");
    let metadata_name = format!("{}.meta", record.record_id.as_str());
    let original_metadata = fs::read(metadata_dir.join(&metadata_name)).unwrap();
    fs::write(outside.path.join(&metadata_name), original_metadata).unwrap();
    fs::remove_dir_all(&metadata_dir).unwrap();
    symlink(&outside.path, &metadata_dir).unwrap();

    assert_eq!(
        manager.load_record(&record.record_id).unwrap_err(),
        WorktreeTrashError::UnsafeRuntimeDirectory
    );
}
