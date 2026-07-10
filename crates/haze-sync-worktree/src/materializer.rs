//! Safe materialization of authoritative Core revisions into the local worktree.

use crate::hashing::content_hash_for_bytes;
use crate::{
    WorktreeAppliedPathState, WorktreeBaseRevision, WorktreeConfig, WorktreePathError,
    WorktreeStateSnapshot,
};
use haze_sync_common::{ContentHash, RevisionId, VaultPath};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const WRITE_TEMP_PREFIX: &str = ".haze-write-";
const ECHO_TEMP_PREFIX: &str = ".haze-echo-";
const TEMP_SUFFIX: &str = ".tmp";
const ECHO_MARKER_SUFFIX: &str = ".echo";

static NEXT_TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);

/// Authoritative file revision to materialize into the local worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeMaterializationRequest {
    /// Vault-relative file path.
    pub vault_path: VaultPath,
    /// Authoritative Core revision represented by the bytes.
    pub revision_id: RevisionId,
    /// Expected SHA-256 content hash.
    pub content_hash: ContentHash,
    /// Authoritative file bytes.
    pub bytes: Vec<u8>,
}

/// Metadata marker for a file written by the Worktree adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEchoMarker {
    /// Vault-relative file path written by the adapter.
    pub vault_path: VaultPath,
    /// Authoritative revision written by the adapter.
    pub revision_id: RevisionId,
    /// Content hash written by the adapter.
    pub content_hash: ContentHash,
}

/// Authoritative file state represented locally after materialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeMaterializedFile {
    /// Vault-relative file path.
    pub vault_path: VaultPath,
    /// Authoritative revision represented locally.
    pub revision_id: RevisionId,
    /// Authoritative content hash represented locally.
    pub content_hash: ContentHash,
}

/// Local change that must be submitted before an incoming revision can be applied safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeLocalImportCandidate {
    /// A locally new or modified file.
    Put {
        /// Vault-relative path of the local file.
        vault_path: VaultPath,
        /// Known Core base revision or explicit null base.
        base_revision: WorktreeBaseRevision,
        /// Hash computed from the stable local bytes.
        content_hash: ContentHash,
        /// Stable local bytes to submit through Core/API.
        bytes: Vec<u8>,
    },
    /// A local deletion of a file previously applied from Core.
    Delete {
        /// Vault-relative path deleted locally.
        vault_path: VaultPath,
        /// Known Core base revision for the deleted file.
        base_revision: RevisionId,
    },
}

/// Deferred incoming materialization paired with the local import candidate blocking it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDeferredMaterialization {
    /// Incoming authoritative request that may be retried after Core handles the local change.
    pub request: WorktreeMaterializationRequest,
    /// Local change that must be submitted instead of silently overwritten.
    pub local_change: WorktreeLocalImportCandidate,
}

/// Result of evaluating and applying one materialization request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeMaterializationOutcome {
    /// The adapter atomically wrote the incoming bytes and an echo marker.
    Applied {
        /// Authoritative file state now represented locally.
        file: WorktreeMaterializedFile,
        /// Echo marker written under the reserved runtime area.
        echo_marker: WorktreeEchoMarker,
    },
    /// The local file already matched the authoritative incoming content.
    AlreadyCurrent(WorktreeMaterializedFile),
    /// A dirty local change must be submitted before the incoming revision is retried.
    NeedsImport(WorktreeDeferredMaterialization),
}

/// Safe, path-redacted materialization failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeMaterializeError {
    /// Vault/local path mapping rejected the request.
    Path(WorktreePathError),
    /// Incoming bytes did not match the expected content hash.
    ContentHashMismatch { vault_path: VaultPath },
    /// The configured worktree root was unavailable.
    RootUnavailable,
    /// The configured worktree root was not a safe directory.
    UnsafeRoot,
    /// A parent component was not a safe directory.
    UnsafeParent { vault_path: VaultPath },
    /// A target path existed but was not a regular file.
    TargetNotRegularFile { vault_path: VaultPath },
    /// A local file could not be read safely.
    LocalReadFailed { vault_path: VaultPath },
    /// A local file changed while it was being observed.
    LocalChangedDuringRead { vault_path: VaultPath },
    /// The target changed after dirty-state evaluation and before atomic replacement.
    TargetChangedBeforeCommit { vault_path: VaultPath },
    /// A required destination parent directory could not be created.
    ParentDirectoryCreateFailed { vault_path: VaultPath },
    /// The reserved runtime directory could not be prepared safely.
    RuntimeDirectoryFailed,
    /// A temporary file could not be created.
    TempFileCreateFailed,
    /// A temporary file could not be written completely.
    TempFileWriteFailed,
    /// A temporary file could not be synchronized before rename.
    TempFileSyncFailed,
    /// The platform could not atomically replace an existing target.
    AtomicReplaceUnsupported { vault_path: VaultPath },
    /// The prepared file could not be atomically renamed into place.
    AtomicRenameFailed { vault_path: VaultPath },
    /// The destination directory could not be synchronized after rename.
    DirectorySyncFailed { vault_path: VaultPath },
    /// The echo marker could not be committed safely.
    EchoMarkerWriteFailed { vault_path: VaultPath },
    /// Stale Worktree-owned temp files could not be inspected or removed.
    TempCleanupFailed,
}

