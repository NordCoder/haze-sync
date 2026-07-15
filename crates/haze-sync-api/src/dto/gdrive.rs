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

/// Opaque Drive change cursor accepted only by the authenticated private commit contract.
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveLastOperationsSummaryDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import: Option<OperationIdDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export: Option<OperationIdDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_mutation: Option<OperationIdDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveEchoFactsDto {
    pub state: GDriveEchoStateDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<OperationIdDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_version: Option<GDriveProviderIdentifierDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveDeleteCandidateFactsDto {
    pub first_seen_at: TimestampDto,
    pub last_seen_at: TimestampDto,
    pub generation: u64,
    pub blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_audit_id: Option<GDriveProviderIdentifierDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveStateSnapshotResponse {
    pub adapter_id: AdapterIdDto,
    pub state_format_version: u32,
    pub state_version: u64,
    pub cursor: GDriveCursorSummaryDto,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveCursorAdvanceDto {
    pub next_generation: u64,
    pub cursor: GDriveRawCursorDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GDriveCursorCommitDto {
    pub expected_generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advance: Option<GDriveCursorAdvanceDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
