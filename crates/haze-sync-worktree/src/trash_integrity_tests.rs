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
fn tampered_restore_metadata_fails_record_id_integrity_check() {
    let root = TempRoot::new("tampered-metadata");
    let config = root.config();
    let vault_path = path("note.md");
    let local_path = config.vault_path_to_local(&vault_path).unwrap();
    fs::write(&local_path, b"content").unwrap();
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(
        vault_path.clone(),
        revision("rev_present"),
        content_hash_for_bytes(b"content"),
    );
    let manager = WorktreeTrashManager::new(
        config,
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
    let metadata_path = root.path.join(record.metadata_relative_path());
    let metadata = String::from_utf8(fs::read(&metadata_path).unwrap()).unwrap();
    let tampered = metadata.replace("size=7", "size=8");
    fs::write(metadata_path, tampered).unwrap();

    assert_eq!(
        manager.load_record(&record.record_id).unwrap_err(),
        WorktreeTrashError::InvalidMetadata
    );
}
