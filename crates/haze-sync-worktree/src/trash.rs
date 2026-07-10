//! Reversible materialization of authoritative Core tombstones.
//!
//! User files are moved into Worktree-owned retained trash. A durable,
//! path-redacted metadata record is committed before local last-applied state is
//! advanced. This module never performs retention cleanup or hard-deletes user
//! content.

use crate::hashing::content_hash_for_bytes;
use crate::{WorktreeConfig, WorktreePathError, WorktreeStateSnapshot};
use haze_sync_common::{ContentHash, RevisionId, VaultPath};
use std::fmt;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TRASH_RECORDS_DIR_NAME: &str = "records";
const TRASH_METADATA_DIR_NAME: &str = "trash";
const TRASH_METADATA_SUFFIX: &str = ".meta";
const TRASH_METADATA_VERSION: &str = "1";
const MAX_TRASH_METADATA_BYTES: u64 = 16 * 1024;
const METADATA_TEMP_PREFIX: &str = ".trash-metadata-";
const METADATA_TEMP_SUFFIX: &str = ".tmp";

static NEXT_METADATA_TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// Local retention policy applied to newly materialized tombstones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeTrashPolicy {
    retention: Duration,
}

impl WorktreeTrashPolicy {
    /// Construct a whole-second, non-zero local retention policy.
    pub fn new(retention: Duration) -> Result<Self, WorktreeTrashError> {
        if retention.is_zero() || retention.subsec_nanos() != 0 {
            return Err(WorktreeTrashError::InvalidRetention);
        }
        Ok(Self { retention })
    }

    /// Duration for which moved local content must remain retained.
    #[must_use]
    pub const fn retention(self) -> Duration {
        self.retention
    }
}

/// Authoritative Core tombstone to materialize locally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeTombstoneMaterializationRequest {
    /// Vault-relative path to remove from the materialized view.
    pub vault_path: VaultPath,
    /// Authoritative Core tombstone revision.
    pub tombstone_revision_id: RevisionId,
    /// Time at which the tombstone was accepted/applied.
    pub tombstoned_at: SystemTime,
}

/// Stable identifier for one durable local trash record.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorktreeTrashRecordId(String);

impl WorktreeTrashRecordId {
    /// Parse a 64-character lowercase hexadecimal record identifier.
    pub fn parse(input: &str) -> Result<Self, WorktreeTrashError> {
        if input.len() != 64
            || !input
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(WorktreeTrashError::InvalidMetadata);
        }
        Ok(Self(input.to_owned()))
    }

    /// Borrow the record identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Durable restore-ready facts for one retained local file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeTrashRecord {
    /// Stable record identifier used by runtime paths.
    pub record_id: WorktreeTrashRecordId,
    /// Original vault-relative path.
    pub vault_path: VaultPath,
    /// Authoritative tombstone revision that triggered the move.
    pub tombstone_revision_id: RevisionId,
    /// Hash of retained file bytes.
    pub content_hash: ContentHash,
    /// Retained file byte length.
    pub size: u64,
    /// Local retention start time, normalized to whole Unix seconds.
    pub retained_at: SystemTime,
    /// Earliest time at which a separate maintenance policy may consider cleanup.
    pub retention_until: SystemTime,
}

impl WorktreeTrashRecord {
    /// Safe worktree-relative location of the retained file.
    #[must_use]
    pub fn trash_relative_path(&self) -> String {
        format!(
            "_haze_runtime/trash/{}/{}/{}",
            TRASH_RECORDS_DIR_NAME,
            self.record_id.as_str(),
            self.vault_path
        )
    }

    /// Safe worktree-relative location of the durable metadata record.
    #[must_use]
    pub fn metadata_relative_path(&self) -> String {
        format!(
            "_haze_runtime/metadata/{}/{}{}",
            TRASH_METADATA_DIR_NAME,
            self.record_id.as_str(),
            TRASH_METADATA_SUFFIX
        )
    }

    /// True while the record remains inside its required retention window.
    #[must_use]
    pub fn is_retained_at(&self, now: SystemTime) -> bool {
        now < self.retention_until
    }
}

