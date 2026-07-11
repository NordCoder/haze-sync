//! Safe path mapping between Core `VaultPath` values and local worktree paths.

use haze_sync_common::{ValidationError, VaultPath};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Top-level runtime directory reserved inside the configured worktree root.
pub const WORKTREE_RUNTIME_DIR_NAME: &str = "_haze_runtime";

/// Runtime subdirectory for temporary adapter writes.
pub const TEMP_DIR_NAME: &str = "tmp";

/// Runtime subdirectory for retained local trash/backup files.
pub const TRASH_DIR_NAME: &str = "trash";

/// Runtime subdirectory for local metadata owned by the worktree adapter.
pub const METADATA_DIR_NAME: &str = "metadata";

/// Runtime subdirectory for echo-guard state owned by the worktree adapter.
pub const ECHO_DIR_NAME: &str = "echo";

const RESERVED_TOP_LEVEL_DIRS: &[&str] = &[
    WORKTREE_RUNTIME_DIR_NAME,
    "_haze_tmp",
    "state",
    "logs",
    "trash",
];

const RESERVED_FILE_SUFFIXES: &[&str] = &[".tmp", ".part", ".swp"];

/// Safe configuration for mapping vault paths under one local worktree root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeConfig {
    root: PathBuf,
}

impl WorktreeConfig {
    /// Create a worktree configuration from an absolute root path.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, WorktreePathError> {
        let root = root.into();
        validate_root(&root)?;
        Ok(Self { root })
    }

    pub(crate) fn root_path(&self) -> &Path {
        &self.root
    }

    /// Map a validated `VaultPath` to a local path under the configured root.
    ///
    /// When the root exists, this also rejects an existing symlink or
    /// non-directory anywhere in the configured root or vault-parent chain
    /// before callers can use the returned path for filesystem access.
    pub fn vault_path_to_local(
        &self,
        vault_path: &VaultPath,
    ) -> Result<PathBuf, WorktreePathError> {
        if vault_path.is_reserved_runtime_path() {
            return Err(WorktreePathError::ReservedRuntimePath);
        }

        let segments: Vec<&str> = vault_path.segments().collect();
        let mut local_path = self.root.clone();
        for segment in &segments {
            if segment.contains('\\') {
                return Err(WorktreePathError::BackslashEscape);
            }
            local_path.push(segment);
        }

        if !local_path.starts_with(&self.root) {
            return Err(WorktreePathError::LocalPathOutsideRoot);
        }

        self.validate_existing_parent_chain(&segments)?;
        Ok(local_path)
    }

    /// Map a local path back to a `VaultPath` only when it is under the configured root.
    pub fn local_path_to_vault(
        &self,
        local_path: impl AsRef<Path>,
    ) -> Result<VaultPath, WorktreePathError> {
        let relative = self.relative_path_under_root(local_path.as_ref())?;
        let segments = relative_segments(relative)?;

        if is_reserved_relative_segments(&segments) {
            return Err(WorktreePathError::ReservedRuntimePath);
        }

        VaultPath::parse(&segments.join("/")).map_err(WorktreePathError::InvalidVaultPath)
    }

    /// Returns true when a local path is under the reserved worktree runtime area.
    pub fn is_reserved_local_path(&self, local_path: impl AsRef<Path>) -> bool {
        let Ok(relative) = self.relative_path_under_root(local_path.as_ref()) else {
            return false;
        };
        let Ok(segments) = relative_segments(relative) else {
            return false;
        };
        is_reserved_relative_segments(&segments)
    }

    /// Path to the reserved runtime directory under the configured root.
    #[must_use]
    pub fn runtime_dir(&self) -> PathBuf {
        self.root.join(WORKTREE_RUNTIME_DIR_NAME)
    }

    /// Path to the reserved temp-write directory under the runtime directory.
    #[must_use]
    pub fn temp_dir(&self) -> PathBuf {
        self.runtime_dir().join(TEMP_DIR_NAME)
    }

    /// Path to the reserved local trash/backup directory under the runtime directory.
    #[must_use]
    pub fn trash_dir(&self) -> PathBuf {
        self.runtime_dir().join(TRASH_DIR_NAME)
    }

    /// Path to the reserved metadata directory under the runtime directory.
    #[must_use]
    pub fn metadata_dir(&self) -> PathBuf {
        self.runtime_dir().join(METADATA_DIR_NAME)
    }

    /// Path to the reserved echo-guard directory under the runtime directory.
    #[must_use]
    pub fn echo_dir(&self) -> PathBuf {
        self.runtime_dir().join(ECHO_DIR_NAME)
    }

    fn relative_path_under_root<'a>(
        &self,
        local_path: &'a Path,
    ) -> Result<&'a Path, WorktreePathError> {
        local_path
            .strip_prefix(&self.root)
            .map_err(|_| WorktreePathError::LocalPathOutsideRoot)
    }

    fn validate_existing_parent_chain(&self, segments: &[&str]) -> Result<(), WorktreePathError> {
        if !existing_root_chain_is_safe(&self.root)? {
            return Ok(());
        }

        let mut current = self.root.clone();
        for segment in segments.iter().take(segments.len().saturating_sub(1)) {
            current.push(segment);
            if !existing_directory_is_safe(&current)? {
                return Ok(());
            }
        }

        Ok(())
    }
}