impl WorktreeMaterializeError {
    /// Stable machine-readable code with no local absolute path details.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Path(error) => error.code(),
            Self::ContentHashMismatch { .. } => "content_hash_mismatch",
            Self::RootUnavailable => "root_unavailable",
            Self::UnsafeRoot => "unsafe_root",
            Self::UnsafeParent { .. } => "unsafe_parent",
            Self::TargetNotRegularFile { .. } => "target_not_regular_file",
            Self::LocalReadFailed { .. } => "local_read_failed",
            Self::LocalChangedDuringRead { .. } => "local_changed_during_read",
            Self::TargetChangedBeforeCommit { .. } => "target_changed_before_commit",
            Self::ParentDirectoryCreateFailed { .. } => "parent_directory_create_failed",
            Self::RuntimeDirectoryFailed => "runtime_directory_failed",
            Self::TempFileCreateFailed => "temp_file_create_failed",
            Self::TempFileWriteFailed => "temp_file_write_failed",
            Self::TempFileSyncFailed => "temp_file_sync_failed",
            Self::AtomicReplaceUnsupported { .. } => "atomic_replace_unsupported",
            Self::AtomicRenameFailed { .. } => "atomic_rename_failed",
            Self::DirectorySyncFailed { .. } => "directory_sync_failed",
            Self::EchoMarkerWriteFailed { .. } => "echo_marker_write_failed",
            Self::TempCleanupFailed => "temp_cleanup_failed",
        }
    }

    /// Stable human-readable message with no absolute local paths or raw I/O errors.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::Path(error) => error.message(),
            Self::ContentHashMismatch { .. } => {
                "incoming bytes do not match the expected content hash"
            }
            Self::RootUnavailable => "worktree root is unavailable",
            Self::UnsafeRoot => "worktree root is not a safe directory",
            Self::UnsafeParent { .. } => "worktree parent path is not a safe directory",
            Self::TargetNotRegularFile { .. } => "worktree target is not a regular file",
            Self::LocalReadFailed { .. } => "local worktree file could not be read safely",
            Self::LocalChangedDuringRead { .. } => {
                "local worktree file changed while it was being read"
            }
            Self::TargetChangedBeforeCommit { .. } => {
                "worktree target changed before the atomic commit"
            }
            Self::ParentDirectoryCreateFailed { .. } => {
                "worktree parent directory could not be created"
            }
            Self::RuntimeDirectoryFailed => {
                "reserved worktree runtime directory could not be prepared"
            }
            Self::TempFileCreateFailed => "worktree temporary file could not be created",
            Self::TempFileWriteFailed => "worktree temporary file could not be written",
            Self::TempFileSyncFailed => "worktree temporary file could not be synchronized",
            Self::AtomicReplaceUnsupported { .. } => {
                "atomic replacement of an existing worktree file is unsupported"
            }
            Self::AtomicRenameFailed { .. } => "worktree file could not be renamed atomically",
            Self::DirectorySyncFailed { .. } => {
                "worktree destination directory could not be synchronized"
            }
            Self::EchoMarkerWriteFailed { .. } => {
                "worktree echo marker could not be committed safely"
            }
            Self::TempCleanupFailed => "stale worktree temporary files could not be cleaned",
        }
    }

    /// Safe vault-relative path context when the error belongs to one file.
    #[must_use]
    pub fn vault_path(&self) -> Option<&VaultPath> {
        match self {
            Self::ContentHashMismatch { vault_path }
            | Self::UnsafeParent { vault_path }
            | Self::TargetNotRegularFile { vault_path }
            | Self::LocalReadFailed { vault_path }
            | Self::LocalChangedDuringRead { vault_path }
            | Self::TargetChangedBeforeCommit { vault_path }
            | Self::ParentDirectoryCreateFailed { vault_path }
            | Self::AtomicReplaceUnsupported { vault_path }
            | Self::AtomicRenameFailed { vault_path }
            | Self::DirectorySyncFailed { vault_path }
            | Self::EchoMarkerWriteFailed { vault_path } => Some(vault_path),
            Self::Path(_)
            | Self::RootUnavailable
            | Self::UnsafeRoot
            | Self::RuntimeDirectoryFailed
            | Self::TempFileCreateFailed
            | Self::TempFileWriteFailed
            | Self::TempFileSyncFailed
            | Self::TempCleanupFailed => None,
        }
    }
}