/// Local outcome after applying an authoritative tombstone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeTombstoneMaterializationOutcome {
    /// A present regular file was moved into retained trash.
    Trashed(WorktreeTrashRecord),
    /// The materialized view was already missing the path.
    AlreadyAbsent {
        /// Vault-relative absent path.
        vault_path: VaultPath,
        /// Authoritative tombstone revision recorded locally.
        tombstone_revision_id: RevisionId,
        /// Retention boundary that would have applied to retained content.
        retention_until: SystemTime,
    },
}

/// Safe failures from local trash/retention handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeTrashError {
    /// Retention duration was zero or not representable by the metadata format.
    InvalidRetention,
    /// Tombstone time could not be represented safely.
    InvalidTimestamp,
    /// Vault path could not be mapped safely.
    Path(WorktreePathError),
    /// Configured worktree root was missing or not a safe directory.
    RootUnavailable,
    /// Reserved runtime/trash/metadata directory chain was unsafe.
    UnsafeRuntimeDirectory,
    /// Required Worktree-owned directory could not be created.
    RuntimeDirectoryCreateFailed,
    /// Local target was a symlink, directory, or special file.
    TargetNotRegularFile { vault_path: VaultPath },
    /// Local target bytes could not be read safely.
    TargetReadFailed { vault_path: VaultPath },
    /// Local target changed between observation and move.
    TargetChangedBeforeMove { vault_path: VaultPath },
    /// A retained destination or metadata record already existed unexpectedly.
    TrashDestinationExists { vault_path: VaultPath },
    /// Durable metadata could not be staged or committed.
    MetadataWriteFailed { vault_path: VaultPath },
    /// Durable metadata could not be read.
    MetadataReadFailed,
    /// Durable metadata was malformed, inconsistent, or tampered with.
    InvalidMetadata,
    /// The retained file referenced by metadata was missing.
    RetainedFileMissing { vault_path: VaultPath },
    /// Retained bytes did not match the restore metadata hash or size.
    RetainedFileMismatch { vault_path: VaultPath },
    /// User content could not be moved into retained trash.
    MoveFailed { vault_path: VaultPath },
    /// A failed commit could not restore the original local file.
    RollbackFailed { vault_path: VaultPath },
}

impl WorktreeTrashError {
    /// Stable machine-readable code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidRetention => "invalid_trash_retention",
            Self::InvalidTimestamp => "invalid_trash_timestamp",
            Self::Path(error) => error.code(),
            Self::RootUnavailable => "worktree_root_unavailable",
            Self::UnsafeRuntimeDirectory => "unsafe_trash_runtime_directory",
            Self::RuntimeDirectoryCreateFailed => "trash_runtime_directory_create_failed",
            Self::TargetNotRegularFile { .. } => "trash_target_not_regular_file",
            Self::TargetReadFailed { .. } => "trash_target_read_failed",
            Self::TargetChangedBeforeMove { .. } => "trash_target_changed_before_move",
            Self::TrashDestinationExists { .. } => "trash_destination_exists",
            Self::MetadataWriteFailed { .. } => "trash_metadata_write_failed",
            Self::MetadataReadFailed => "trash_metadata_read_failed",
            Self::InvalidMetadata => "invalid_trash_metadata",
            Self::RetainedFileMissing { .. } => "retained_trash_file_missing",
            Self::RetainedFileMismatch { .. } => "retained_trash_file_mismatch",
            Self::MoveFailed { .. } => "trash_move_failed",
            Self::RollbackFailed { .. } => "trash_rollback_failed",
        }
    }

    /// Stable path-redacted message.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::InvalidRetention => {
                "trash retention must be a non-zero whole-second duration"
            }
            Self::InvalidTimestamp => "trash timestamp could not be represented safely",
            Self::Path(error) => error.message(),
            Self::RootUnavailable => "worktree root is unavailable for trash materialization",
            Self::UnsafeRuntimeDirectory => "worktree trash runtime directory is unsafe",
            Self::RuntimeDirectoryCreateFailed => {
                "worktree trash runtime directory could not be created"
            }
            Self::TargetNotRegularFile { .. } => "tombstone target is not a regular local file",
            Self::TargetReadFailed { .. } => "tombstone target could not be read safely",
            Self::TargetChangedBeforeMove { .. } => "tombstone target changed before retained move",
            Self::TrashDestinationExists { .. } => "retained trash destination already exists",
            Self::MetadataWriteFailed { .. } => "restore metadata could not be committed safely",
            Self::MetadataReadFailed => "restore metadata could not be read safely",
            Self::InvalidMetadata => "restore metadata is invalid",
            Self::RetainedFileMissing { .. } => "retained trash content is missing",
            Self::RetainedFileMismatch { .. } => "retained trash content does not match metadata",
            Self::MoveFailed { .. } => "local file could not be moved into retained trash",
            Self::RollbackFailed { .. } => {
                "failed trash operation could not restore the original local file"
            }
        }
    }
}

