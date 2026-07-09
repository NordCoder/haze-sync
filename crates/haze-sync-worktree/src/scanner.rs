//! Correctness-oriented filesystem scanner for the local worktree.

use crate::{WorktreeConfig, WorktreePathError};
use haze_sync_common::{ContentHash, VaultPath};
use std::fmt;
use std::fs::{self, File, Metadata};
use std::io::{self, Read};
use std::path::Path;
use std::time::SystemTime;

const READ_BUFFER_SIZE: usize = 8 * 1024;

/// Scanner that walks a configured worktree root and emits stable local file facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeScanner {
    config: WorktreeConfig,
}

impl WorktreeScanner {
    /// Create a scanner for the configured worktree root.
    #[must_use]
    pub fn new(config: WorktreeConfig) -> Self {
        Self { config }
    }

    /// Scan the configured root for stable, safe local files.
    pub fn scan(&self) -> Result<WorktreeScanResult, WorktreeScanError> {
        let root = self.config.root_path();
        let root_metadata =
            fs::symlink_metadata(root).map_err(|_| WorktreeScanError::RootUnavailable)?;
        if !root_metadata.file_type().is_dir() {
            return Err(WorktreeScanError::RootNotDirectory);
        }

        let mut result = WorktreeScanResult::default();
        self.scan_directory(root, &mut result, true)?;
        result.sort_deterministically();
        Ok(result)
    }

    fn scan_directory(
        &self,
        directory: &Path,
        result: &mut WorktreeScanResult,
        is_root: bool,
    ) -> Result<(), WorktreeScanError> {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(_) if is_root => return Err(WorktreeScanError::RootUnreadable),
            Err(_) => {
                result.skipped.push(self.skipped_for_local_path(
                    directory,
                    WorktreeScanSkipReason::FilesystemError,
                ));
                return Ok(());
            }
        };

        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(_) => {
                    result.skipped.push(WorktreeScanSkipped::without_vault_path(
                        WorktreeScanSkipReason::FilesystemError,
                    ));
                    continue;
                }
            };

            self.scan_entry(&path, result)?;
        }

        Ok(())
    }

    fn scan_entry(
        &self,
        path: &Path,
        result: &mut WorktreeScanResult,
    ) -> Result<(), WorktreeScanError> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => {
                result.skipped.push(
                    self.skipped_for_local_path(path, WorktreeScanSkipReason::FilesystemError),
                );
                return Ok(());
            }
        };

        if self.config.is_reserved_local_path(path) {
            result.skipped.push(
                self.skipped_for_local_path(path, WorktreeScanSkipReason::ReservedPath),
            );
            return Ok(());
        }

        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            result
                .skipped
                .push(self.skipped_for_local_path(path, WorktreeScanSkipReason::Symlink));
            return Ok(());
        }

        if file_type.is_dir() {
            return self.scan_directory(path, result, false);
        }

        if file_type.is_file() {
            self.scan_file(path, metadata, result);
            return Ok(());
        }

        result
            .skipped
            .push(self.skipped_for_local_path(path, WorktreeScanSkipReason::SpecialFile));
        Ok(())
    }

    fn scan_file(&self, path: &Path, before_metadata: Metadata, result: &mut WorktreeScanResult) {
        let vault_path = match self.config.local_path_to_vault(path) {
            Ok(vault_path) => vault_path,
            Err(error) => {
                result.skipped.push(WorktreeScanSkipped::without_vault_path(
                    WorktreeScanSkipReason::from_path_error(error),
                ));
                return;
            }
        };

        let before = StableFileObservation::from_metadata(&before_metadata);
        let content_hash = match content_hash_for_path(path) {
            Ok(content_hash) => content_hash,
            Err(_) => {
                result.skipped.push(WorktreeScanSkipped::for_vault_path(
                    vault_path,
                    WorktreeScanSkipReason::ReadError,
                ));
                return;
            }
        };

        let after_metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => {
                result.skipped.push(WorktreeScanSkipped::for_vault_path(
                    vault_path,
                    WorktreeScanSkipReason::FilesystemError,
                ));
                return;
            }
        };

        if !after_metadata.file_type().is_file() {
            result.skipped.push(WorktreeScanSkipped::for_vault_path(
                vault_path,
                WorktreeScanSkipReason::UnstableFile,
            ));
            return;
        }

        let after = StableFileObservation::from_metadata(&after_metadata);
        let stability = StableFileDetector::classify(before, after);
        if !stability.is_stable() {
            result.skipped.push(WorktreeScanSkipped::for_vault_path(
                vault_path,
                WorktreeScanSkipReason::UnstableFile,
            ));
            return;
        }

        result.files.push(WorktreeFileSnapshot {
            vault_path,
            size: after.size,
            modified: after.modified,
            content_hash,
            stability,
        });
    }

    fn skipped_for_local_path(
        &self,
        path: &Path,
        reason: WorktreeScanSkipReason,
    ) -> WorktreeScanSkipped {
        match self.config.local_path_to_vault(path) {
            Ok(vault_path) => WorktreeScanSkipped::for_vault_path(vault_path, reason),
            Err(_) => WorktreeScanSkipped::without_vault_path(reason),
        }
    }
}