impl fmt::Display for WorktreeMaterializeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for WorktreeMaterializeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Path(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WorktreePathError> for WorktreeMaterializeError {
    fn from(error: WorktreePathError) -> Self {
        Self::Path(error)
    }
}

/// Atomic writer for Worktree-owned file and runtime-marker commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicWorktreeWriter {
    config: WorktreeConfig,
}

impl AtomicWorktreeWriter {
    /// Create an atomic writer for one configured worktree root.
    #[must_use]
    pub fn new(config: WorktreeConfig) -> Self {
        Self { config }
    }

    /// Remove stale Worktree-owned temporary files left by interrupted writes.
    pub fn cleanup_stale_temp_files(&self) -> Result<usize, WorktreeMaterializeError> {
        self.ensure_runtime_directory(&[crate::TEMP_DIR_NAME])?;
        let entries = fs::read_dir(self.config.temp_dir())
            .map_err(|_| WorktreeMaterializeError::TempCleanupFailed)?;
        let mut removed = 0;

        for entry in entries {
            let entry = entry.map_err(|_| WorktreeMaterializeError::TempCleanupFailed)?;
            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str() else {
                continue;
            };
            if !is_owned_temp_name(file_name) {
                continue;
            }

            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|_| WorktreeMaterializeError::TempCleanupFailed)?;
            if !metadata.file_type().is_file() {
                continue;
            }
            fs::remove_file(entry.path())
                .map_err(|_| WorktreeMaterializeError::TempCleanupFailed)?;
            removed += 1;
        }

