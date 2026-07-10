//! Adapter-local mapping, cursor, and echo-state model.
//!
//! This module defines the state shape the adapter needs before a concrete
//! persistence mechanism is accepted. It intentionally contains no database
//! access, no provider calls, no Core/API writes, and no raw provider cursor
//! payloads. Durable persistence remains a Server/API or accepted repository
//! fan-in decision outside this module.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateError {
    EmptyField(&'static str),
    InvalidVaultPath(&'static str),
    DriveFileIdMismatch,
    CursorRegression { current: u64, attempted: u64 },
    DirectDatabaseAccessNotAccepted,
}

impl StateError {
    fn empty(field: &'static str) -> Self {
        Self::EmptyField(field)
    }
}

impl fmt::Display for StateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(formatter, "state field {field} must not be empty"),
            Self::InvalidVaultPath(reason) => write!(formatter, "invalid vault path: {reason}"),
            Self::DriveFileIdMismatch => {
                formatter.write_str("Drive observation does not match mapping identity")
            }
            Self::CursorRegression { current, attempted } => write!(
                formatter,
                "cursor regression rejected: current={current}, attempted={attempted}"
            ),
            Self::DirectDatabaseAccessNotAccepted => formatter
                .write_str("direct database access is not accepted for gdrive-adapter state"),
        }
    }
}

impl Error for StateError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SafeTimestamp(String);

