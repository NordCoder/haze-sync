//! DTOs for GET /v1/server-info.
//!
//! This module defines the server capability response shape only. It does not
//! implement the endpoint or read runtime configuration.

use serde::{Deserialize, Serialize};

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

/// Public server capability names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_info_roundtrips_contract_json() {
        let response = ServerInfoResponse {
            server_id: "haze-sync-vps-1".to_owned(),
            protocol_version: 1,
            max_upload_bytes: 52_428_800,
            capabilities: vec![
                ServerCapabilityDto::Sha256,
                ServerCapabilityDto::OperationLog,
                ServerCapabilityDto::Tombstones,
                ServerCapabilityDto::Conflicts,
                ServerCapabilityDto::ConflictCenter,
                ServerCapabilityDto::BatchChanges,
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("batch_changes"));

        let decoded: ServerInfoResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, response);
    }
}
