//! Passive public DTOs for Google Drive durable-state transport.
//!
//! These types describe authenticated Server/API transport only. They do not
//! access Storage, execute provider work, refresh OAuth tokens, schedule cycles,
//! or derive synchronization policy.

use std::{error::Error, fmt};

use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

use super::primitives::{AdapterIdDto, OperationIdDto, RevisionIdDto, TimestampDto, VaultPathDto};

pub const MAX_GDRIVE_STATE_ITEMS: usize = 500;
pub const MAX_GDRIVE_CURSOR_BYTES: usize = 8_192;
pub const MAX_GDRIVE_IDENTIFIER_BYTES: usize = 1_024;
pub const MAX_GDRIVE_TEXT_BYTES: usize = 4_096;
pub const GDRIVE_FACTS_FINGERPRINT_HEX_BYTES: usize = 64;
pub const GDRIVE_MD5_HEX_BYTES: usize = 32;

fn validate_bounded_atom(value: &str, max_bytes: usize) -> Result<(), GDriveValueError> {
    if value.is_empty() {
        return Err(GDriveValueError::Empty);
    }
    if value.len() > max_bytes {
        return Err(GDriveValueError::TooLong { max_bytes });
    }
    if value
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(GDriveValueError::InvalidCharacter);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GDriveValueError {
    Empty,
    TooLong { max_bytes: usize },
    InvalidCharacter,
    InvalidHex { expected_bytes: usize },
}

impl fmt::Display for GDriveValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("value must not be empty"),
            Self::TooLong { max_bytes } => {
                write!(formatter, "value must not exceed {max_bytes} bytes")
            }
            Self::InvalidCharacter => formatter.write_str("value contains an invalid character"),
            Self::InvalidHex { expected_bytes } => {
                write!(
                    formatter,
                    "value must contain exactly {expected_bytes} hexadecimal bytes"
                )
            }
        }
    }
}

impl Error for GDriveValueError {}

/// Bounded opaque provider identifier used by the authenticated adapter state contract.
///
/// The value serializes for the matching adapter but is redacted from formatting.
#[derive(Clone, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct GDriveProviderIdentifierDto(String);

impl GDriveProviderIdentifierDto {
    pub fn parse(value: impl Into<String>) -> Result<Self, GDriveValueError> {
        let value = value.into();
        validate_bounded_atom(&value, MAX_GDRIVE_IDENTIFIER_BYTES)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn expose_for_private_transport(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for GDriveProviderIdentifierDto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for GDriveProviderIdentifierDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GDriveProviderIdentifierDto(<redacted>)")
    }
}

impl fmt::Display for GDriveProviderIdentifierDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

/// Opaque Drive change cursor accepted only by authenticated private contracts.
#[derive(Clone, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct GDriveRawCursorDto(String);

impl GDriveRawCursorDto {
    pub fn parse(value: impl Into<String>) -> Result<Self, GDriveValueError> {
        let value = value.into();
        if value.is_empty() {
            return Err(GDriveValueError::Empty);
        }
        if value.len() > MAX_GDRIVE_CURSOR_BYTES {
            return Err(GDriveValueError::TooLong {
                max_bytes: MAX_GDRIVE_CURSOR_BYTES,
            });
        }
        if value.chars().any(char::is_control) {
            return Err(GDriveValueError::InvalidCharacter);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn expose_for_private_commit(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for GDriveRawCursorDto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for GDriveRawCursorDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GDriveRawCursorDto(<redacted>)")
    }
}

impl fmt::Display for GDriveRawCursorDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

/// Complete private cursor state returned only to the matching GDrive adapter.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum GDrivePrivateCursorStateDto {
    Absent { generation: u64 },
    Present {
        generation: u64,
        cursor: GDriveRawCursorDto,
    },
}

impl fmt::Debug for GDrivePrivateCursorStateDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GDrivePrivateCursorStateDto(<redacted>)")
    }
}

impl fmt::Display for GDrivePrivateCursorStateDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

/// SHA-256 facts fingerprint without a wire prefix.
#[derive(Clone, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct GDriveFactsFingerprintDto(String);