/// Scan output containing stable file facts and safe skip diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeScanResult {
    /// Stable files that are safe to submit as local facts in a later import phase.
    pub files: Vec<WorktreeFileSnapshot>,
    /// Entries skipped with safe reason codes and optional vault-relative paths.
    pub skipped: Vec<WorktreeScanSkipped>,
}

impl WorktreeScanResult {
    fn sort_deterministically(&mut self) {
        self.files
            .sort_by(|left, right| left.vault_path.as_str().cmp(right.vault_path.as_str()));
        self.skipped.sort_by(|left, right| {
            left.sort_key()
                .cmp(&right.sort_key())
                .then_with(|| left.reason.cmp(&right.reason))
        });
    }
}

/// Stable file fact produced by a scanner pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeFileSnapshot {
    /// Vault-relative normalized path.
    pub vault_path: VaultPath,
    /// File byte length observed during stable scan.
    pub size: u64,
    /// Last modification timestamp when the platform provides one.
    pub modified: Option<SystemTime>,
    /// SHA-256 content hash of the observed bytes.
    pub content_hash: ContentHash,
    /// Stability classification for the emitted snapshot.
    pub stability: StableFileState,
}

/// Safe skipped-entry diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeScanSkipped {
    /// Vault-relative path when it can be represented safely.
    pub vault_path: Option<VaultPath>,
    /// Safe reason code with no absolute local path or raw filesystem error.
    pub reason: WorktreeScanSkipReason,
}

impl WorktreeScanSkipped {
    fn for_vault_path(vault_path: VaultPath, reason: WorktreeScanSkipReason) -> Self {
        Self {
            vault_path: Some(vault_path),
            reason,
        }
    }

    fn without_vault_path(reason: WorktreeScanSkipReason) -> Self {
        Self {
            vault_path: None,
            reason,
        }
    }

    fn sort_key(&self) -> String {
        self.vault_path
            .as_ref()
            .map_or_else(String::new, |vault_path| vault_path.as_str().to_owned())
    }
}

/// Safe reason codes for skipped filesystem entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorktreeScanSkipReason {
    /// Entry is reserved for Worktree runtime state or temporary writes.
    ReservedPath,
    /// Entry cannot be represented as a safe vault-relative path.
    UnsafePath,
    /// Entry is a symlink and is not followed by the scanner.
    Symlink,
    /// Entry is neither a regular file nor a directory.
    SpecialFile,
    /// Entry could not be inspected through safe filesystem metadata operations.
    FilesystemError,
    /// File contents could not be read for hashing.
    ReadError,
    /// File metadata changed while being read.
    UnstableFile,
}

