//! DTOs for GET /v1/server-info.
//!
//! This module defines the server capability response shape only. It does not
//! implement the endpoint or read runtime configuration.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Current public Haze Sync API protocol version.
pub const CURRENT_PROTOCOL_VERSION: u32 = 1;

/// Maximum accepted byte length for a public server identifier.
pub const MAX_SERVER_ID_BYTES: usize = 128;

/// Response body for GET /v1/server-info.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInfoResponse {
    /// Stable server identifier safe to expose to adapters.
    pub server_id: String,
    /// Core API protocol version.
    pub protocol_version: u32,
    /// Maximum accepted upload size in bytes.
    pub max_upload_bytes: u64,
    /// Public capabilities supported by the server.
    pub capabilities: Vec<ServerCapabilityDto>,
}

impl ServerInfoResponse {
    /// Build server-info metadata for the current public protocol version.
    ///
    /// The caller remains responsible for advertising only capabilities actually
    /// supported by the runtime. This constructor validates transport-safe public
    /// metadata and rejects duplicate capability names.
    pub fn current(
        server_id: impl Into<String>,
        max_upload_bytes: u64,
        capabilities: Vec<ServerCapabilityDto>,
    ) -> Result<Self, ServerInfoContractError> {
        Self::new(
            server_id,
            CURRENT_PROTOCOL_VERSION,
            max_upload_bytes,
            capabilities,
        )
    }

    /// Build validated server-info metadata for an explicit protocol version.
    pub fn new(
        server_id: impl Into<String>,
        protocol_version: u32,
        max_upload_bytes: u64,
        capabilities: Vec<ServerCapabilityDto>,
    ) -> Result<Self, ServerInfoContractError> {
        let response = Self {
            server_id: server_id.into(),
            protocol_version,
            max_upload_bytes,
            capabilities,
        };
        response.validate()?;
        Ok(response)
    }

    /// Validate public metadata without consulting runtime state.
    pub fn validate(&self) -> Result<(), ServerInfoContractError> {
        validate_server_id(&self.server_id)?;
        if self.protocol_version == 0 {
            return Err(ServerInfoContractError::InvalidProtocolVersion);
        }
        if self.max_upload_bytes == 0 {
            return Err(ServerInfoContractError::InvalidMaxUploadBytes);
        }
        if has_duplicate_capabilities(&self.capabilities) {
            return Err(ServerInfoContractError::DuplicateCapability);
        }
        Ok(())
    }

    /// Whether the server advertises a specific public capability.
    #[must_use]
    pub fn supports(&self, capability: ServerCapabilityDto) -> bool {
        self.capabilities.contains(&capability)
    }

    /// Whether an adapter using the supplied protocol version is compatible.
    ///
    /// V1 currently requires an exact version match; future negotiation can add a
    /// separate range contract without silently changing this rule.
    #[must_use]
    pub const fn is_protocol_compatible(&self, adapter_protocol_version: u32) -> bool {
        self.protocol_version == adapter_protocol_version
    }
}

/// Public server capability names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerCapabilityDto {
    /// Content hashes use SHA-256.
    Sha256,
    /// Operation log change feed is available.
    OperationLog,
    /// Deletes are represented by tombstones.
    Tombstones,
    /// Conflict records are supported.
    Conflicts,
    /// Conflict Center workflows are supported.
    ConflictCenter,
    /// Batched change feed responses are supported.
    BatchChanges,
}

/// Safe validation error for server-info metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerInfoContractError {
    /// Server id was empty, too long, or contained path/URL/control syntax.
    InvalidServerId,
    /// Protocol version zero is reserved and cannot be advertised.
    InvalidProtocolVersion,
    /// Upload limit must be positive.
    InvalidMaxUploadBytes,
    /// The capability list contained a duplicate value.
    DuplicateCapability,
}

impl fmt::Display for ServerInfoContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidServerId => "invalid public server identifier",
            Self::InvalidProtocolVersion => "protocol version must be positive",
            Self::InvalidMaxUploadBytes => "maximum upload size must be positive",
            Self::DuplicateCapability => "server capability list contains duplicates",
        })
    }
}