        Ok(removed)
    }

    fn write_materialized_file(
        &self,
        request: &WorktreeMaterializationRequest,
        expected_target: ExpectedTarget,
        marker: &WorktreeEchoMarker,
    ) -> Result<(), WorktreeMaterializeError> {
        self.ensure_parent_directories(&request.vault_path)?;
        self.ensure_runtime_directory(&[crate::TEMP_DIR_NAME])?;
        self.ensure_runtime_directory(&[crate::ECHO_DIR_NAME])?;

        let staged_file = self.stage_bytes(WRITE_TEMP_PREFIX, &request.bytes)?;
        self.verify_expected_target(&request.vault_path, expected_target)?;
        self.commit_echo_marker(marker)?;

        let destination = self.config.vault_path_to_local(&request.vault_path)?;
        staged_file.commit_to(&destination, &request.vault_path)?;
        sync_directory(destination.parent(), &request.vault_path)?;
        Ok(())
    }

    fn stage_bytes(
        &self,
        prefix: &str,
        bytes: &[u8],
    ) -> Result<StagedFile, WorktreeMaterializeError> {
        let temp_path = self.next_temp_path(prefix);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|_| WorktreeMaterializeError::TempFileCreateFailed)?;
        file.write_all(bytes)
            .map_err(|_| WorktreeMaterializeError::TempFileWriteFailed)?;
        file.sync_all()
            .map_err(|_| WorktreeMaterializeError::TempFileSyncFailed)?;
        drop(file);
        Ok(StagedFile::new(temp_path))
    }

    fn commit_echo_marker(
        &self,
        marker: &WorktreeEchoMarker,
    ) -> Result<(), WorktreeMaterializeError> {
        let marker_bytes = echo_marker_bytes(marker);
        let marker_path = self.echo_marker_path(marker);

        match fs::symlink_metadata(&marker_path) {
            Ok(metadata) => {
                if !metadata.file_type().is_file() {
                    return Err(WorktreeMaterializeError::EchoMarkerWriteFailed {
                        vault_path: marker.vault_path.clone(),
                    });
                }
                let existing = fs::read(&marker_path).map_err(|_| {
                    WorktreeMaterializeError::EchoMarkerWriteFailed {
                        vault_path: marker.vault_path.clone(),
                    }
                })?;
                if existing == marker_bytes {
                    return Ok(());
                }
                return Err(WorktreeMaterializeError::EchoMarkerWriteFailed {
                    vault_path: marker.vault_path.clone(),
                });
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(WorktreeMaterializeError::EchoMarkerWriteFailed {
                    vault_path: marker.vault_path.clone(),
                });
            }
        }

        let staged_marker = self.stage_bytes(ECHO_TEMP_PREFIX, &marker_bytes)?;
        staged_marker
            .commit_new(&marker_path)
            .map_err(|_| WorktreeMaterializeError::EchoMarkerWriteFailed {
                vault_path: marker.vault_path.clone(),
            })?;
        sync_runtime_directory(marker_path.parent())
            .map_err(|_| WorktreeMaterializeError::EchoMarkerWriteFailed {
                vault_path: marker.vault_path.clone(),
            })?;
        Ok(())
    }

    fn verify_expected_target(
        &self,
        vault_path: &VaultPath,
        expected_target: ExpectedTarget,
    ) -> Result<(), WorktreeMaterializeError> {
        let observed = observe_local_file(&self.config, vault_path)?;
        let matches = match (expected_target, observed) {
            (ExpectedTarget::Absent, None) => true,
            (ExpectedTarget::Present(expected_hash), Some(file)) => {
                file.content_hash == expected_hash
            }
            _ => false,
        };

        if matches {
            Ok(())
        } else {
            Err(WorktreeMaterializeError::TargetChangedBeforeCommit {
                vault_path: vault_path.clone(),
            })
        }
    }

    fn ensure_parent_directories(
        &self,
        vault_path: &VaultPath,
    ) -> Result<(), WorktreeMaterializeError> {
        self.ensure_safe_root()?;
        let segments: Vec<&str> = vault_path.segments().collect();
        let mut current = self.config.root_path().to_path_buf();

        for segment in segments.iter().take(segments.len().saturating_sub(1)) {
            current.push(segment);
            ensure_directory_component(&current, || {
                WorktreeMaterializeError::UnsafeParent {
                    vault_path: vault_path.clone(),
                }
            }, || WorktreeMaterializeError::ParentDirectoryCreateFailed {
                vault_path: vault_path.clone(),
            })?;
        }
        Ok(())
    }

    fn ensure_runtime_directory(
        &self,
        suffix_segments: &[&str],
    ) -> Result<(), WorktreeMaterializeError> {
        self.ensure_safe_root()?;
        let mut current = self.config.root_path().to_path_buf();
        current.push(crate::WORKTREE_RUNTIME_DIR_NAME);
        ensure_directory_component(
            &current,
            || WorktreeMaterializeError::RuntimeDirectoryFailed,
            || WorktreeMaterializeError::RuntimeDirectoryFailed,
        )?;

        for segment in suffix_segments {
            current.push(segment);
            ensure_directory_component(
                &current,
                || WorktreeMaterializeError::RuntimeDirectoryFailed,
                || WorktreeMaterializeError::RuntimeDirectoryFailed,
            )?;
        }
        Ok(())
    }

    fn ensure_safe_root(&self) -> Result<(), WorktreeMaterializeError> {
        let metadata = match fs::symlink_metadata(self.config.root_path()) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(WorktreeMaterializeError::RootUnavailable);
            }
            Err(_) => return Err(WorktreeMaterializeError::RootUnavailable),
        };

        if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
            return Err(WorktreeMaterializeError::UnsafeRoot);
        }
        Ok(())
    }

    fn next_temp_path(&self, prefix: &str) -> PathBuf {
        let id = NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        self.config.temp_dir().join(format!(
            "{prefix}{}-{id}{TEMP_SUFFIX}",
            std::process::id()
        ))
    }

    fn echo_marker_path(&self, marker: &WorktreeEchoMarker) -> PathBuf {
        let path_hash = content_hash_for_bytes(marker.vault_path.as_str().as_bytes()).as_hex();
        let revision_hash = content_hash_for_bytes(marker.revision_id.as_str().as_bytes()).as_hex();
        self.config.echo_dir().join(format!(
            "{path_hash}-{revision_hash}-{}{ECHO_MARKER_SUFFIX}",
            marker.content_hash.as_hex()
        ))
    }
}

/// Materializes authoritative Core revisions while protecting dirty local files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeMaterializer {
    writer: AtomicWorktreeWriter,
}

impl WorktreeMaterializer {
    /// Create a materializer for one configured worktree root.
    #[must_use]
    pub fn new(config: WorktreeConfig) -> Self {
        Self {
            writer: AtomicWorktreeWriter::new(config),
        }
    }

    /// Access the underlying atomic writer for startup temp cleanup.
    #[must_use]
    pub fn writer(&self) -> &AtomicWorktreeWriter {
        &self.writer
    }

