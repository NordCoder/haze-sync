//! Bounded echo suppression for files written by the Worktree materializer.

use crate::materializer::WorktreeEchoMarker;
use crate::WorktreeConfig;
use haze_sync_common::{ContentHash, RevisionId, VaultPath};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const ECHO_MARKER_SUFFIX: &str = ".echo";
const MAX_MARKER_BYTES: u64 = 4 * 1024;

/// Bounded lifetime and capacity policy for echo suppression state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeEchoGuardPolicy {
    ttl: Duration,
    max_entries: usize,
}

impl WorktreeEchoGuardPolicy {
    /// Build a policy with a non-zero lifetime and capacity.
    pub fn new(ttl: Duration, max_entries: usize) -> Result<Self, WorktreeEchoGuardError> {
        if ttl.is_zero() || max_entries == 0 {
            return Err(WorktreeEchoGuardError::InvalidPolicy);
        }
        Ok(Self { ttl, max_entries })
    }

    /// Maximum age of one adapter-write marker.
    #[must_use]
    pub const fn ttl(self) -> Duration {
        self.ttl
    }

    /// Maximum number of paths retained by the guard.
    #[must_use]
    pub const fn max_entries(self) -> usize {
        self.max_entries
    }
}

/// Outcome of checking one filesystem observation against the last adapter write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeEchoDecision {
    /// No marker existed for the path.
    NoMarker,
    /// The observation exactly matched the bounded marker and the marker was consumed.
    Suppressed(WorktreeEchoMarker),
    /// A marker existed but did not match current authoritative and filesystem facts.
    Stale(WorktreeEchoMarker),
}

/// Summary of markers loaded from the reserved Worktree runtime directory.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WorktreeEchoLoadSummary {
    /// Valid, non-expired markers retained by the guard.
    pub loaded: usize,
    /// Expired markers removed from guard/runtime state.
    pub expired: usize,
    /// Malformed or unsafe marker files ignored and removed when safe.
    pub invalid: usize,
    /// Older markers removed because a newer path marker or capacity bound won.
    pub evicted: usize,
}

/// Summary of recording one in-memory adapter write.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WorktreeEchoRecordSummary {
    /// True when an older marker for the same path was replaced.
    pub replaced: bool,
    /// Entries removed because they had expired.
    pub expired: usize,
    /// Entries removed to preserve the configured capacity bound.
    pub evicted: usize,
}

/// Safe failures while loading or consuming Worktree-owned echo markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeEchoGuardError {
    /// Policy used a zero lifetime or zero capacity.
    InvalidPolicy,
    /// The configured root or runtime directory chain was unsafe.
    UnsafeRuntimeDirectory,
    /// The reserved echo directory could not be enumerated safely.
    RuntimeDirectoryReadFailed,
    /// A marker file could not be read safely.
    MarkerReadFailed,
    /// A consumed, stale, expired, or invalid marker could not be removed.
    MarkerCleanupFailed,
}

impl WorktreeEchoGuardError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidPolicy => "invalid_echo_policy",
            Self::UnsafeRuntimeDirectory => "unsafe_echo_runtime_directory",
            Self::RuntimeDirectoryReadFailed => "echo_runtime_directory_read_failed",
            Self::MarkerReadFailed => "echo_marker_read_failed",
            Self::MarkerCleanupFailed => "echo_marker_cleanup_failed",
        }
    }

    /// Stable path-redacted error message.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidPolicy => "echo guard policy requires non-zero lifetime and capacity",
            Self::UnsafeRuntimeDirectory => "echo runtime directory is not safe",
            Self::RuntimeDirectoryReadFailed => "echo runtime directory could not be read",
            Self::MarkerReadFailed => "echo marker could not be read safely",
            Self::MarkerCleanupFailed => "echo marker could not be removed safely",
        }
    }
}

impl fmt::Display for WorktreeEchoGuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for WorktreeEchoGuardError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorktreeEchoEntry {
    marker: WorktreeEchoMarker,
    written_at: SystemTime,
    runtime_path: Option<PathBuf>,
}

/// Narrow, bounded state used to suppress one scanner echo per adapter write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEchoGuard {
    policy: WorktreeEchoGuardPolicy,
    entries: BTreeMap<VaultPath, WorktreeEchoEntry>,
}

impl WorktreeEchoGuard {
    /// Create an empty bounded echo guard.
    #[must_use]
    pub fn new(policy: WorktreeEchoGuardPolicy) -> Self {
        Self {
            policy,
            entries: BTreeMap::new(),
        }
    }

