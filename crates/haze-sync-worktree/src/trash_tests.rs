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
            "haze-sync-trash-{name}-{}-{id}",
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
        Err(error) => panic!("failed to clear trash test root: {error}"),
    }
}

fn path(input: &str) -> VaultPath {
    VaultPath::parse(input).unwrap()
}

fn revision(input: &str) -> RevisionId {
    RevisionId::parse(input).unwrap()
}

#[test]
fn tombstone_moves_file_and_persists_restore_ready_metadata() {
    let root = TempRoot::new("move-and-metadata");
    let config = root.config();
    let vault_path = path("Notes/a=b.md");
    let local_path = config.vault_path_to_local(&vault_path).unwrap();
    fs::create_dir_all(local_path.parent().unwrap()).unwrap();
    let bytes = b"retained content";
    fs::write(&local_path, bytes).unwrap();
    let original_hash = content_hash_for_bytes(bytes);
    let original_revision = revision("rev_before_delete");
    let tombstone_revision = revision("rev_tombstone");
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(
        vault_path.clone(),
        original_revision,
        original_hash,
    );
    let retained_at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    let policy = WorktreeTrashPolicy::new(Duration::from_secs(30 * 24 * 60 * 60)).unwrap();
    let manager = WorktreeTrashManager::new(config.clone(), policy);

    let outcome = manager
        .materialize_tombstone(
            &mut state,
            WorktreeTombstoneMaterializationRequest {
                vault_path: vault_path.clone(),
                tombstone_revision_id: tombstone_revision.clone(),
                tombstoned_at: retained_at,
            },
        )
        .unwrap();
    let WorktreeTombstoneMaterializationOutcome::Trashed(record) = outcome else {
        panic!("expected retained trash record");
    };

    assert!(!local_path.exists());
    let retained_path = root.path.join(record.trash_relative_path());
    assert_eq!(fs::read(&retained_path).unwrap(), bytes);
    assert_eq!(record.content_hash, original_hash);
    assert_eq!(record.size, bytes.len() as u64);
    assert_eq!(record.retained_at, retained_at);
    assert_eq!(
        record.retention_until,
        retained_at + Duration::from_secs(30 * 24 * 60 * 60)
    );
    assert!(record.is_retained_at(retained_at + Duration::from_secs(1)));
    assert!(!record.is_retained_at(record.retention_until));

    let loaded = manager.load_record(&record.record_id).unwrap();
    assert_eq!(loaded, record);
    let metadata_path = root.path.join(record.metadata_relative_path());
    let metadata = String::from_utf8(fs::read(metadata_path).unwrap()).unwrap();
    assert!(!metadata.contains(root.path.to_str().unwrap()));
    assert!(!metadata.contains("path=Notes/a=b.md"));

    assert!(matches!(
        state.path_state(&vault_path),
        Some(WorktreeAppliedPathState::Tombstoned(tombstone))
            if tombstone.revision_id == tombstone_revision
    ));

    let after_retention = record.retention_until + Duration::from_secs(24 * 60 * 60);
    assert!(!record.is_retained_at(after_retention));
    assert_eq!(fs::read(retained_path).unwrap(), bytes);
}

#[test]
fn already_absent_target_advances_tombstone_without_creating_trash_record() {
    let root = TempRoot::new("already-absent");
    let config = root.config();
    let vault_path = path("missing.md");
    let tombstone_revision = revision("rev_missing_tombstone");
    let retained_at = SystemTime::UNIX_EPOCH + Duration::from_secs(500);
    let policy = WorktreeTrashPolicy::new(Duration::from_secs(60)).unwrap();
    let manager = WorktreeTrashManager::new(config.clone(), policy);
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(
        vault_path.clone(),
        revision("rev_missing_before"),
        content_hash_for_bytes(b"missing"),
    );

    let outcome = manager
        .materialize_tombstone(
            &mut state,
            WorktreeTombstoneMaterializationRequest {
                vault_path: vault_path.clone(),
                tombstone_revision_id: tombstone_revision.clone(),
                tombstoned_at: retained_at,
            },
        )
        .unwrap();

    assert_eq!(
        outcome,
        WorktreeTombstoneMaterializationOutcome::AlreadyAbsent {
            vault_path: vault_path.clone(),
            tombstone_revision_id: tombstone_revision.clone(),
            retention_until: retained_at + Duration::from_secs(60),
        }
    );
    assert!(!config.runtime_dir().exists());
    assert!(matches!(
        state.path_state(&vault_path),
        Some(WorktreeAppliedPathState::Tombstoned(tombstone))
            if tombstone.revision_id == tombstone_revision
    ));
}

#[test]
fn invalid_retention_and_invalid_record_ids_are_rejected() {
    assert_eq!(
        WorktreeTrashPolicy::new(Duration::ZERO).unwrap_err(),
        WorktreeTrashError::InvalidRetention
    );
    assert_eq!(
        WorktreeTrashRecordId::parse("not-a-record").unwrap_err(),
        WorktreeTrashError::InvalidMetadata
    );
}

#[cfg(unix)]
#[test]
fn symlink_target_and_symlinked_trash_directory_are_rejected_without_touching_outside_files() {
    use std::os::unix::fs::symlink;

    let root = TempRoot::new("symlink-safety");
    let config = root.config();
    let outside = TempRoot::new("symlink-outside");
    let outside_file = outside.path.join("outside.md");
    fs::write(&outside_file, b"outside").unwrap();
    let vault_path = path("linked.md");
    let local_path = config.vault_path_to_local(&vault_path).unwrap();
    symlink(&outside_file, &local_path).unwrap();
    let policy = WorktreeTrashPolicy::new(Duration::from_secs(60)).unwrap();
    let manager = WorktreeTrashManager::new(config.clone(), policy);
    let mut state = WorktreeStateSnapshot::new();

    let error = manager
        .materialize_tombstone(
            &mut state,
            WorktreeTombstoneMaterializationRequest {
                vault_path: vault_path.clone(),
                tombstone_revision_id: revision("rev_symlink"),
                tombstoned_at: SystemTime::UNIX_EPOCH + Duration::from_secs(10),
            },
        )
        .unwrap_err();
    assert_eq!(
        error,
        WorktreeTrashError::TargetNotRegularFile {
            vault_path: vault_path.clone()
        }
    );
    assert_eq!(fs::read(&outside_file).unwrap(), b"outside");
    fs::remove_file(&local_path).unwrap();

    fs::create_dir(config.runtime_dir()).unwrap();
    symlink(&outside.path, config.trash_dir()).unwrap();
    fs::write(&local_path, b"local").unwrap();
    let error = manager
        .materialize_tombstone(
            &mut state,
            WorktreeTombstoneMaterializationRequest {
                vault_path: vault_path.clone(),
                tombstone_revision_id: revision("rev_unsafe_trash"),
                tombstoned_at: SystemTime::UNIX_EPOCH + Duration::from_secs(20),
            },
        )
        .unwrap_err();
    assert_eq!(error, WorktreeTrashError::UnsafeRuntimeDirectory);
    assert_eq!(fs::read(&local_path).unwrap(), b"local");
    assert_eq!(fs::read(&outside_file).unwrap(), b"outside");
}