impl GDriveFactsFingerprintDto {
    pub fn parse(value: impl Into<String>) -> Result<Self, GDriveValueError> {
        let value = value.into();
        if value.len() != GDRIVE_FACTS_FINGERPRINT_HEX_BYTES
            || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(GDriveValueError::InvalidHex {
                expected_bytes: GDRIVE_FACTS_FINGERPRINT_HEX_BYTES,
            });
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    #[must_use]
    pub fn expose_for_private_commit(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for GDriveFactsFingerprintDto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for GDriveFactsFingerprintDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GDriveFactsFingerprintDto(<redacted>)")
    }
}

impl fmt::Display for GDriveFactsFingerprintDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GDriveEchoStateDto {
    None,
    Pending,
    Confirmed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GDriveOperationKindDto {
    Import,
    Export,
    ProviderMutation,
    CursorCheckpoint,
    DeleteCandidate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveCursorSummaryDto {
    pub generation: u64,
    pub present: bool,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveLastOperationsSummaryDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import: Option<OperationIdDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export: Option<OperationIdDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_mutation: Option<OperationIdDto>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveEchoFactsDto {
    pub state: GDriveEchoStateDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<OperationIdDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_version: Option<GDriveProviderIdentifierDto>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveDeleteCandidateFactsDto {
    pub first_seen_at: TimestampDto,
    pub last_seen_at: TimestampDto,
    pub generation: u64,
    pub blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_audit_id: Option<GDriveProviderIdentifierDto>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveMappingFactsDto {
    pub path: VaultPathDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_file_id: Option<GDriveProviderIdentifierDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_parent_id: Option<GDriveProviderIdentifierDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5_checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_revision_id: Option<GDriveProviderIdentifierDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_version: Option<GDriveProviderIdentifierDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_modified_time: Option<TimestampDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_object_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_revision_id: Option<RevisionIdDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_seq: Option<u64>,
    pub echo: GDriveEchoFactsDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_candidate: Option<GDriveDeleteCandidateFactsDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_imported_at: Option<TimestampDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_exported_at: Option<TimestampDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<TimestampDto>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveStateSnapshotResponse {
    pub adapter_id: AdapterIdDto,
    pub state_format_version: u32,
    pub state_version: u64,
    pub cursor: GDrivePrivateCursorStateDto,
    pub core_export_checkpoint: u64,
    pub last_operations: GDriveLastOperationsSummaryDto,
    pub mappings: Vec<GDriveMappingFactsDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_after_path: Option<VaultPathDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveEchoCountSummaryDto {
    pub none: u64,
    pub pending: u64,
    pub confirmed: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveDeleteCandidateCountSummaryDto {
    pub total: u64,
    pub blocked: u64,
}

/// Provider-identifier-free snapshot available to an authenticated admin.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveStateAdminSummaryResponse {
    pub adapter_id: AdapterIdDto,
    pub state_format_version: u32,
    pub state_version: u64,
    pub cursor: GDriveCursorSummaryDto,
    pub core_export_checkpoint: u64,
    pub mapping_count: u64,
    pub echo_counts: GDriveEchoCountSummaryDto,
    pub delete_candidates: GDriveDeleteCandidateCountSummaryDto,
    pub last_operation_presence: GDriveLastOperationPresenceDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveLastOperationPresenceDto {
    pub import: bool,
    pub export: bool,
    pub provider_mutation: bool,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveCursorAdvanceDto {
    pub next_generation: u64,
    pub cursor: GDriveRawCursorDto,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveCursorCommitDto {
    pub expected_generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advance: Option<GDriveCursorAdvanceDto>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveOperationFactsDto {
    pub operation_id: OperationIdDto,
    pub kind: GDriveOperationKindDto,
    pub facts_fingerprint: GDriveFactsFingerprintDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping_path: Option<VaultPathDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_version: Option<GDriveProviderIdentifierDto>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveStateCommitRequest {
    pub expected_state_version: u64,
    pub cursor: GDriveCursorCommitDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_export_checkpoint: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping: Option<GDriveMappingFactsDto>,
    pub operation: GDriveOperationFactsDto,
}

macro_rules! redacted_private_debug {
    ($($type:ty => $name:literal),+ $(,)?) => {
        $(
            impl fmt::Debug for $type {
                fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str(concat!($name, "(<redacted>)"))
                }
            }
        )+
    };
}

redacted_private_debug!(
    GDriveLastOperationsSummaryDto => "GDriveLastOperationsSummaryDto",
    GDriveEchoFactsDto => "GDriveEchoFactsDto",
    GDriveDeleteCandidateFactsDto => "GDriveDeleteCandidateFactsDto",
    GDriveMappingFactsDto => "GDriveMappingFactsDto",
    GDriveStateSnapshotResponse => "GDriveStateSnapshotResponse",
    GDriveCursorAdvanceDto => "GDriveCursorAdvanceDto",
    GDriveCursorCommitDto => "GDriveCursorCommitDto",
    GDriveOperationFactsDto => "GDriveOperationFactsDto",
    GDriveStateCommitRequest => "GDriveStateCommitRequest",
);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GDriveStateCommitResponse {
    Committed {
        state_version: u64,
        cursor_generation: u64,
        core_export_checkpoint: u64,
    },
    Replayed {
        state_version: u64,
    },
    StaleState,
    CursorRegression,
    CursorGap,
    MappingConflict,
    IdempotencyConflict,
    ValidationFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GDriveStateErrorCode {
    Unauthorized,
    Forbidden,
    AdapterNotFound,
    StateVersionMismatch,
    InvalidCursorState,
    StaleState,
    CursorRegression,
    CursorGap,
    MappingConflict,
    IdempotencyConflict,
    ValidationError,
    Unavailable,
    Internal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveStatePublicError {
    pub code: GDriveStateErrorCode,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveStateErrorResponse {
    pub error: GDriveStatePublicError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_values_roundtrip_but_format_as_redacted() {
        let cursor = GDriveRawCursorDto::parse("cursor-fixture-01").unwrap();
        let provider_id = GDriveProviderIdentifierDto::parse("drive-file-fixture-01").unwrap();
        let fingerprint = GDriveFactsFingerprintDto::parse("a".repeat(64)).unwrap();

        assert_eq!(
            serde_json::to_string(&cursor).unwrap(),
            "\"cursor-fixture-01\""
        );
        assert_eq!(format!("{cursor:?}"), "GDriveRawCursorDto(<redacted>)");
        assert_eq!(format!("{cursor}"), "<redacted>");
        assert_eq!(
            format!("{provider_id:?}"),
            "GDriveProviderIdentifierDto(<redacted>)"
        );
        assert_eq!(
            format!("{fingerprint:?}"),
            "GDriveFactsFingerprintDto(<redacted>)"
        );
    }

    #[test]
    fn private_cursor_state_uses_exact_strict_tagged_wire_shapes() {
        let absent = GDrivePrivateCursorStateDto::Absent { generation: 0 };
        assert_eq!(
            serde_json::to_string(&absent).unwrap(),
            "{\"state\":\"absent\",\"generation\":0}"
        );

        let sentinel = "synthetic-private-cursor-sentinel";
        let present = GDrivePrivateCursorStateDto::Present {
            generation: 3,
            cursor: GDriveRawCursorDto::parse(sentinel).unwrap(),
        };
        assert_eq!(
            serde_json::to_string(&present).unwrap(),
            format!(
                "{{\"state\":\"present\",\"generation\":3,\"cursor\":\"{sentinel}\"}}"
            )
        );
        assert_eq!(
            format!("{present:?}"),
            "GDrivePrivateCursorStateDto(<redacted>)"
        );
        assert_eq!(format!("{present}"), "<redacted>");
        assert!(!format!("{present:?}").contains(sentinel));
        assert!(!format!("{present}").contains(sentinel));

        for invalid in [
            r#"{"state":"present","generation":3}"#,
            r#"{"state":"absent","generation":0,"cursor":"synthetic"}"#,
            r#"{"state":"absent","generation":0,"unexpected":true}"#,
            r#"{"state":"present","generation":3,"cursor":"","unexpected":false}"#,
        ] {
            assert!(serde_json::from_str::<GDrivePrivateCursorStateDto>(invalid).is_err());
        }
    }

    #[test]
    fn private_cursor_state_preserves_cursor_bounds() {
        let over_bound = "x".repeat(MAX_GDRIVE_CURSOR_BYTES + 1);
        for invalid in [
            serde_json::json!({"state": "present", "generation": 1, "cursor": ""}),
            serde_json::json!({"state": "present", "generation": 1, "cursor": "line\nbreak"}),
            serde_json::json!({"state": "present", "generation": 1, "cursor": over_bound}),
        ] {
            assert!(serde_json::from_value::<GDrivePrivateCursorStateDto>(invalid).is_err());
        }
    }

    #[test]
    fn private_fact_dtos_never_debug_format_sentinel_values() {
        let json = serde_json::json!({
            "expected_state_version": 7,
            "cursor": {
                "expected_generation": 3,
                "advance": {
                    "next_generation": 4,
                    "cursor": "sentinel-raw-cursor"
                }
            },
            "core_export_checkpoint": 19,
            "mapping": {
                "path": "Sentinel/private-path.md",
                "drive_file_id": "sentinel-drive-file-id",
                "drive_parent_id": "sentinel-drive-parent-id",
                "drive_name": "sentinel-drive-name.md",
                "mime_type": "application/x-sentinel-mime",
                "md5_checksum": "0123456789abcdef0123456789abcdef",
                "head_revision_id": "sentinel-head-revision",
                "drive_version": "sentinel-drive-version",
                "drive_modified_time": "2099-01-01T00:00:01Z",
                "core_object_id": "sentinel-core-object-id",
                "core_revision_id": "sentinel-core-revision-id",
                "core_seq": 19,
                "echo": {
                    "state": "confirmed",
                    "operation_id": "sentinel-echo-operation-id",
                    "provider_version": "sentinel-echo-provider-version"
                },
                "delete_candidate": {
                    "first_seen_at": "2099-01-01T00:00:02Z",
                    "last_seen_at": "2099-01-01T00:00:03Z",
                    "generation": 2,
                    "blocked": true,
                    "confirmation_audit_id": "sentinel-confirmation-audit-id"
                },
                "last_imported_at": "2099-01-01T00:00:04Z",
                "last_exported_at": "2099-01-01T00:00:05Z",
                "last_seen_at": "2099-01-01T00:00:06Z"
            },
            "operation": {
                "operation_id": "sentinel-commit-operation-id",
                "kind": "export",
                "facts_fingerprint": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "mapping_path": "Sentinel/private-path.md",
                "core_seq": 19,
                "drive_version": "sentinel-operation-drive-version"
            }
        });
        let request: GDriveStateCommitRequest = serde_json::from_value(json).unwrap();
        let debug = format!("{request:?}");

        assert_eq!(debug, "GDriveStateCommitRequest(<redacted>)");
        for sentinel in [
            "sentinel-raw-cursor",
            "Sentinel/private-path.md",
            "sentinel-drive-file-id",
            "sentinel-drive-parent-id",
            "sentinel-drive-name.md",
            "application/x-sentinel-mime",
            "0123456789abcdef0123456789abcdef",
            "sentinel-head-revision",
            "sentinel-drive-version",
            "2099-01-01T00:00:01Z",
            "sentinel-core-object-id",
            "sentinel-core-revision-id",
            "sentinel-echo-operation-id",
            "sentinel-echo-provider-version",
            "sentinel-confirmation-audit-id",
            "sentinel-commit-operation-id",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "sentinel-operation-drive-version",
        ] {
            assert!(!debug.contains(sentinel));
        }
    }

    #[test]
    fn private_value_deserialization_is_bounded() {
        assert!(serde_json::from_str::<GDriveRawCursorDto>("\"\"").is_err());
        assert!(serde_json::from_str::<GDriveProviderIdentifierDto>("\"with space\"").is_err());
        assert!(serde_json::from_str::<GDriveFactsFingerprintDto>("\"abcd\"").is_err());
    }

    #[test]
    fn commit_outcomes_use_stable_tagged_json() {
        assert_eq!(
            serde_json::to_string(&GDriveStateCommitResponse::CursorGap).unwrap(),
            "{\"status\":\"cursor_gap\"}"
        );
        assert_eq!(
            serde_json::to_string(&GDriveStateCommitResponse::Committed {
                state_version: 8,
                cursor_generation: 4,
                core_export_checkpoint: 31,
            })
            .unwrap(),
            "{\"status\":\"committed\",\"state_version\":8,\"cursor_generation\":4,\"core_export_checkpoint\":31}"
        );
    }
}