    /// Number of currently retained path markers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no path marker is retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Record a last adapter write directly from a successful materialization outcome.
    pub fn record(
        &mut self,
        marker: WorktreeEchoMarker,
        written_at: SystemTime,
        now: SystemTime,
    ) -> Result<WorktreeEchoRecordSummary, WorktreeEchoGuardError> {
        let expired = self.expire(now)?;
        let replaced = self
            .insert_entry(WorktreeEchoEntry {
                marker,
                written_at,
                runtime_path: None,
            })?
            .is_some();
        let evicted = self.enforce_capacity()?;
        Ok(WorktreeEchoRecordSummary {
            replaced,
            expired,
            evicted,
        })
    }

    /// Load durable marker files written under `_haze_runtime/echo`.
    ///
    /// Marker contents are never authoritative by themselves. Every later
    /// suppression still requires an exact revision/hash match against current
    /// applied state and a stable filesystem observation.
    pub fn load_runtime_markers(
        &mut self,
        config: &WorktreeConfig,
        now: SystemTime,
    ) -> Result<WorktreeEchoLoadSummary, WorktreeEchoGuardError> {
        let mut summary = WorktreeEchoLoadSummary {
            expired: self.expire(now)?,
            ..WorktreeEchoLoadSummary::default()
        };

        if !safe_existing_directory_chain(config.root_path())? {
            return Ok(summary);
        }
        if !safe_existing_directory(&config.runtime_dir())? {
            return Ok(summary);
        }
        if !safe_existing_directory(&config.echo_dir())? {
            return Ok(summary);
        }

        let entries = fs::read_dir(config.echo_dir())
            .map_err(|_| WorktreeEchoGuardError::RuntimeDirectoryReadFailed)?;
        for entry in entries {
            let entry = entry.map_err(|_| WorktreeEchoGuardError::RuntimeDirectoryReadFailed)?;
            let marker_path = entry.path();
            let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !file_name.ends_with(ECHO_MARKER_SUFFIX) {
                continue;
            }

            let metadata = fs::symlink_metadata(&marker_path)
                .map_err(|_| WorktreeEchoGuardError::MarkerReadFailed)?;
            if !metadata.file_type().is_file() || metadata.len() > MAX_MARKER_BYTES {
                remove_marker_file(&marker_path)?;
                summary.invalid += 1;
                continue;
            }

            let bytes = read_bounded_marker(&marker_path)?;
            let after = fs::symlink_metadata(&marker_path)
                .map_err(|_| WorktreeEchoGuardError::MarkerReadFailed)?;
            if !same_marker_observation(&metadata, &after, bytes.len() as u64) {
                remove_marker_file(&marker_path)?;
                summary.invalid += 1;
                continue;
            }

            let Some(marker) = parse_marker(&bytes) else {
                remove_marker_file(&marker_path)?;
                summary.invalid += 1;
                continue;
            };
            let written_at = after.modified().unwrap_or(now);
            if is_expired(self.policy, written_at, now) {
                remove_marker_file(&marker_path)?;
                summary.expired += 1;
                continue;
            }

            let replaced = self.insert_entry(WorktreeEchoEntry {
                marker,
                written_at,
                runtime_path: Some(marker_path),
            })?;
            if replaced.is_some() {
                summary.evicted += 1;
            }
        }

        summary.evicted += self.enforce_capacity()?;
        summary.loaded = self.entries.len();
        Ok(summary)
    }

    /// Expire old markers and remove their Worktree-owned runtime files.
    pub fn expire(&mut self, now: SystemTime) -> Result<usize, WorktreeEchoGuardError> {
        let expired_paths: Vec<VaultPath> = self
            .entries
            .iter()
            .filter_map(|(vault_path, entry)| {
                is_expired(self.policy, entry.written_at, now).then(|| vault_path.clone())
            })
            .collect();

        for vault_path in &expired_paths {
            if let Some(entry) = self.entries.remove(vault_path) {
                cleanup_runtime_marker(&entry)?;
            }
        }
        Ok(expired_paths.len())
    }

    /// Consume the marker for one observed path.
    ///
    /// Suppression occurs only when marker revision, applied hash, and observed
    /// hash all agree. Any mismatching marker is consumed as stale so it cannot
    /// become permanently authoritative.
    pub fn consume_for_observation(
        &mut self,
        vault_path: &VaultPath,
        applied_revision: Option<&RevisionId>,
        applied_hash: Option<ContentHash>,
        observed_hash: Option<ContentHash>,
        now: SystemTime,
    ) -> Result<WorktreeEchoDecision, WorktreeEchoGuardError> {
        self.expire(now)?;
        let Some(entry) = self.entries.remove(vault_path) else {
            return Ok(WorktreeEchoDecision::NoMarker);
        };

        cleanup_runtime_marker(&entry)?;
        let exact_match = applied_revision.is_some_and(|revision| revision == &entry.marker.revision_id)
            && applied_hash == Some(entry.marker.content_hash)
            && observed_hash == Some(entry.marker.content_hash);

        if exact_match {
            Ok(WorktreeEchoDecision::Suppressed(entry.marker))
        } else {
            Ok(WorktreeEchoDecision::Stale(entry.marker))
        }
    }