impl WorktreeScanSkipReason {
    fn from_path_error(error: WorktreePathError) -> Self {
        match error {
            WorktreePathError::ReservedRuntimePath => Self::ReservedPath,
            WorktreePathError::EmptyRoot
            | WorktreePathError::RelativeRoot
            | WorktreePathError::RootHasNoNormalSegment
            | WorktreePathError::UnsafeRootComponent
            | WorktreePathError::LocalPathOutsideRoot
            | WorktreePathError::LocalPathIsRoot
            | WorktreePathError::UnsafeLocalComponent
            | WorktreePathError::BackslashEscape
            | WorktreePathError::NonUtf8LocalPath
            | WorktreePathError::InvalidVaultPath(_) => Self::UnsafePath,
        }
    }

    /// Stable machine-readable reason code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ReservedPath => "reserved_path",
            Self::UnsafePath => "unsafe_path",
            Self::Symlink => "symlink",
            Self::SpecialFile => "special_file",
            Self::FilesystemError => "filesystem_error",
            Self::ReadError => "read_error",
            Self::UnstableFile => "unstable_file",
        }
    }

    /// Stable human-readable message with no raw filesystem details.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::ReservedPath => "path is reserved for worktree runtime state",
            Self::UnsafePath => "path cannot be represented safely as a vault path",
            Self::Symlink => "symlink entries are not followed by the worktree scanner",
            Self::SpecialFile => "special filesystem entries are not imported",
            Self::FilesystemError => "filesystem entry could not be inspected safely",
            Self::ReadError => "file bytes could not be read safely",
            Self::UnstableFile => "file changed while being scanned",
        }
    }
}

/// Safe scanner-level failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeScanError {
    /// The configured root could not be inspected.
    RootUnavailable,
    /// The configured root is not a directory.
    RootNotDirectory,
    /// The configured root could not be read as a directory.
    RootUnreadable,
}

impl WorktreeScanError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RootUnavailable => "root_unavailable",
            Self::RootNotDirectory => "root_not_directory",
            Self::RootUnreadable => "root_unreadable",
        }
    }

    /// Stable human-readable error message with no absolute local root path.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::RootUnavailable => "worktree root could not be inspected",
            Self::RootNotDirectory => "worktree root must be a directory",
            Self::RootUnreadable => "worktree root could not be read",
        }
    }
}

impl fmt::Display for WorktreeScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for WorktreeScanError {}

/// Metadata observation used by the deterministic stable-file heuristic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StableFileObservation {
    /// Observed byte length.
    pub size: u64,
    /// Observed modification timestamp when available.
    pub modified: Option<SystemTime>,
}

impl StableFileObservation {
    fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            size: metadata.len(),
            modified: metadata.modified().ok(),
        }
    }
}

/// Stability classification for a file observed during scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StableFileState {
    metadata_unchanged: bool,
}

impl StableFileState {
    /// Stable when the same metadata was observed before and after hashing.
    #[must_use]
    pub const fn stable() -> Self {
        Self {
            metadata_unchanged: true,
        }
    }

    /// Unstable when metadata changed during hashing.
    #[must_use]
    pub const fn unstable() -> Self {
        Self {
            metadata_unchanged: false,
        }
    }

    /// Returns true when the file was stable enough to emit as a scan fact.
    #[must_use]
    pub const fn is_stable(self) -> bool {
        self.metadata_unchanged
    }
}

/// Deterministic stable-file detector.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StableFileDetector;

impl StableFileDetector {
    /// Classify stability from metadata observed before and after hashing.
    #[must_use]
    pub fn classify(before: StableFileObservation, after: StableFileObservation) -> StableFileState {
        if before == after {
            StableFileState::stable()
        } else {
            StableFileState::unstable()
        }
    }
}