impl fmt::Display for WorktreeTrashError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for WorktreeTrashError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Path(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WorktreePathError> for WorktreeTrashError {
    fn from(error: WorktreePathError) -> Self {
        Self::Path(error)
    }
}

/// Worktree-owned reversible trash manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeTrashManager {
    config: WorktreeConfig,
    policy: WorktreeTrashPolicy,
}

impl WorktreeTrashManager {
    /// Create a manager for one configured worktree root.
    #[must_use]
    pub fn new(config: WorktreeConfig, policy: WorktreeTrashPolicy) -> Self {
        Self { config, policy }
    }

    /// Move a local file into retained trash and then advance local tombstone state.
    pub fn materialize_tombstone(
        &self,
        state: &mut WorktreeStateSnapshot,
        request: WorktreeTombstoneMaterializationRequest,
    ) -> Result<WorktreeTombstoneMaterializationOutcome, WorktreeTrashError> {
        ensure_existing_root_chain(self.config.root_path())?;
        let retained_at_seconds = unix_seconds(request.tombstoned_at)?;
        let retained_at = system_time_from_unix_seconds(retained_at_seconds)
            .ok_or(WorktreeTrashError::InvalidTimestamp)?;
        let retention_until = retained_at
            .checked_add(self.policy.retention)
            .ok_or(WorktreeTrashError::InvalidTimestamp)?;
        let retention_until_seconds = unix_seconds(retention_until)?;
        let local_path = self.config.vault_path_to_local(&request.vault_path)?;

        let Some(observed) = observe_regular_file(&local_path, &request.vault_path)? else {
            state.record_tombstoned(
                request.vault_path.clone(),
                request.tombstone_revision_id.clone(),
            );
            return Ok(WorktreeTombstoneMaterializationOutcome::AlreadyAbsent {
                vault_path: request.vault_path,
                tombstone_revision_id: request.tombstone_revision_id,
                retention_until,
            });
        };

        let record = WorktreeTrashRecord {
            record_id: record_id_for(
                &request.vault_path,
                &request.tombstone_revision_id,
                observed.content_hash,
                observed.size,
                retained_at_seconds,
                retention_until_seconds,
            ),
            vault_path: request.vault_path.clone(),
            tombstone_revision_id: request.tombstone_revision_id.clone(),
            content_hash: observed.content_hash,
            size: observed.size,
            retained_at,
            retention_until,
        };

        self.ensure_runtime_layout(&record)?;
        let destination = self.retained_file_path(&record);
        let metadata_path = self.metadata_path(&record.record_id);
        ensure_absent(&destination, &request.vault_path)?;
        ensure_absent(&metadata_path, &request.vault_path)?;
        let staged_metadata = self.stage_metadata(&record)?;

        let current = observe_regular_file(&local_path, &request.vault_path)?.ok_or_else(|| {
            cleanup_owned_file(&staged_metadata);
            WorktreeTrashError::TargetChangedBeforeMove {
                vault_path: request.vault_path.clone(),
            }
        })?;
        if current != observed {
            cleanup_owned_file(&staged_metadata);
            return Err(WorktreeTrashError::TargetChangedBeforeMove {
                vault_path: request.vault_path,
            });
        }

        if fs::rename(&local_path, &destination).is_err() {
            cleanup_owned_file(&staged_metadata);
            return Err(WorktreeTrashError::MoveFailed {
                vault_path: request.vault_path,
            });
        }
        if sync_directory(local_path.parent()).is_err()
            || sync_directory(destination.parent()).is_err()
        {
            cleanup_owned_file(&staged_metadata);
            rollback_move(&destination, &local_path, &request.vault_path)?;
            return Err(WorktreeTrashError::MoveFailed {
                vault_path: request.vault_path,
            });
        }

        if fs::rename(&staged_metadata, &metadata_path).is_err()
            || sync_directory(metadata_path.parent()).is_err()
        {
            cleanup_owned_file(&staged_metadata);
            cleanup_owned_file(&metadata_path);
            let _ = sync_directory(metadata_path.parent());
            rollback_move(&destination, &local_path, &request.vault_path)?;
            return Err(WorktreeTrashError::MetadataWriteFailed {
                vault_path: request.vault_path,
            });
        }

        state.record_tombstoned(request.vault_path, request.tombstone_revision_id);
        Ok(WorktreeTombstoneMaterializationOutcome::Trashed(record))
    }