    /// Materialize one authoritative file revision or return a dirty-local import plan.
    pub fn materialize(
        &self,
        state: &mut WorktreeStateSnapshot,
        request: WorktreeMaterializationRequest,
    ) -> Result<WorktreeMaterializationOutcome, WorktreeMaterializeError> {
        verify_request_content_hash(&request)?;
        let local_file = observe_local_file(&self.writer.config, &request.vault_path)?;

        if local_file
            .as_ref()
            .is_some_and(|local| local.content_hash == request.content_hash)
        {
            let file = materialized_file(&request);
            state.record_present(
                request.vault_path,
                request.revision_id,
                request.content_hash,
            );
            return Ok(WorktreeMaterializationOutcome::AlreadyCurrent(file));
        }

        if let Some(local_change) = dirty_local_change(
            state.path_state(&request.vault_path),
            &request.vault_path,
            local_file.as_ref(),
        ) {
            return Ok(WorktreeMaterializationOutcome::NeedsImport(
                WorktreeDeferredMaterialization {
                    request,
                    local_change,
                },
            ));
        }

        let expected_target = local_file
            .as_ref()
            .map_or(ExpectedTarget::Absent, |local| {
                ExpectedTarget::Present(local.content_hash)
            });
        let marker = WorktreeEchoMarker {
            vault_path: request.vault_path.clone(),
            revision_id: request.revision_id.clone(),
            content_hash: request.content_hash,
        };
        self.writer
            .write_materialized_file(&request, expected_target, &marker)?;

        let file = materialized_file(&request);
        state.record_present(
            request.vault_path,
            request.revision_id,
            request.content_hash,
        );
        Ok(WorktreeMaterializationOutcome::Applied {
            file,
            echo_marker: marker,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedTarget {
    Absent,
    Present(ContentHash),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalFileObservation {
    content_hash: ContentHash,
    bytes: Vec<u8>,
}

fn verify_request_content_hash(
    request: &WorktreeMaterializationRequest,
) -> Result<(), WorktreeMaterializeError> {
    if content_hash_for_bytes(&request.bytes) == request.content_hash {
        Ok(())
    } else {
        Err(WorktreeMaterializeError::ContentHashMismatch {
            vault_path: request.vault_path.clone(),
        })
    }
}

fn materialized_file(request: &WorktreeMaterializationRequest) -> WorktreeMaterializedFile {
    WorktreeMaterializedFile {
        vault_path: request.vault_path.clone(),
        revision_id: request.revision_id.clone(),
        content_hash: request.content_hash,
    }
}

fn dirty_local_change(
    path_state: Option<&WorktreeAppliedPathState>,
    vault_path: &VaultPath,
    local_file: Option<&LocalFileObservation>,
) -> Option<WorktreeLocalImportCandidate> {
    match (path_state, local_file) {
        (Some(WorktreeAppliedPathState::Present(applied)), None) => {
            Some(WorktreeLocalImportCandidate::Delete {
                vault_path: vault_path.clone(),
                base_revision: applied.revision_id.clone(),
            })
        }
        (Some(WorktreeAppliedPathState::Present(applied)), Some(local))
            if local.content_hash != applied.content_hash =>
        {
            Some(local_put_candidate(
                vault_path,
                WorktreeBaseRevision::Known(applied.revision_id.clone()),
                local,
            ))
        }
        (Some(WorktreeAppliedPathState::Tombstoned(tombstone)), Some(local)) => {
            Some(local_put_candidate(
                vault_path,
                WorktreeBaseRevision::Known(tombstone.revision_id.clone()),
                local,
            ))
        }
        (None, Some(local)) => Some(local_put_candidate(
            vault_path,
            WorktreeBaseRevision::Null,
            local,
        )),
        _ => None,
    }
}

fn local_put_candidate(
    vault_path: &VaultPath,
    base_revision: WorktreeBaseRevision,
    local: &LocalFileObservation,
) -> WorktreeLocalImportCandidate {
    WorktreeLocalImportCandidate::Put {
        vault_path: vault_path.clone(),
        base_revision,
        content_hash: local.content_hash,
        bytes: local.bytes.clone(),
    }
}

fn observe_local_file(
    config: &WorktreeConfig,
    vault_path: &VaultPath,
) -> Result<Option<LocalFileObservation>, WorktreeMaterializeError> {
    let local_path = config.vault_path_to_local(vault_path)?;
    let before = match fs::symlink_metadata(&local_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(WorktreeMaterializeError::LocalReadFailed {
                vault_path: vault_path.clone(),
            });
        }
    };
    ensure_regular_target(&before, vault_path)?;

    let mut file = File::open(&local_path).map_err(|_| WorktreeMaterializeError::LocalReadFailed {
        vault_path: vault_path.clone(),
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| WorktreeMaterializeError::LocalReadFailed {
            vault_path: vault_path.clone(),
        })?;
    drop(file);

    let after = fs::symlink_metadata(&local_path).map_err(|_| {
        WorktreeMaterializeError::LocalChangedDuringRead {
            vault_path: vault_path.clone(),
        }
    })?;
    ensure_regular_target(&after, vault_path)?;

    if !same_file_observation(&before, &after) || after.len() != bytes.len() as u64 {
        return Err(WorktreeMaterializeError::LocalChangedDuringRead {
            vault_path: vault_path.clone(),
        });
    }

    Ok(Some(LocalFileObservation {
        content_hash: content_hash_for_bytes(&bytes),
        bytes,
    }))
}

fn same_file_observation(before: &fs::Metadata, after: &fs::Metadata) -> bool {
    before.len() == after.len()
        && before.modified().ok() == after.modified().ok()
        && before.file_type().is_file() == after.file_type().is_file()
}

fn ensure_regular_target(
    metadata: &fs::Metadata,
    vault_path: &VaultPath,
) -> Result<(), WorktreeMaterializeError> {
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        Err(WorktreeMaterializeError::TargetNotRegularFile {
            vault_path: vault_path.clone(),
        })
    } else {
        Ok(())
    }
}

fn ensure_directory_component<Unsafe, Create>(
    path: &Path,
    unsafe_error: Unsafe,
    create_error: Create,
) -> Result<(), WorktreeMaterializeError>
where
    Unsafe: Fn() -> WorktreeMaterializeError,
    Create: Fn() -> WorktreeMaterializeError,
{
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                return Err(unsafe_error());
            }
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|_| create_error())?;
            let metadata = fs::symlink_metadata(path).map_err(|_| create_error())?;
            if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                return Err(unsafe_error());
            }
            Ok(())
        }
        Err(_) => Err(unsafe_error()),
    }
}