fn content_hash_for_path(path: &Path) -> io::Result<ContentHash> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256State::new();
    let mut buffer = [0_u8; READ_BUFFER_SIZE];

    loop {
        match file.read(&mut buffer) {
            Ok(0) => return Ok(ContentHash::from_bytes(hasher.finalize())),
            Ok(bytes_read) => hasher.update(&buffer[..bytes_read]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
fn content_hash_for_bytes(bytes: &[u8]) -> ContentHash {
    let mut hasher = Sha256State::new();
    hasher.update(bytes);
    ContentHash::from_bytes(hasher.finalize())
}

#[derive(Debug, Clone)]
struct Sha256State {
    state: [u32; 8],
    message_len: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Sha256State {
    const INITIAL_STATE: [u32; 8] = [
        0x6a09e667,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];

    const ROUND_CONSTANTS: [u32; 64] = [
        0x428a2f98,
        0x71374491,
        0xb5c0fbcf,
        0xe9b5dba5,
        0x3956c25b,
        0x59f111f1,
        0x923f82a4,
        0xab1c5ed5,
        0xd807aa98,
        0x12835b01,
        0x243185be,
        0x550c7dc3,
        0x72be5d74,
        0x80deb1fe,
        0x9bdc06a7,
        0xc19bf174,
        0xe49b69c1,
        0xefbe4786,
        0x0fc19dc6,
        0x240ca1cc,
        0x2de92c6f,
        0x4a7484aa,
        0x5cb0a9dc,
        0x76f988da,
        0x983e5152,
        0xa831c66d,
        0xb00327c8,
        0xbf597fc7,
        0xc6e00bf3,
        0xd5a79147,
        0x06ca6351,
        0x14292967,
        0x27b70a85,
        0x2e1b2138,
        0x4d2c6dfc,
        0x53380d13,
        0x650a7354,
        0x766a0abb,
        0x81c2c92e,
        0x92722c85,
        0xa2bfe8a1,
        0xa81a664b,
        0xc24b8b70,
        0xc76c51a3,
        0xd192e819,
        0xd6990624,
        0xf40e3585,
        0x106aa070,
        0x19a4c116,
        0x1e376c08,
        0x2748774c,
        0x34b0bcb5,
        0x391c0cb3,
        0x4ed8aa4a,
        0x5b9cca4f,
        0x682e6ff3,
        0x748f82ee,
        0x78a5636f,
        0x84c87814,
        0x8cc70208,
        0x90befffa,
        0xa4506ceb,
        0xbef9a3f7,
        0xc67178f2,
    ];

    fn new() -> Self {
        Self {
            state: Self::INITIAL_STATE,
            message_len: 0,
            buffer: [0; 64],
            buffer_len: 0,
        }
    }

    fn update(&mut self, mut input: &[u8]) {
        self.message_len = self.message_len.wrapping_add(input.len() as u64);

        while !input.is_empty() {
            let available = self.buffer.len() - self.buffer_len;
            let copy_len = available.min(input.len());
            self.buffer[self.buffer_len..self.buffer_len + copy_len]
                .copy_from_slice(&input[..copy_len]);
            self.buffer_len += copy_len;
            input = &input[copy_len..];

            if self.buffer_len == self.buffer.len() {
                let block = self.buffer;
                self.process_block(&block);
                self.buffer_len = 0;
            }
        }
    }

    fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.message_len.wrapping_mul(8);
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        if self.buffer_len > 56 {
            for index in self.buffer_len..self.buffer.len() {
                self.buffer[index] = 0;
            }
            let block = self.buffer;
            self.process_block(&block);
            self.buffer = [0; 64];
            self.buffer_len = 0;
        }

        for index in self.buffer_len..56 {
            self.buffer[index] = 0;
        }
        self.buffer[56..].copy_from_slice(&bit_len.to_be_bytes());
        let block = self.buffer;
        self.process_block(&block);

        let mut output = [0_u8; 32];
        for (index, word) in self.state.iter().enumerate() {
            output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        output
    }

    fn process_block(&mut self, block: &[u8; 64]) {
        let mut schedule = [0_u32; 64];
        for index in 0..16 {
            let offset = index * 4;
            schedule[index] = u32::from_be_bytes([
                block[offset],
                block[offset + 1],
                block[offset + 2],
                block[offset + 3],
            ]);
        }
        for index in 16..64 {
            schedule[index] = small_sigma_1(schedule[index - 2])
                .wrapping_add(schedule[index - 7])
                .wrapping_add(small_sigma_0(schedule[index - 15]))
                .wrapping_add(schedule[index - 16]);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        for (index, word) in schedule.iter().enumerate() {
            let temp1 = h
                .wrapping_add(big_sigma_1(e))
                .wrapping_add(ch(e, f, g))
                .wrapping_add(Self::ROUND_CONSTANTS[index])
                .wrapping_add(*word);
            let temp2 = big_sigma_0(a).wrapping_add(maj(a, b, c));
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

const fn ch(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}

const fn maj(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}

const fn big_sigma_0(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}

const fn big_sigma_1(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}

const fn small_sigma_0(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}

const fn small_sigma_1(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::ErrorKind;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ROOT_ID: AtomicU64 = AtomicU64::new(0);

    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new(name: &str) -> Self {
            let id = NEXT_TEMP_ROOT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "haze-sync-worktree-{name}-{}-{id}",
                std::process::id()
            ));
            remove_dir_if_exists(&path);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
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
            Err(error) => panic!("failed to clear temp test root: {error}"),
        }
    }

    fn scanner(root: &TempRoot) -> WorktreeScanner {
        WorktreeScanner::new(WorktreeConfig::new(root.path().to_path_buf()).unwrap())
    }

    #[test]
    fn hashes_bytes_as_sha256() {
        assert_eq!(
            content_hash_for_bytes(b"abc").to_string(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn scans_stable_files_with_vault_path_metadata_and_hash() {
        let root = TempRoot::new("stable-files");
        fs::create_dir_all(root.path().join("Notes")).unwrap();
        fs::write(root.path().join("Notes/a.md"), b"abc").unwrap();

        let result = scanner(&root).scan().unwrap();

        assert_eq!(result.skipped, Vec::new());
        assert_eq!(result.files.len(), 1);
        let snapshot = &result.files[0];
        assert_eq!(snapshot.vault_path.as_str(), "Notes/a.md");
        assert_eq!(snapshot.size, 3);
        assert_eq!(
            snapshot.content_hash.to_string(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(snapshot.stability.is_stable());
    }

    #[test]
    fn skips_reserved_runtime_dirs_and_temp_files() {
        let root = TempRoot::new("reserved-paths");
        fs::create_dir_all(root.path().join("_haze_runtime/tmp")).unwrap();
        fs::create_dir_all(root.path().join("Notes")).unwrap();
        fs::write(root.path().join("_haze_runtime/tmp/write.tmp"), b"ignored").unwrap();
        fs::write(root.path().join("Notes/draft.swp"), b"ignored").unwrap();
        fs::write(root.path().join("Notes/ok.md"), b"ok").unwrap();

        let result = scanner(&root).scan().unwrap();

        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].vault_path.as_str(), "Notes/ok.md");
        assert_eq!(
            result
                .skipped
                .iter()
                .filter(|skipped| skipped.reason == WorktreeScanSkipReason::ReservedPath)
                .count(),
            2
        );
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlinks_without_following_them() {
        use std::os::unix::fs::symlink;

        let root = TempRoot::new("symlink");
        fs::write(root.path().join("target.md"), b"target").unwrap();
        symlink(root.path().join("target.md"), root.path().join("link.md")).unwrap();

        let result = scanner(&root).scan().unwrap();

        assert!(result.skipped.iter().any(|skipped| {
            skipped.reason == WorktreeScanSkipReason::Symlink
                && skipped
                    .vault_path
                    .as_ref()
                    .is_some_and(|path| path.as_str() == "link.md")
        }));
    }

    #[test]
    fn stable_detector_rejects_changed_metadata() {
        let before = StableFileObservation {
            size: 3,
            modified: Some(SystemTime::UNIX_EPOCH),
        };
        let after = StableFileObservation {
            size: 4,
            modified: Some(SystemTime::UNIX_EPOCH),
        };

        assert!(!StableFileDetector::classify(before, after).is_stable());
    }

    #[test]
    fn root_errors_do_not_expose_local_paths() {
        let root = TempRoot::new("root-error");
        let file_root = root.path().join("not-a-directory");
        fs::write(&file_root, b"not a directory").unwrap();

        let error = WorktreeScanner::new(WorktreeConfig::new(file_root).unwrap())
            .scan()
            .unwrap_err();

        assert_eq!(error.code(), "root_not_directory");
        assert!(!error.to_string().contains("haze-sync-worktree"));
    }
}