    /// Load and validate restore metadata together with its retained bytes.
    pub fn load_record(
        &self,
        record_id: &WorktreeTrashRecordId,
    ) -> Result<WorktreeTrashRecord, WorktreeTrashError> {
        ensure_existing_root_chain(self.config.root_path())?;
        self.validate_metadata_layout()?;
        let metadata_path = self.metadata_path(record_id);
        let bytes = read_metadata_file(&metadata_path)?;
        let record = parse_metadata(&bytes)?;
        if &record.record_id != record_id || recompute_record_id(&record)? != record.record_id {
            return Err(WorktreeTrashError::InvalidMetadata);
        }
        self.validate_retained_file(&record)?;
        Ok(record)
    }

    fn ensure_runtime_layout(
        &self,
        record: &WorktreeTrashRecord,
    ) -> Result<(), WorktreeTrashError> {
        ensure_directory(&self.config.runtime_dir())?;
        ensure_directory(&self.config.temp_dir())?;
        ensure_directory(&self.config.trash_dir())?;
        let records_root = self.config.trash_dir().join(TRASH_RECORDS_DIR_NAME);
        ensure_directory(&records_root)?;
        let record_root = records_root.join(record.record_id.as_str());
        ensure_directory(&record_root)?;
        let mut current = record_root;
        let segments: Vec<&str> = record.vault_path.segments().collect();
        for segment in segments.iter().take(segments.len().saturating_sub(1)) {
            current.push(segment);
            ensure_directory(&current)?;
        }
        ensure_directory(&self.config.metadata_dir())?;
        ensure_directory(&self.config.metadata_dir().join(TRASH_METADATA_DIR_NAME))
    }

    fn validate_metadata_layout(&self) -> Result<(), WorktreeTrashError> {
        for directory in [
            self.config.runtime_dir(),
            self.config.metadata_dir(),
            self.config.metadata_dir().join(TRASH_METADATA_DIR_NAME),
        ] {
            validate_existing_directory(&directory, WorktreeTrashError::MetadataReadFailed)?;
        }
        Ok(())
    }

    fn validate_retained_file(
        &self,
        record: &WorktreeTrashRecord,
    ) -> Result<(), WorktreeTrashError> {
        let missing = WorktreeTrashError::RetainedFileMissing {
            vault_path: record.vault_path.clone(),
        };
        let records_root = self.config.trash_dir().join(TRASH_RECORDS_DIR_NAME);
        let record_root = records_root.join(record.record_id.as_str());
        for directory in [
            self.config.runtime_dir(),
            self.config.trash_dir(),
            records_root,
            record_root.clone(),
        ] {
            validate_existing_directory(&directory, missing.clone())?;
        }

        let segments: Vec<&str> = record.vault_path.segments().collect();
        let mut current = record_root;
        for segment in segments.iter().take(segments.len().saturating_sub(1)) {
            current.push(segment);
            validate_existing_directory(&current, missing.clone())?;
        }

        let retained_path = self.retained_file_path(record);
        let observed = observe_regular_file(&retained_path, &record.vault_path)?
            .ok_or_else(|| missing.clone())?;
        if observed.content_hash != record.content_hash || observed.size != record.size {
            return Err(WorktreeTrashError::RetainedFileMismatch {
                vault_path: record.vault_path.clone(),
            });
        }
        Ok(())
    }

    fn retained_file_path(&self, record: &WorktreeTrashRecord) -> PathBuf {
        let mut path = self
            .config
            .trash_dir()
            .join(TRASH_RECORDS_DIR_NAME)
            .join(record.record_id.as_str());
        for segment in record.vault_path.segments() {
            path.push(segment);
        }
        path
    }