fn echo_marker_bytes(marker: &WorktreeEchoMarker) -> Vec<u8> {
    format!(
        "version=1\npath={}\nrevision={}\ncontent_hash={}\n",
        marker.vault_path, marker.revision_id, marker.content_hash
    )
    .into_bytes()
}

fn is_owned_temp_name(file_name: &str) -> bool {
    (file_name.starts_with(WRITE_TEMP_PREFIX) || file_name.starts_with(ECHO_TEMP_PREFIX))
        && file_name.ends_with(TEMP_SUFFIX)
}

struct StagedFile {
    path: PathBuf,
    committed: bool,
}

impl StagedFile {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            committed: false,
        }
    }

    fn commit_to(
        mut self,
        destination: &Path,
        vault_path: &VaultPath,
    ) -> Result<(), WorktreeMaterializeError> {
        atomic_replace(&self.path, destination, vault_path)?;
        self.committed = true;
        Ok(())
    }

    fn commit_new(mut self, destination: &Path) -> io::Result<()> {
        fs::rename(&self.path, destination)?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        match fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => {}
        }
    }
}

#[cfg(unix)]
fn atomic_replace(
    staged_path: &Path,
    destination: &Path,
    vault_path: &VaultPath,
) -> Result<(), WorktreeMaterializeError> {
    fs::rename(staged_path, destination).map_err(|_| WorktreeMaterializeError::AtomicRenameFailed {
        vault_path: vault_path.clone(),
    })
}

#[cfg(not(unix))]
fn atomic_replace(
    staged_path: &Path,
    destination: &Path,
    vault_path: &VaultPath,
) -> Result<(), WorktreeMaterializeError> {
    if destination.exists() {
        return Err(WorktreeMaterializeError::AtomicReplaceUnsupported {
            vault_path: vault_path.clone(),
        });
    }
    fs::rename(staged_path, destination).map_err(|_| WorktreeMaterializeError::AtomicRenameFailed {
        vault_path: vault_path.clone(),
    })
}

#[cfg(unix)]
fn sync_directory(
    directory: Option<&Path>,
    vault_path: &VaultPath,
) -> Result<(), WorktreeMaterializeError> {
    let directory = directory.ok_or_else(|| WorktreeMaterializeError::DirectorySyncFailed {
        vault_path: vault_path.clone(),
    })?;
    File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|_| WorktreeMaterializeError::DirectorySyncFailed {
            vault_path: vault_path.clone(),
        })
}