impl Error for ServerInfoContractError {}

fn validate_server_id(server_id: &str) -> Result<(), ServerInfoContractError> {
    if server_id.is_empty()
        || server_id.len() > MAX_SERVER_ID_BYTES
        || server_id.chars().any(|character| {
            character.is_control()
                || character.is_whitespace()
                || matches!(character, '/' | '\\' | ':')
        })
    {
        return Err(ServerInfoContractError::InvalidServerId);
    }
    Ok(())
}

fn has_duplicate_capabilities(capabilities: &[ServerCapabilityDto]) -> bool {
    capabilities
        .iter()
        .enumerate()
        .any(|(index, capability)| capabilities[..index].contains(capability))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities() -> Vec<ServerCapabilityDto> {
        vec![
            ServerCapabilityDto::Sha256,
            ServerCapabilityDto::OperationLog,
            ServerCapabilityDto::Tombstones,
            ServerCapabilityDto::Conflicts,
            ServerCapabilityDto::ConflictCenter,
            ServerCapabilityDto::BatchChanges,
        ]
    }

    #[test]
    fn current_server_info_roundtrips_stable_capability_and_protocol_metadata() {
        let response =
            ServerInfoResponse::current("haze-sync-vps-1", 52_428_800, capabilities()).unwrap();

        assert_eq!(response.protocol_version, CURRENT_PROTOCOL_VERSION);
        assert!(response.is_protocol_compatible(CURRENT_PROTOCOL_VERSION));
        assert!(!response.is_protocol_compatible(CURRENT_PROTOCOL_VERSION + 1));
        assert!(response.supports(ServerCapabilityDto::Sha256));
        assert!(response.supports(ServerCapabilityDto::ConflictCenter));

        let json = serde_json::to_string(&response).unwrap();
        assert_eq!(
            json,
            "{\"server_id\":\"haze-sync-vps-1\",\"protocol_version\":1,\"max_upload_bytes\":52428800,\"capabilities\":[\"sha256\",\"operation_log\",\"tombstones\",\"conflicts\",\"conflict_center\",\"batch_changes\"]}"
        );
        assert_no_runtime_secrets(&json);

        let decoded: ServerInfoResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn unsafe_or_malformed_server_info_metadata_is_rejected() {
        for server_id in [
            "",
            "haze sync",
            "postgres://db.internal/haze",
            "/srv/haze-sync",
            "C:\\haze-sync",
            "haze-sync\ninternal",
        ] {
            assert_eq!(
                ServerInfoResponse::current(server_id, 1, vec![]).unwrap_err(),
                ServerInfoContractError::InvalidServerId
            );
        }

        assert_eq!(
            ServerInfoResponse::new("haze-sync", 0, 1, vec![]).unwrap_err(),
            ServerInfoContractError::InvalidProtocolVersion
        );
        assert_eq!(
            ServerInfoResponse::current("haze-sync", 0, vec![]).unwrap_err(),
            ServerInfoContractError::InvalidMaxUploadBytes
        );
        assert_eq!(
            ServerInfoResponse::current(
                "haze-sync",
                1,
                vec![ServerCapabilityDto::Sha256, ServerCapabilityDto::Sha256],
            )
            .unwrap_err(),
            ServerInfoContractError::DuplicateCapability
        );
    }

    #[test]
    fn capability_json_vocabulary_is_stable() {
        let json = serde_json::to_string(&capabilities()).unwrap();
        assert_eq!(
            json,
            "[\"sha256\",\"operation_log\",\"tombstones\",\"conflicts\",\"conflict_center\",\"batch_changes\"]"
        );
    }

    fn assert_no_runtime_secrets(json: &str) {
        let json = json.to_ascii_lowercase();
        for forbidden in [
            "bearer ",
            "token_hash",
            "database_url",
            "postgres://",
            "provider_payload",
            "raw_error",
            "raw_cursor",
            "/srv/",
            "c:\\",
            "stack_trace",
            "backtrace",
        ] {
            assert!(!json.contains(forbidden));
        }
    }
}