/// Safe, path-redacted failures from worktree configuration and path mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreePathError {
    /// The configured worktree root was empty.
    EmptyRoot,
    /// The configured worktree root was not absolute.
    RelativeRoot,
    /// The configured worktree root did not contain a normal path segment.
    RootHasNoNormalSegment,
    /// The configured worktree root contained an unsafe path component.
    UnsafeRootComponent,
    /// The local path is not under the configured worktree root.
    LocalPathOutsideRoot,
    /// The local path points at the worktree root itself, not a file path.
    LocalPathIsRoot,
    /// The local path contained an unsafe, symlinked, or non-directory component.
    UnsafeLocalComponent,
    /// A path used a backslash escape where vault-relative paths require `/` separators.
    BackslashEscape,
    /// A local path segment could not be represented as UTF-8.
    NonUtf8LocalPath,
    /// The path is reserved for worktree runtime state and must not sync.
    ReservedRuntimePath,
    /// The resulting vault path failed shared `VaultPath` validation.
    InvalidVaultPath(ValidationError),
}

impl WorktreePathError {
    /// Stable machine-readable error code with no local path details.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyRoot => "empty_root",
            Self::RelativeRoot => "relative_root",
            Self::RootHasNoNormalSegment => "root_has_no_normal_segment",
            Self::UnsafeRootComponent => "unsafe_root_component",
            Self::LocalPathOutsideRoot => "local_path_outside_root",
            Self::LocalPathIsRoot => "local_path_is_root",
            Self::UnsafeLocalComponent => "unsafe_local_component",
            Self::BackslashEscape => "backslash_escape",
            Self::NonUtf8LocalPath => "non_utf8_local_path",
            Self::ReservedRuntimePath => "reserved_runtime_path",
            Self::InvalidVaultPath(error) => error.code(),
        }
    }

    /// Stable human-readable message with no absolute local paths or raw filesystem errors.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::EmptyRoot => "worktree root must not be empty",
            Self::RelativeRoot => "worktree root must be absolute",
            Self::RootHasNoNormalSegment => "worktree root must contain a normal path segment",
            Self::UnsafeRootComponent => "worktree root contains unsafe path components",
            Self::LocalPathOutsideRoot => "local path is outside the worktree root",
            Self::LocalPathIsRoot => "local path must refer to an item below the worktree root",
            Self::UnsafeLocalComponent => "local path contains unsafe components",
            Self::BackslashEscape => "path must not use backslash separators",
            Self::NonUtf8LocalPath => "local path must be valid UTF-8 before vault mapping",
            Self::ReservedRuntimePath => {
                "path is reserved for worktree runtime state and must not sync"
            }
            Self::InvalidVaultPath(error) => error.message(),
        }
    }
}

impl fmt::Display for WorktreePathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for WorktreePathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidVaultPath(error) => Some(error),
            _ => None,
        }
    }
}

fn validate_root(root: &Path) -> Result<(), WorktreePathError> {
    if root.as_os_str().is_empty() {
        return Err(WorktreePathError::EmptyRoot);
    }

    if !root.is_absolute() {
        return Err(WorktreePathError::RelativeRoot);
    }

    let mut has_normal_segment = false;
    for component in root.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {}
            Component::Normal(_) => has_normal_segment = true,
            Component::CurDir | Component::ParentDir => {
                return Err(WorktreePathError::UnsafeRootComponent);
            }
        }
    }

    if !has_normal_segment {
        return Err(WorktreePathError::RootHasNoNormalSegment);
    }

    Ok(())
}