    fn metadata_path(&self, record_id: &WorktreeTrashRecordId) -> PathBuf {
        self.config
            .metadata_dir()
            .join(TRASH_METADATA_DIR_NAME)
            .join(format!("{}{}", record_id.as_str(), TRASH_METADATA_SUFFIX))
    }

    fn stage_metadata(&self, record: &WorktreeTrashRecord) -> Result<PathBuf, WorktreeTrashError> {
        let id = NEXT_METADATA_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let temp_path = self.config.temp_dir().join(format!(
            "{METADATA_TEMP_PREFIX}{}-{id}{METADATA_TEMP_SUFFIX}",
            std::process::id()
        ));
        let bytes = metadata_bytes(record)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|_| WorktreeTrashError::MetadataWriteFailed {
                vault_path: record.vault_path.clone(),
            })?;
        if file.write_all(&bytes).is_err() || file.sync_all().is_err() {
            cleanup_owned_file(&temp_path);
            return Err(WorktreeTrashError::MetadataWriteFailed {
                vault_path: record.vault_path.clone(),
            });
        }
        Ok(temp_path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedRegularFile {
    content_hash: ContentHash,
    size: u64,
    modified: Option<SystemTime>,
}

fn observe_regular_file(
    path: &Path,
    vault_path: &VaultPath,
) -> Result<Option<ObservedRegularFile>, WorktreeTrashError> {
    let before = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(WorktreeTrashError::TargetReadFailed {
                vault_path: vault_path.clone(),
            });
        }
    };
    ensure_regular_file(&before, vault_path)?;

    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| WorktreeTrashError::TargetReadFailed {
            vault_path: vault_path.clone(),
        })?
        .read_to_end(&mut bytes)
        .map_err(|_| WorktreeTrashError::TargetReadFailed {
            vault_path: vault_path.clone(),
        })?;

    let after =
        fs::symlink_metadata(path).map_err(|_| WorktreeTrashError::TargetChangedBeforeMove {
            vault_path: vault_path.clone(),
        })?;
    ensure_regular_file(&after, vault_path)?;
    if !same_file_metadata(&before, &after) || after.len() != bytes.len() as u64 {
        return Err(WorktreeTrashError::TargetChangedBeforeMove {
            vault_path: vault_path.clone(),
        });
    }

    Ok(Some(ObservedRegularFile {
        content_hash: content_hash_for_bytes(&bytes),
        size: after.len(),
        modified: after.modified().ok(),
    }))
}

fn read_metadata_file(path: &Path) -> Result<Vec<u8>, WorktreeTrashError> {
    let before = fs::symlink_metadata(path).map_err(|_| WorktreeTrashError::MetadataReadFailed)?;
    if !before.file_type().is_file() || before.len() > MAX_TRASH_METADATA_BYTES {
        return Err(WorktreeTrashError::InvalidMetadata);
    }

    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| WorktreeTrashError::MetadataReadFailed)?
        .take(MAX_TRASH_METADATA_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| WorktreeTrashError::MetadataReadFailed)?;
    if bytes.len() as u64 > MAX_TRASH_METADATA_BYTES {
        return Err(WorktreeTrashError::InvalidMetadata);
    }

    let after = fs::symlink_metadata(path).map_err(|_| WorktreeTrashError::MetadataReadFailed)?;
    if !same_file_metadata(&before, &after) || after.len() != bytes.len() as u64 {
        return Err(WorktreeTrashError::InvalidMetadata);
    }
    Ok(bytes)
}

fn ensure_regular_file(
    metadata: &Metadata,
    vault_path: &VaultPath,
) -> Result<(), WorktreeTrashError> {
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(WorktreeTrashError::TargetNotRegularFile {
            vault_path: vault_path.clone(),
        });
    }
    Ok(())
}

fn same_file_metadata(before: &Metadata, after: &Metadata) -> bool {
    before.file_type().is_file() == after.file_type().is_file()
        && before.len() == after.len()
        && before.modified().ok() == after.modified().ok()
}