#[cfg(not(unix))]
fn sync_directory(
    _directory: Option<&Path>,
    _vault_path: &VaultPath,
) -> Result<(), WorktreeMaterializeError> {
    Ok(())
}

#[cfg(unix)]
fn sync_runtime_directory(directory: Option<&Path>) -> io::Result<()> {
    let directory = directory.ok_or_else(|| io::Error::other("missing runtime directory"))?;
    File::open(directory)?.sync_all()
}

#[cfg(not(unix))]
fn sync_runtime_directory(_directory: Option<&Path>) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ROOT_ID: AtomicU64 = AtomicU64::new(0);

    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new(name: &str) -> Self {
            let id = NEXT_TEST_ROOT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "haze-sync-materializer-{name}-{}-{id}",
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
            match fs::remove_dir_all(&self.path) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(_) => {}
            }
        }
    }

    fn remove_dir_if_exists(path: &Path) {
        match fs::remove_dir_all(path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => panic!("failed to clear materializer test root: {error}"),
        }
    }

    fn path(input: &str) -> VaultPath {
        VaultPath::parse(input).unwrap()
    }

    fn revision(input: &str) -> RevisionId {
        RevisionId::parse(input).unwrap()
    }

    fn hash(bytes: &[u8]) -> ContentHash {
        content_hash_for_bytes(bytes)
    }

    fn request(vault_path: &str, revision_id: &str, bytes: &[u8]) -> WorktreeMaterializationRequest {
        WorktreeMaterializationRequest {
            vault_path: path(vault_path),
            revision_id: revision(revision_id),
            content_hash: hash(bytes),
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn materializes_new_file_updates_state_and_writes_echo_marker() {
        let root = TempRoot::new("new-file");
        let config = root.config();
        let materializer = WorktreeMaterializer::new(config.clone());
        let mut state = WorktreeStateSnapshot::new();

        let outcome = materializer
            .materialize(&mut state, request("Notes/a.md", "rev_new", b"hello"))
            .unwrap();

        assert!(matches!(outcome, WorktreeMaterializationOutcome::Applied { .. }));
        assert_eq!(
            fs::read(config.vault_path_to_local(&path("Notes/a.md")).unwrap()).unwrap(),
            b"hello"
        );
        assert!(matches!(
            state.path_state(&path("Notes/a.md")),
            Some(WorktreeAppliedPathState::Present(applied))
                if applied.revision_id == revision("rev_new")
                    && applied.content_hash == hash(b"hello")
        ));
        assert_eq!(fs::read_dir(config.echo_dir()).unwrap().count(), 1);
    }

    #[test]
    fn rejects_hash_mismatch_without_writing_target() {
        let root = TempRoot::new("hash-mismatch");
        let config = root.config();
        let materializer = WorktreeMaterializer::new(config.clone());
        let mut state = WorktreeStateSnapshot::new();
        let mut request = request("a.md", "rev_new", b"expected");
        request.bytes = b"different".to_vec();

        let error = materializer.materialize(&mut state, request).unwrap_err();

        assert_eq!(error.code(), "content_hash_mismatch");
        assert!(!config.vault_path_to_local(&path("a.md")).unwrap().exists());
        assert!(state.is_empty());
    }

    #[test]
    fn replaces_clean_existing_file_atomically() {
        let root = TempRoot::new("clean-replace");
        let config = root.config();
        let target = config.vault_path_to_local(&path("a.md")).unwrap();
        fs::write(&target, b"base").unwrap();
        let materializer = WorktreeMaterializer::new(config);
        let mut state = WorktreeStateSnapshot::new();
        state.record_present(path("a.md"), revision("rev_base"), hash(b"base"));

        let outcome = materializer
            .materialize(&mut state, request("a.md", "rev_remote", b"remote"))
            .unwrap();

        assert!(matches!(outcome, WorktreeMaterializationOutcome::Applied { .. }));
        assert_eq!(fs::read(target).unwrap(), b"remote");
    }

    #[test]
    fn returns_put_import_candidate_for_dirty_file_without_overwrite() {
        let root = TempRoot::new("dirty-file");
        let config = root.config();
        let target = config.vault_path_to_local(&path("a.md")).unwrap();
        fs::write(&target, b"local edit").unwrap();
        let materializer = WorktreeMaterializer::new(config);
        let mut state = WorktreeStateSnapshot::new();
        state.record_present(path("a.md"), revision("rev_base"), hash(b"base"));

        let outcome = materializer
            .materialize(&mut state, request("a.md", "rev_remote", b"remote"))
            .unwrap();

        assert!(matches!(
            outcome,
            WorktreeMaterializationOutcome::NeedsImport(WorktreeDeferredMaterialization {
                local_change: WorktreeLocalImportCandidate::Put {
                    base_revision: WorktreeBaseRevision::Known(base),
                    content_hash,
                    bytes,
                    ..
                },
                ..
            }) if base == revision("rev_base")
                && content_hash == hash(b"local edit")
                && bytes == b"local edit"
        ));
        assert_eq!(fs::read(target).unwrap(), b"local edit");
    }

    #[test]
    fn returns_delete_import_candidate_for_missing_tracked_file() {
        let root = TempRoot::new("dirty-delete");
        let config = root.config();
        let target = config.vault_path_to_local(&path("a.md")).unwrap();
        let materializer = WorktreeMaterializer::new(config);
        let mut state = WorktreeStateSnapshot::new();
        state.record_present(path("a.md"), revision("rev_base"), hash(b"base"));

        let outcome = materializer
            .materialize(&mut state, request("a.md", "rev_remote", b"remote"))
            .unwrap();

        assert!(matches!(
            outcome,
            WorktreeMaterializationOutcome::NeedsImport(WorktreeDeferredMaterialization {
                local_change: WorktreeLocalImportCandidate::Delete { base_revision, .. },
                ..
            }) if base_revision == revision("rev_base")
        ));
        assert!(!target.exists());
    }

    #[test]
    fn already_current_file_updates_state_without_rewrite() {
        let root = TempRoot::new("already-current");
        let config = root.config();
        let target = config.vault_path_to_local(&path("a.md")).unwrap();
        fs::write(&target, b"same").unwrap();
        let materializer = WorktreeMaterializer::new(config.clone());
        let mut state = WorktreeStateSnapshot::new();

        let outcome = materializer
            .materialize(&mut state, request("a.md", "rev_same", b"same"))
            .unwrap();

        assert!(matches!(
            outcome,
            WorktreeMaterializationOutcome::AlreadyCurrent(_)
        ));
        assert!(!config.echo_dir().exists());
        assert!(matches!(
            state.path_state(&path("a.md")),
            Some(WorktreeAppliedPathState::Present(applied))
                if applied.revision_id == revision("rev_same")
        ));
    }

    #[test]
    fn writer_rejects_target_changed_after_planning() {
        let root = TempRoot::new("changed-before-commit");
        let config = root.config();
        let target = config.vault_path_to_local(&path("a.md")).unwrap();
        fs::write(&target, b"changed").unwrap();
        let writer = AtomicWorktreeWriter::new(config);
        let request = request("a.md", "rev_new", b"new");
        let marker = WorktreeEchoMarker {
            vault_path: request.vault_path.clone(),
            revision_id: request.revision_id.clone(),
            content_hash: request.content_hash,
        };

        let error = writer
            .write_materialized_file(
                &request,
                ExpectedTarget::Present(hash(b"old")),
                &marker,
            )
            .unwrap_err();

        assert_eq!(error.code(), "target_changed_before_commit");
        assert_eq!(fs::read(target).unwrap(), b"changed");
    }

    #[test]
    fn cleanup_removes_only_worktree_owned_temp_files() {
        let root = TempRoot::new("temp-cleanup");
        let config = root.config();
        let writer = AtomicWorktreeWriter::new(config.clone());
        writer.ensure_runtime_directory(&[crate::TEMP_DIR_NAME]).unwrap();
        fs::write(config.temp_dir().join(".haze-write-stale.tmp"), b"partial").unwrap();
        fs::write(config.temp_dir().join("keep.txt"), b"keep").unwrap();

        assert_eq!(writer.cleanup_stale_temp_files().unwrap(), 1);
        assert!(!config.temp_dir().join(".haze-write-stale.tmp").exists());
        assert!(config.temp_dir().join("keep.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_target_without_following_it() {
        use std::os::unix::fs::symlink;

        let root = TempRoot::new("symlink-target");
        let config = root.config();
        let outside = root.path.join("outside.txt");
        fs::write(&outside, b"outside").unwrap();
        let target = config.vault_path_to_local(&path("a.md")).unwrap();
        symlink(&outside, &target).unwrap();
        let materializer = WorktreeMaterializer::new(config);
        let mut state = WorktreeStateSnapshot::new();

        let error = materializer
            .materialize(&mut state, request("a.md", "rev_new", b"new"))
            .unwrap_err();

        assert_eq!(error.code(), "target_not_regular_file");
        assert_eq!(fs::read(outside).unwrap(), b"outside");
    }
}