impl SafeTimestamp {
    pub fn new(raw: impl Into<String>) -> Result<Self, StateError> {
        let value = raw.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(StateError::empty("timestamp"));
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for SafeTimestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VaultPath(String);

impl VaultPath {
    pub fn new(raw: impl Into<String>) -> Result<Self, StateError> {
        let value = raw.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(StateError::InvalidVaultPath("path must not be empty"));
        }
        if trimmed.starts_with('/') {
            return Err(StateError::InvalidVaultPath("path must be vault-relative"));
        }
        if trimmed.contains('\\') {
            return Err(StateError::InvalidVaultPath(
                "path must use forward slashes",
            ));
        }
        if trimmed
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        {
            return Err(StateError::InvalidVaultPath(
                "path must not contain empty, current, or parent segments",
            ));
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for VaultPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveStateObservation {
    pub drive_file_id: String,
    pub parent_id: String,
    pub name: String,
    pub checksum: Option<String>,
    pub drive_version: Option<String>,
    pub drive_modified_time: Option<SafeTimestamp>,
    pub observed_at: SafeTimestamp,
}

impl DriveStateObservation {
    pub fn new(
        drive_file_id: impl Into<String>,
        parent_id: impl Into<String>,
        name: impl Into<String>,
        observed_at: SafeTimestamp,
    ) -> Result<Self, StateError> {
        Ok(Self {
            drive_file_id: required_string("drive_file_id", drive_file_id)?,
            parent_id: required_string("parent_id", parent_id)?,
            name: required_string("name", name)?,
            checksum: None,
            drive_version: None,
            drive_modified_time: None,
            observed_at,
        })
    }

    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Result<Self, StateError> {
        self.checksum = Some(required_string("checksum", checksum)?);
        Ok(self)
    }

    pub fn with_drive_version(
        mut self,
        drive_version: impl Into<String>,
    ) -> Result<Self, StateError> {
        self.drive_version = Some(required_string("drive_version", drive_version)?);
        Ok(self)
    }

    pub fn with_drive_modified_time(mut self, drive_modified_time: SafeTimestamp) -> Self {
        self.drive_modified_time = Some(drive_modified_time);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreStateObservation {
    pub core_revision: String,
    pub core_sequence: u64,
}

impl CoreStateObservation {
    pub fn new(core_revision: impl Into<String>, core_sequence: u64) -> Result<Self, StateError> {
        Ok(Self {
            core_revision: required_string("core_revision", core_revision)?,
            core_sequence,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GDriveMapping {
    pub vault_path: VaultPath,
    pub drive_file_id: String,
    pub parent_id: String,
    pub name: String,
    pub checksum: Option<String>,
    pub drive_version: Option<String>,
    pub drive_modified_time: Option<SafeTimestamp>,
    pub core_revision: Option<String>,
    pub core_sequence: Option<u64>,
    pub last_imported_at: Option<SafeTimestamp>,
    pub last_exported_at: Option<SafeTimestamp>,
    pub last_seen_at: Option<SafeTimestamp>,
    pub delete_candidate_since: Option<SafeTimestamp>,
}

impl GDriveMapping {
    pub fn new(
        vault_path: VaultPath,
        drive_file_id: impl Into<String>,
        parent_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, StateError> {
        Ok(Self {
            vault_path,
            drive_file_id: required_string("drive_file_id", drive_file_id)?,
            parent_id: required_string("parent_id", parent_id)?,
            name: required_string("name", name)?,
            checksum: None,
            drive_version: None,
            drive_modified_time: None,
            core_revision: None,
            core_sequence: None,
            last_imported_at: None,
            last_exported_at: None,
            last_seen_at: None,
            delete_candidate_since: None,
        })
    }

    pub fn record_drive_observation(
        &mut self,
        observation: DriveStateObservation,
    ) -> Result<(), StateError> {
        if self.drive_file_id != observation.drive_file_id {
            return Err(StateError::DriveFileIdMismatch);
        }

        self.parent_id = observation.parent_id;
        self.name = observation.name;
        self.checksum = observation.checksum;
        self.drive_version = observation.drive_version;
        self.drive_modified_time = observation.drive_modified_time;
        self.last_seen_at = Some(observation.observed_at);
        Ok(())
    }

    pub fn record_import(&mut self, core: CoreStateObservation, imported_at: SafeTimestamp) {
        self.core_revision = Some(core.core_revision);
        self.core_sequence = Some(core.core_sequence);
        self.last_imported_at = Some(imported_at);
        self.delete_candidate_since = None;
    }

    pub fn record_export(&mut self, core: CoreStateObservation, exported_at: SafeTimestamp) {
        self.core_revision = Some(core.core_revision);
        self.core_sequence = Some(core.core_sequence);
        self.last_exported_at = Some(exported_at);
        self.delete_candidate_since = None;
    }

    pub fn mark_delete_candidate(&mut self, detected_at: SafeTimestamp) {
        self.delete_candidate_since = Some(detected_at);
    }

    pub fn clear_delete_candidate(&mut self) {
        self.delete_candidate_since = None;
    }

    pub fn is_delete_candidate(&self) -> bool {
        self.delete_candidate_since.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveChangeCursor {
    pub start_page_token: Option<String>,
    pub next_page_token: Option<String>,
    pub sync_token: Option<String>,
    pub last_polled_at: Option<SafeTimestamp>,
    pub invalidated_at: Option<SafeTimestamp>,
}

impl DriveChangeCursor {
    pub fn new(start_page_token: impl Into<String>) -> Result<Self, StateError> {
        Ok(Self {
            start_page_token: Some(required_string("start_page_token", start_page_token)?),
            next_page_token: None,
            sync_token: None,
            last_polled_at: None,
            invalidated_at: None,
        })
    }

    pub fn advance_page(
        &mut self,
        next_page_token: impl Into<String>,
        polled_at: SafeTimestamp,
    ) -> Result<(), StateError> {
        self.next_page_token = Some(required_string("next_page_token", next_page_token)?);
        self.last_polled_at = Some(polled_at);
        Ok(())
    }

    pub fn finish_batch(
        &mut self,
        sync_token: impl Into<String>,
        polled_at: SafeTimestamp,
    ) -> Result<(), StateError> {
        self.sync_token = Some(required_string("sync_token", sync_token)?);
        self.next_page_token = None;
        self.last_polled_at = Some(polled_at);
        self.invalidated_at = None;
        Ok(())
    }

    pub fn invalidate(&mut self, invalidated_at: SafeTimestamp) {
        self.invalidated_at = Some(invalidated_at);
    }

    pub fn requires_full_scan(&self) -> bool {
        self.invalidated_at.is_some() || self.sync_token.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreChangeCursor {
    pub next_sequence: u64,
    pub last_seen_sequence: Option<u64>,
    pub last_polled_at: Option<SafeTimestamp>,
}

impl CoreChangeCursor {
    pub fn new(next_sequence: u64) -> Self {
        Self {
            next_sequence,
            last_seen_sequence: None,
            last_polled_at: None,
        }
    }

    pub fn advance_to(
        &mut self,
        next_sequence: u64,
        polled_at: SafeTimestamp,
    ) -> Result<(), StateError> {
        if next_sequence < self.next_sequence {
            return Err(StateError::CursorRegression {
                current: self.next_sequence,
                attempted: next_sequence,
            });
        }

        self.last_seen_sequence = next_sequence.checked_sub(1);
        self.next_sequence = next_sequence;
        self.last_polled_at = Some(polled_at);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveEchoObservation {
    pub drive_file_id: String,
    pub checksum: Option<String>,
    pub drive_version: Option<String>,
}

impl DriveEchoObservation {
    pub fn new(drive_file_id: impl Into<String>) -> Result<Self, StateError> {
        Ok(Self {
            drive_file_id: required_string("drive_file_id", drive_file_id)?,
            checksum: None,
            drive_version: None,
        })
    }

    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Result<Self, StateError> {
        self.checksum = Some(required_string("checksum", checksum)?);
        Ok(self)
    }

    pub fn with_drive_version(
        mut self,
        drive_version: impl Into<String>,
    ) -> Result<Self, StateError> {
        self.drive_version = Some(required_string("drive_version", drive_version)?);
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EchoGuardEntry {
    pub drive_file_id: String,
    pub checksum: Option<String>,
    pub drive_version: Option<String>,
    pub core_revision: Option<String>,
    pub core_sequence: Option<u64>,
    pub exported_at: SafeTimestamp,
}

impl EchoGuardEntry {
    pub fn from_mapping(
        mapping: &GDriveMapping,
        exported_at: SafeTimestamp,
    ) -> Result<Self, StateError> {
        Ok(Self {
            drive_file_id: required_string("drive_file_id", &mapping.drive_file_id)?,
            checksum: mapping.checksum.clone(),
            drive_version: mapping.drive_version.clone(),
            core_revision: mapping.core_revision.clone(),
            core_sequence: mapping.core_sequence,
            exported_at,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoDecision {
    AcceptRemoteChange,
    SuppressAdapterEcho,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EchoGuard {
    entries_by_drive_file_id: BTreeMap<String, EchoGuardEntry>,
}

impl EchoGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_exported_write(&mut self, entry: EchoGuardEntry) {
        self.entries_by_drive_file_id
            .insert(entry.drive_file_id.clone(), entry);
    }

    pub fn decision_for(&self, observation: &DriveEchoObservation) -> EchoDecision {
        let Some(entry) = self
            .entries_by_drive_file_id
            .get(&observation.drive_file_id)
        else {
            return EchoDecision::AcceptRemoteChange;
        };

        if echo_fingerprint_matches(entry, observation) {
            EchoDecision::SuppressAdapterEcho
        } else {
            EchoDecision::AcceptRemoteChange
        }
    }

    pub fn acknowledge_observed_echo(&mut self, observation: &DriveEchoObservation) -> bool {
        if self.decision_for(observation) != EchoDecision::SuppressAdapterEcho {
            return false;
        }

        self.entries_by_drive_file_id
            .remove(&observation.drive_file_id)
            .is_some()
    }

    pub fn expire(&mut self, drive_file_id: &str) -> bool {
        self.entries_by_drive_file_id
            .remove(drive_file_id)
            .is_some()
    }

    pub fn len(&self) -> usize {
        self.entries_by_drive_file_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries_by_drive_file_id.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingPersistenceBoundary {
    Unresolved,
    ServerApiMediated,
    StorageRepositoryMediated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatePersistencePolicy {
    pub boundary: MappingPersistenceBoundary,
}

impl StatePersistencePolicy {
    pub const fn unresolved() -> Self {
        Self {
            boundary: MappingPersistenceBoundary::Unresolved,
        }
    }

    pub const fn server_api_mediated() -> Self {
        Self {
            boundary: MappingPersistenceBoundary::ServerApiMediated,
        }
    }

    pub const fn storage_repository_mediated() -> Self {
        Self {
            boundary: MappingPersistenceBoundary::StorageRepositoryMediated,
        }
    }

    pub const fn is_resolved(self) -> bool {
        !matches!(self.boundary, MappingPersistenceBoundary::Unresolved)
    }

    pub const fn direct_database_access_allowed(self) -> bool {
        false
    }

    pub fn reject_direct_database_access(self) -> Result<(), StateError> {
        Err(StateError::DirectDatabaseAccessNotAccepted)
    }
}

impl Default for StatePersistencePolicy {
    fn default() -> Self {
        Self::unresolved()
    }
}

fn required_string(field: &'static str, raw: impl Into<String>) -> Result<String, StateError> {
    let value = raw.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(StateError::empty(field));
    }
    Ok(trimmed.to_owned())
}

fn echo_fingerprint_matches(
    entry: &EchoGuardEntry,
    observation: &DriveEchoObservation,
) -> bool {
    let mut compared_field = false;
    for (expected, observed) in [
        (&entry.checksum, &observation.checksum),
        (&entry.drive_version, &observation.drive_version),
    ] {
        match (expected.as_deref(), observed.as_deref()) {
            (Some(expected), Some(observed)) if expected != observed => return false,
            (Some(_), Some(_)) => compared_field = true,
            _ => {}
        }
    }
    compared_field
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(value: &str) -> SafeTimestamp {
        SafeTimestamp::new(value).expect("timestamp")
    }

    fn mapping() -> GDriveMapping {
        GDriveMapping::new(
            VaultPath::new("notes/plan.md").expect("path"),
            "drive-1",
            "root",
            "plan.md",
        )
        .expect("mapping")
    }

    #[test]
    fn mapping_tracks_drive_core_and_delete_candidate_state() {
        let mut mapping = mapping();
        let drive_observation =
            DriveStateObservation::new("drive-1", "root", "plan.md", ts("2026-07-09T12:00:00Z"))
                .expect("drive observation")
                .with_checksum("checksum-1")
                .expect("checksum")
                .with_drive_version("drive-version-1")
                .expect("drive version")
                .with_drive_modified_time(ts("2026-07-09T11:59:00Z"));

        mapping
            .record_drive_observation(drive_observation)
            .expect("record drive observation");
        mapping.record_import(
            CoreStateObservation::new("core-rev-1", 7).expect("core observation"),
            ts("2026-07-09T12:01:00Z"),
        );
        mapping.mark_delete_candidate(ts("2026-07-09T12:02:00Z"));

        assert_eq!(mapping.vault_path.as_str(), "notes/plan.md");
        assert_eq!(mapping.checksum.as_deref(), Some("checksum-1"));
        assert_eq!(mapping.drive_version.as_deref(), Some("drive-version-1"));
        assert_eq!(mapping.core_revision.as_deref(), Some("core-rev-1"));
        assert_eq!(mapping.core_sequence, Some(7));
        assert!(mapping.is_delete_candidate());

        mapping.clear_delete_candidate();

        assert!(!mapping.is_delete_candidate());
    }

    #[test]
    fn mapping_rejects_observation_for_different_drive_file() {
        let mut mapping = mapping();
        let observation = DriveStateObservation::new(
            "drive-2",
            "root",
            "other.md",
            ts("2026-07-09T12:00:00Z"),
        )
        .expect("drive observation");

        assert_eq!(
            mapping.record_drive_observation(observation),
            Err(StateError::DriveFileIdMismatch)
        );
        assert_eq!(mapping.drive_file_id, "drive-1");
        assert_eq!(mapping.name, "plan.md");
        assert_eq!(mapping.last_seen_at, None);
    }

    #[test]
    fn vault_path_rejects_absolute_parent_and_empty_segments() {
        assert!(VaultPath::new("/notes/plan.md").is_err());
        assert!(VaultPath::new("notes/../plan.md").is_err());
        assert!(VaultPath::new("notes//plan.md").is_err());
        assert!(VaultPath::new("notes\\plan.md").is_err());
    }

    #[test]
    fn drive_cursor_tracks_page_sync_and_invalidation() {
        let mut cursor = DriveChangeCursor::new("start-token").expect("cursor");

        assert!(cursor.requires_full_scan());

        cursor
            .advance_page("next-page", ts("2026-07-09T12:00:00Z"))
            .expect("advance page");
        cursor
            .finish_batch("sync-token", ts("2026-07-09T12:01:00Z"))
            .expect("finish batch");

        assert_eq!(cursor.next_page_token, None);
        assert_eq!(cursor.sync_token.as_deref(), Some("sync-token"));
        assert!(!cursor.requires_full_scan());

        cursor.invalidate(ts("2026-07-09T12:02:00Z"));

        assert!(cursor.requires_full_scan());
    }

    #[test]
    fn core_cursor_rejects_regressions() {
        let mut cursor = CoreChangeCursor::new(10);

        cursor
            .advance_to(12, ts("2026-07-09T12:00:00Z"))
            .expect("advance");

        assert_eq!(cursor.next_sequence, 12);
        assert_eq!(cursor.last_seen_sequence, Some(11));
        assert_eq!(
            cursor.advance_to(11, ts("2026-07-09T12:01:00Z")),
            Err(StateError::CursorRegression {
                current: 12,
                attempted: 11,
            })
        );
    }

    #[test]
    fn echo_guard_suppresses_only_consistent_adapter_fingerprint() {
        let mut mapping = mapping();
        mapping.checksum = Some("exported-checksum".to_owned());
        mapping.drive_version = Some("exported-version".to_owned());
        mapping.record_export(
            CoreStateObservation::new("core-rev-2", 12).expect("core observation"),
            ts("2026-07-09T12:00:00Z"),
        );

        let mut guard = EchoGuard::new();
        guard.record_exported_write(
            EchoGuardEntry::from_mapping(&mapping, ts("2026-07-09T12:00:00Z"))
                .expect("echo entry"),
        );

        let adapter_echo = DriveEchoObservation::new("drive-1")
            .expect("observation")
            .with_checksum("exported-checksum")
            .expect("checksum")
            .with_drive_version("exported-version")
            .expect("drive version");
        let checksum_conflict = DriveEchoObservation::new("drive-1")
            .expect("observation")
            .with_checksum("remote-checksum")
            .expect("checksum")
            .with_drive_version("exported-version")
            .expect("drive version");
        let version_conflict = DriveEchoObservation::new("drive-1")
            .expect("observation")
            .with_checksum("exported-checksum")
            .expect("checksum")
            .with_drive_version("remote-version")
            .expect("drive version");

        assert_eq!(
            guard.decision_for(&adapter_echo),
            EchoDecision::SuppressAdapterEcho
        );
        assert_eq!(
            guard.decision_for(&checksum_conflict),
            EchoDecision::AcceptRemoteChange
        );
        assert_eq!(
            guard.decision_for(&version_conflict),
            EchoDecision::AcceptRemoteChange
        );
        assert!(guard.acknowledge_observed_echo(&adapter_echo));
        assert!(guard.is_empty());
    }

    #[test]
    fn echo_guard_can_match_single_available_fingerprint() {
        let mut mapping = mapping();
        mapping.checksum = Some("exported-checksum".to_owned());

        let mut guard = EchoGuard::new();
        guard.record_exported_write(
            EchoGuardEntry::from_mapping(&mapping, ts("2026-07-09T12:00:00Z"))
                .expect("echo entry"),
        );
        let adapter_echo = DriveEchoObservation::new("drive-1")
            .expect("observation")
            .with_checksum("exported-checksum")
            .expect("checksum");

        assert_eq!(
            guard.decision_for(&adapter_echo),
            EchoDecision::SuppressAdapterEcho
        );
    }

    #[test]
    fn persistence_policy_defaults_to_unresolved_boundary() {
        let unresolved = StatePersistencePolicy::default();
        let server_api = StatePersistencePolicy::server_api_mediated();

        assert_eq!(
            unresolved.boundary,
            MappingPersistenceBoundary::Unresolved
        );
        assert!(!unresolved.is_resolved());
        assert!(server_api.is_resolved());
        assert!(!unresolved.direct_database_access_allowed());
        assert_eq!(
            unresolved.reject_direct_database_access(),
            Err(StateError::DirectDatabaseAccessNotAccepted)
        );
    }
}