fn ensure_existing_root_chain(root: &Path) -> Result<(), WorktreeTrashError> {
    let mut ancestors: Vec<&Path> = root
        .ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .collect();
    ancestors.reverse();
    for ancestor in ancestors {
        let metadata =
            fs::symlink_metadata(ancestor).map_err(|_| WorktreeTrashError::RootUnavailable)?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
            return Err(WorktreeTrashError::RootUnavailable);
        }
    }
    Ok(())
}

fn ensure_directory(path: &Path) -> Result<(), WorktreeTrashError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                return Err(WorktreeTrashError::UnsafeRuntimeDirectory);
            }
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|_| WorktreeTrashError::RuntimeDirectoryCreateFailed)?;
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| WorktreeTrashError::RuntimeDirectoryCreateFailed)?;
            if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                return Err(WorktreeTrashError::UnsafeRuntimeDirectory);
            }
            Ok(())
        }
        Err(_) => Err(WorktreeTrashError::UnsafeRuntimeDirectory),
    }
}

fn validate_existing_directory(
    path: &Path,
    missing_error: WorktreeTrashError,
) -> Result<(), WorktreeTrashError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                return Err(WorktreeTrashError::UnsafeRuntimeDirectory);
            }
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(missing_error),
        Err(_) => Err(WorktreeTrashError::UnsafeRuntimeDirectory),
    }
}

fn ensure_absent(path: &Path, vault_path: &VaultPath) -> Result<(), WorktreeTrashError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Ok(_) | Err(_) => Err(WorktreeTrashError::TrashDestinationExists {
            vault_path: vault_path.clone(),
        }),
    }
}

fn rollback_move(
    retained_path: &Path,
    original_path: &Path,
    vault_path: &VaultPath,
) -> Result<(), WorktreeTrashError> {
    fs::rename(retained_path, original_path).map_err(|_| WorktreeTrashError::RollbackFailed {
        vault_path: vault_path.clone(),
    })?;
    sync_directory(original_path.parent()).map_err(|_| WorktreeTrashError::RollbackFailed {
        vault_path: vault_path.clone(),
    })?;
    sync_directory(retained_path.parent()).map_err(|_| WorktreeTrashError::RollbackFailed {
        vault_path: vault_path.clone(),
    })
}

fn cleanup_owned_file(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => {}
    }
}

#[cfg(unix)]
fn sync_directory(directory: Option<&Path>) -> io::Result<()> {
    let directory = directory.ok_or_else(|| io::Error::other("missing directory"))?;
    File::open(directory)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_directory: Option<&Path>) -> io::Result<()> {
    Ok(())
}

fn record_id_for(
    vault_path: &VaultPath,
    revision_id: &RevisionId,
    content_hash: ContentHash,
    size: u64,
    retained_at_seconds: u64,
    retention_until_seconds: u64,
) -> WorktreeTrashRecordId {
    let mut bytes = Vec::new();
    append_length_prefixed(&mut bytes, vault_path.as_str().as_bytes());
    append_length_prefixed(&mut bytes, revision_id.as_str().as_bytes());
    append_length_prefixed(&mut bytes, content_hash.to_string().as_bytes());
    bytes.extend_from_slice(&size.to_be_bytes());
    bytes.extend_from_slice(&retained_at_seconds.to_be_bytes());
    bytes.extend_from_slice(&retention_until_seconds.to_be_bytes());
    WorktreeTrashRecordId(content_hash_for_bytes(&bytes).as_hex().to_owned())
}

fn recompute_record_id(
    record: &WorktreeTrashRecord,
) -> Result<WorktreeTrashRecordId, WorktreeTrashError> {
    Ok(record_id_for(
        &record.vault_path,
        &record.tombstone_revision_id,
        record.content_hash,
        record.size,
        unix_seconds(record.retained_at)?,
        unix_seconds(record.retention_until)?,
    ))
}

fn append_length_prefixed(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u64).to_be_bytes());
    output.extend_from_slice(value);
}

fn metadata_bytes(record: &WorktreeTrashRecord) -> Result<Vec<u8>, WorktreeTrashError> {
    Ok(format!(
        "version={TRASH_METADATA_VERSION}\nrecord_id={}\npath_hex={}\ntombstone_revision_hex={}\ncontent_hash={}\nsize={}\nretained_at_unix_seconds={}\nretention_until_unix_seconds={}\n",
        record.record_id.as_str(),
        hex_encode(record.vault_path.as_str().as_bytes()),
        hex_encode(record.tombstone_revision_id.as_str().as_bytes()),
        record.content_hash,
        record.size,
        unix_seconds(record.retained_at)?,
        unix_seconds(record.retention_until)?,
    )
    .into_bytes())
}