    fn insert_entry(
        &mut self,
        candidate: WorktreeEchoEntry,
    ) -> Result<Option<WorktreeEchoEntry>, WorktreeEchoGuardError> {
        let vault_path = candidate.marker.vault_path.clone();
        let Some(existing) = self.entries.remove(&vault_path) else {
            self.entries.insert(vault_path, candidate);
            return Ok(None);
        };

        let candidate_is_newer = entry_order_key(&candidate) >= entry_order_key(&existing);
        if candidate_is_newer {
            cleanup_runtime_marker(&existing)?;
            self.entries.insert(vault_path, candidate);
            Ok(Some(existing))
        } else {
            cleanup_runtime_marker(&candidate)?;
            self.entries.insert(vault_path, existing);
            Ok(Some(candidate))
        }
    }

    fn enforce_capacity(&mut self) -> Result<usize, WorktreeEchoGuardError> {
        let mut evicted = 0;
        while self.entries.len() > self.policy.max_entries {
            let oldest_path = self
                .entries
                .iter()
                .min_by(|(left_path, left), (right_path, right)| {
                    entry_order_key(left)
                        .cmp(&entry_order_key(right))
                        .then_with(|| left_path.as_str().cmp(right_path.as_str()))
                })
                .map(|(vault_path, _)| vault_path.clone())
                .expect("capacity overflow requires at least one echo entry");
            let entry = self
                .entries
                .remove(&oldest_path)
                .expect("selected echo entry must still exist");
            cleanup_runtime_marker(&entry)?;
            evicted += 1;
        }
        Ok(evicted)
    }
}

fn entry_order_key(entry: &WorktreeEchoEntry) -> (SystemTime, &str, ContentHash) {
    (
        entry.written_at,
        entry.marker.revision_id.as_str(),
        entry.marker.content_hash,
    )
}

fn is_expired(policy: WorktreeEchoGuardPolicy, written_at: SystemTime, now: SystemTime) -> bool {
    now.duration_since(written_at)
        .is_ok_and(|age| age >= policy.ttl)
}

fn safe_existing_directory_chain(root: &Path) -> Result<bool, WorktreeEchoGuardError> {
    let mut ancestors: Vec<&Path> = root
        .ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .collect();
    ancestors.reverse();

    for ancestor in ancestors {
        if !safe_existing_directory(ancestor)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn safe_existing_directory(path: &Path) -> Result<bool, WorktreeEchoGuardError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                return Err(WorktreeEchoGuardError::UnsafeRuntimeDirectory);
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(WorktreeEchoGuardError::UnsafeRuntimeDirectory),
    }
}

fn read_bounded_marker(path: &Path) -> Result<Vec<u8>, WorktreeEchoGuardError> {
    let file = File::open(path).map_err(|_| WorktreeEchoGuardError::MarkerReadFailed)?;
    let mut bytes = Vec::new();
    file.take(MAX_MARKER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| WorktreeEchoGuardError::MarkerReadFailed)?;
    if bytes.len() as u64 > MAX_MARKER_BYTES {
        return Err(WorktreeEchoGuardError::MarkerReadFailed);
    }
    Ok(bytes)
}

fn same_marker_observation(before: &fs::Metadata, after: &fs::Metadata, bytes_read: u64) -> bool {
    before.file_type().is_file()
        && after.file_type().is_file()
        && before.len() == after.len()
        && after.len() == bytes_read
        && before.modified().ok() == after.modified().ok()
}

fn parse_marker(bytes: &[u8]) -> Option<WorktreeEchoMarker> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut version = None;
    let mut vault_path = None;
    let mut revision_id = None;
    let mut content_hash = None;

    for line in text.lines() {
        let (key, value) = line.split_once('=')?;
        match key {
            "version" if version.replace(value).is_none() => {}
            "path" if vault_path.replace(value).is_none() => {}
            "revision" if revision_id.replace(value).is_none() => {}
            "content_hash" if content_hash.replace(value).is_none() => {}
            _ => return None,
        }
    }

    if version != Some("1") {
        return None;
    }
    Some(WorktreeEchoMarker {
        vault_path: VaultPath::parse(vault_path?).ok()?,
        revision_id: RevisionId::parse(revision_id?).ok()?,
        content_hash: ContentHash::parse(content_hash?).ok()?,
    })
}

fn cleanup_runtime_marker(entry: &WorktreeEchoEntry) -> Result<(), WorktreeEchoGuardError> {
    if let Some(runtime_path) = &entry.runtime_path {
        remove_marker_file(runtime_path)?;
    }
    Ok(())
}

fn remove_marker_file(path: &Path) -> Result<(), WorktreeEchoGuardError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(WorktreeEchoGuardError::MarkerCleanupFailed),
    }
}