fn existing_root_chain_is_safe(root: &Path) -> Result<bool, WorktreePathError> {
    let mut ancestors: Vec<&Path> = root
        .ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .collect();
    ancestors.reverse();

    for ancestor in ancestors {
        if !existing_directory_is_safe(ancestor)? {
            return Ok(false);
        }
    }

    Ok(true)
}

fn existing_directory_is_safe(path: &Path) -> Result<bool, WorktreePathError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                return Err(WorktreePathError::UnsafeLocalComponent);
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(WorktreePathError::UnsafeLocalComponent),
    }
}

fn relative_segments(relative: &Path) -> Result<Vec<String>, WorktreePathError> {
    let mut segments = Vec::new();

    for component in relative.components() {
        match component {
            Component::Normal(segment) => {
                let segment = segment
                    .to_str()
                    .ok_or(WorktreePathError::NonUtf8LocalPath)?;
                if segment.contains('\\') {
                    return Err(WorktreePathError::BackslashEscape);
                }
                segments.push(segment.to_owned());
            }
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(WorktreePathError::UnsafeLocalComponent);
            }
        }
    }

    if segments.is_empty() {
        return Err(WorktreePathError::LocalPathIsRoot);
    }

    Ok(segments)
}

fn is_reserved_relative_segments(segments: &[String]) -> bool {
    let Some(first_segment) = segments.first() else {
        return false;
    };

    RESERVED_TOP_LEVEL_DIRS.contains(&first_segment.as_str())
        || segments.last().is_some_and(|last_segment| {
            RESERVED_FILE_SUFFIXES
                .iter()
                .any(|suffix| last_segment.ends_with(suffix))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ROOT_ID: AtomicU64 = AtomicU64::new(0);

    fn config() -> WorktreeConfig {
        WorktreeConfig::new("/srv/haze-vault/worktree").unwrap()
    }

    fn temp_root(name: &str) -> PathBuf {
        let id = NEXT_TEMP_ROOT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "haze-sync-path-mapping-{name}-{}-{id}",
            std::process::id()
        ))
    }

    fn remove_dir_if_exists(path: &Path) {
        match fs::remove_dir_all(path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => panic!("failed to clear path-mapping test root: {error}"),
        }
    }

    #[test]
    fn creates_config_for_absolute_root() {
        let config = config();

        assert_eq!(
            config.runtime_dir(),
            PathBuf::from("/srv/haze-vault/worktree/_haze_runtime")
        );
        assert_eq!(
            config.temp_dir(),
            PathBuf::from("/srv/haze-vault/worktree/_haze_runtime/tmp")
        );
        assert_eq!(
            config.trash_dir(),
            PathBuf::from("/srv/haze-vault/worktree/_haze_runtime/trash")
        );
        assert_eq!(
            config.metadata_dir(),
            PathBuf::from("/srv/haze-vault/worktree/_haze_runtime/metadata")
        );
        assert_eq!(
            config.echo_dir(),
            PathBuf::from("/srv/haze-vault/worktree/_haze_runtime/echo")
        );
    }

    #[test]
    fn rejects_unsafe_roots() {
        assert_eq!(
            WorktreeConfig::new("").unwrap_err(),
            WorktreePathError::EmptyRoot
        );
        assert_eq!(
            WorktreeConfig::new("relative/root").unwrap_err(),
            WorktreePathError::RelativeRoot
        );
        assert_eq!(
            WorktreeConfig::new("/").unwrap_err(),
            WorktreePathError::RootHasNoNormalSegment
        );
    }

    #[test]
    fn maps_vault_path_to_local_under_root() {
        let path = VaultPath::parse("Notes/a.md").unwrap();

        assert_eq!(
            config().vault_path_to_local(&path).unwrap(),
            PathBuf::from("/srv/haze-vault/worktree/Notes/a.md")
        );
    }

    #[test]
    fn maps_normalized_vault_path_to_local() {
        let path = VaultPath::parse("./Notes//./a.md").unwrap();

        assert_eq!(
            config().vault_path_to_local(&path).unwrap(),
            PathBuf::from("/srv/haze-vault/worktree/Notes/a.md")
        );
    }

    #[test]
    fn maps_local_path_inside_root_back_to_vault_path() {
        let path = config()
            .local_path_to_vault("/srv/haze-vault/worktree/Notes/a.md")
            .unwrap();

        assert_eq!(path.as_str(), "Notes/a.md");
    }

    #[test]
    fn rejects_local_path_outside_root() {
        assert_eq!(
            config()
                .local_path_to_vault("/srv/haze-vault/worktree-evil/Notes/a.md")
                .unwrap_err(),
            WorktreePathError::LocalPathOutsideRoot
        );
        assert_eq!(
            config()
                .local_path_to_vault("/srv/haze-vault/other/Notes/a.md")
                .unwrap_err(),
            WorktreePathError::LocalPathOutsideRoot
        );
    }

    #[test]
    fn rejects_root_itself_as_vault_path() {
        assert_eq!(
            config()
                .local_path_to_vault("/srv/haze-vault/worktree")
                .unwrap_err(),
            WorktreePathError::LocalPathIsRoot
        );
    }

    #[test]
    fn rejects_traversal_after_root_prefix() {
        assert_eq!(
            config()
                .local_path_to_vault("/srv/haze-vault/worktree/Notes/../a.md")
                .unwrap_err(),
            WorktreePathError::UnsafeLocalComponent
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_backslash_escape_in_local_segment() {
        assert_eq!(
            config()
                .local_path_to_vault("/srv/haze-vault/worktree/Notes\\a.md")
                .unwrap_err(),
            WorktreePathError::BackslashEscape
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_existing_symlink_in_configured_root_chain() {
        use std::os::unix::fs::symlink;

        let test_root = temp_root("symlink-root");
        let outside_root = test_root.join("outside");
        remove_dir_if_exists(&test_root);
        fs::create_dir_all(outside_root.join("worktree")).unwrap();
        symlink(&outside_root, test_root.join("root-link")).unwrap();
        let config = WorktreeConfig::new(test_root.join("root-link/worktree")).unwrap();
        let vault_path = VaultPath::parse("a.md").unwrap();

        assert_eq!(
            config.vault_path_to_local(&vault_path).unwrap_err(),
            WorktreePathError::UnsafeLocalComponent
        );

        remove_dir_if_exists(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_existing_symlink_in_vault_parent_chain() {
        use std::os::unix::fs::symlink;

        let test_root = temp_root("symlink-parent");
        let worktree_root = test_root.join("worktree");
        let outside_root = test_root.join("outside");
        remove_dir_if_exists(&test_root);
        fs::create_dir_all(&worktree_root).unwrap();
        fs::create_dir_all(&outside_root).unwrap();
        symlink(&outside_root, worktree_root.join("Notes")).unwrap();
        let config = WorktreeConfig::new(worktree_root).unwrap();
        let vault_path = VaultPath::parse("Notes/a.md").unwrap();

        assert_eq!(
            config.vault_path_to_local(&vault_path).unwrap_err(),
            WorktreePathError::UnsafeLocalComponent
        );

        remove_dir_if_exists(&test_root);
    }

    #[test]
    fn rejects_reserved_runtime_paths() {
        assert_eq!(
            config()
                .local_path_to_vault("/srv/haze-vault/worktree/_haze_runtime/tmp/write.tmp")
                .unwrap_err(),
            WorktreePathError::ReservedRuntimePath
        );
        assert_eq!(
            config()
                .local_path_to_vault("/srv/haze-vault/worktree/Notes/upload.part")
                .unwrap_err(),
            WorktreePathError::ReservedRuntimePath
        );
    }

    #[test]
    fn recognizes_reserved_local_paths() {
        let config = config();

        assert!(config.is_reserved_local_path("/srv/haze-vault/worktree/_haze_runtime/echo/marker"));
        assert!(config.is_reserved_local_path("/srv/haze-vault/worktree/Notes/draft.swp"));
        assert!(!config.is_reserved_local_path("/srv/haze-vault/worktree/Notes/a.md"));
        assert!(!config.is_reserved_local_path("/srv/haze-vault/other/Notes/a.md"));
    }

    #[test]
    fn preserves_core_conflict_materialization_paths() {
        let path = VaultPath::parse("_haze_conflicts/open/Projects/Haze/plan.md").unwrap();

        assert_eq!(
            config().vault_path_to_local(&path).unwrap(),
            PathBuf::from("/srv/haze-vault/worktree/_haze_conflicts/open/Projects/Haze/plan.md")
        );
    }

    #[test]
    fn errors_do_not_expose_configured_root() {
        let error = config()
            .local_path_to_vault("/srv/haze-vault/worktree-evil/Notes/a.md")
            .unwrap_err();

        assert_eq!(error.code(), "local_path_outside_root");
        assert!(!error.to_string().contains("/srv/haze-vault"));
    }
}