fn parse_metadata(bytes: &[u8]) -> Result<WorktreeTrashRecord, WorktreeTrashError> {
    let text = std::str::from_utf8(bytes).map_err(|_| WorktreeTrashError::InvalidMetadata)?;
    let mut version = None;
    let mut record_id = None;
    let mut path_hex = None;
    let mut revision_hex = None;
    let mut content_hash = None;
    let mut size = None;
    let mut retained_at = None;
    let mut retention_until = None;

    for line in text.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(WorktreeTrashError::InvalidMetadata)?;
        match key {
            "version" if version.replace(value).is_none() => {}
            "record_id" if record_id.replace(value).is_none() => {}
            "path_hex" if path_hex.replace(value).is_none() => {}
            "tombstone_revision_hex" if revision_hex.replace(value).is_none() => {}
            "content_hash" if content_hash.replace(value).is_none() => {}
            "size" if size.replace(value).is_none() => {}
            "retained_at_unix_seconds" if retained_at.replace(value).is_none() => {}
            "retention_until_unix_seconds" if retention_until.replace(value).is_none() => {}
            _ => return Err(WorktreeTrashError::InvalidMetadata),
        }
    }
    if version != Some(TRASH_METADATA_VERSION) {
        return Err(WorktreeTrashError::InvalidMetadata);
    }

    let path = String::from_utf8(hex_decode(
        path_hex.ok_or(WorktreeTrashError::InvalidMetadata)?,
    )?)
    .map_err(|_| WorktreeTrashError::InvalidMetadata)?;
    let revision = String::from_utf8(hex_decode(
        revision_hex.ok_or(WorktreeTrashError::InvalidMetadata)?,
    )?)
    .map_err(|_| WorktreeTrashError::InvalidMetadata)?;
    let retained_at_seconds = parse_u64(retained_at)?;
    let retention_until_seconds = parse_u64(retention_until)?;
    if retention_until_seconds <= retained_at_seconds {
        return Err(WorktreeTrashError::InvalidMetadata);
    }
    let retained_at = system_time_from_unix_seconds(retained_at_seconds)
        .ok_or(WorktreeTrashError::InvalidMetadata)?;
    let retention_until = system_time_from_unix_seconds(retention_until_seconds)
        .ok_or(WorktreeTrashError::InvalidMetadata)?;

    Ok(WorktreeTrashRecord {
        record_id: WorktreeTrashRecordId::parse(
            record_id.ok_or(WorktreeTrashError::InvalidMetadata)?,
        )?,
        vault_path: VaultPath::parse(&path).map_err(|_| WorktreeTrashError::InvalidMetadata)?,
        tombstone_revision_id: RevisionId::parse(&revision)
            .map_err(|_| WorktreeTrashError::InvalidMetadata)?,
        content_hash: ContentHash::parse(content_hash.ok_or(WorktreeTrashError::InvalidMetadata)?)
            .map_err(|_| WorktreeTrashError::InvalidMetadata)?,
        size: parse_u64(size)?,
        retained_at,
        retention_until,
    })
}

fn parse_u64(value: Option<&str>) -> Result<u64, WorktreeTrashError> {
    value
        .ok_or(WorktreeTrashError::InvalidMetadata)?
        .parse()
        .map_err(|_| WorktreeTrashError::InvalidMetadata)
}

fn unix_seconds(time: SystemTime) -> Result<u64, WorktreeTrashError> {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| WorktreeTrashError::InvalidTimestamp)
}

fn system_time_from_unix_seconds(seconds: u64) -> Option<SystemTime> {
    UNIX_EPOCH.checked_add(Duration::from_secs(seconds))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn hex_decode(input: &str) -> Result<Vec<u8>, WorktreeTrashError> {
    if input.len() % 2 != 0 {
        return Err(WorktreeTrashError::InvalidMetadata);
    }
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn hex_nibble(byte: u8) -> Result<u8, WorktreeTrashError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(WorktreeTrashError::InvalidMetadata),
    }
}
