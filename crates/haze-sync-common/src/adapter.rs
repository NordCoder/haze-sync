//! Adapter role and mode primitives.
//!
//! This module owns shared role/mode vocabulary and stable wire values only.
//! Runtime authorization, adapter scheduling, and rollout enforcement belong to
//! API, Server, CLI, and adapter components.

use crate::ValidationError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// V1 adapter authorization role.
///
/// Roles are stable snake_case wire values. This enum does not decide whether a
/// request is authorized; downstream runtime components enforce that policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterRole {
    ObsidianPlugin,
    GdriveAdapter,
    WorktreeAdapter,
    Admin,
    ReadonlyAgent,
}

impl AdapterRole {
    /// Stable wire value for this role.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ObsidianPlugin => "obsidian_plugin",
            Self::GdriveAdapter => "gdrive_adapter",
            Self::WorktreeAdapter => "worktree_adapter",
            Self::Admin => "admin",
            Self::ReadonlyAgent => "readonly_agent",
        }
    }
}

impl fmt::Display for AdapterRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AdapterRole {
    type Err = ValidationError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "obsidian_plugin" => Ok(Self::ObsidianPlugin),
            "gdrive_adapter" => Ok(Self::GdriveAdapter),
            "worktree_adapter" => Ok(Self::WorktreeAdapter),
            "admin" => Ok(Self::Admin),
            "readonly_agent" => Ok(Self::ReadonlyAgent),
            _ => Err(ValidationError::InvalidAdapterRole),
        }
    }
}

impl TryFrom<&str> for AdapterRole {
    type Error = ValidationError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::from_str(input)
    }
}

/// Adapter rollout mode.
///
/// Modes are stable snake_case wire values. Helper methods expose only
/// declarative capability facts; they are not permission checks and do not
/// implement adapter runtime policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterMode {
    Disabled,
    ReadOnly,
    ImportOnly,
    ExportOnly,
    Bidirectional,
    DryRun,
}

impl AdapterMode {
    /// Stable wire value for this mode.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::ReadOnly => "read_only",
            Self::ImportOnly => "import_only",
            Self::ExportOnly => "export_only",
            Self::Bidirectional => "bidirectional",
            Self::DryRun => "dry_run",
        }
    }

    /// Returns whether this mode declares that an adapter can submit writes to Core.
    ///
    /// Callers must still enforce authorization, rollout, and request-specific
    /// policy in their own component scope.
    #[must_use]
    pub const fn allows_core_writes(self) -> bool {
        matches!(self, Self::ImportOnly | Self::Bidirectional)
    }

    /// Returns whether this mode declares that an adapter can read from Core.
    ///
    /// Callers must still enforce authorization, rollout, and request-specific
    /// policy in their own component scope.
    #[must_use]
    pub const fn allows_core_reads(self) -> bool {
        matches!(
            self,
            Self::ReadOnly | Self::ExportOnly | Self::Bidirectional | Self::DryRun
        )
    }
}

impl fmt::Display for AdapterMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AdapterMode {
    type Err = ValidationError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "disabled" => Ok(Self::Disabled),
            "read_only" => Ok(Self::ReadOnly),
            "import_only" => Ok(Self::ImportOnly),
            "export_only" => Ok(Self::ExportOnly),
            "bidirectional" => Ok(Self::Bidirectional),
            "dry_run" => Ok(Self::DryRun),
            _ => Err(ValidationError::InvalidAdapterMode),
        }
    }
}

impl TryFrom<&str> for AdapterMode {
    type Error = ValidationError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::from_str(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROLE_CASES: &[(&str, AdapterRole)] = &[
        ("obsidian_plugin", AdapterRole::ObsidianPlugin),
        ("gdrive_adapter", AdapterRole::GdriveAdapter),
        ("worktree_adapter", AdapterRole::WorktreeAdapter),
        ("admin", AdapterRole::Admin),
        ("readonly_agent", AdapterRole::ReadonlyAgent),
    ];

    const MODE_CASES: &[(&str, AdapterMode)] = &[
        ("disabled", AdapterMode::Disabled),
        ("read_only", AdapterMode::ReadOnly),
        ("import_only", AdapterMode::ImportOnly),
        ("export_only", AdapterMode::ExportOnly),
        ("bidirectional", AdapterMode::Bidirectional),
        ("dry_run", AdapterMode::DryRun),
    ];

    #[test]
    fn role_parses_and_formats_all_scoped_roles() {
        for (wire, expected) in ROLE_CASES {
            assert_eq!(AdapterRole::from_str(wire).unwrap(), *expected);
            assert_eq!(AdapterRole::try_from(*wire).unwrap(), *expected);
            assert_eq!(expected.as_str(), *wire);
            assert_eq!(expected.to_string(), *wire);
        }

        assert_eq!(
            AdapterRole::from_str("root").unwrap_err(),
            ValidationError::InvalidAdapterRole
        );
    }

    #[test]
    fn mode_parses_and_formats_all_scoped_modes() {
        for (wire, expected) in MODE_CASES {
            assert_eq!(AdapterMode::from_str(wire).unwrap(), *expected);
            assert_eq!(AdapterMode::try_from(*wire).unwrap(), *expected);
            assert_eq!(expected.as_str(), *wire);
            assert_eq!(expected.to_string(), *wire);
        }

        assert_eq!(
            AdapterMode::from_str("unsafe_full_access").unwrap_err(),
            ValidationError::InvalidAdapterMode
        );
    }

    #[test]
    fn adapter_mode_capabilities_follow_rollout_boundaries() {
        let cases = [
            (AdapterMode::Disabled, false, false),
            (AdapterMode::ReadOnly, true, false),
            (AdapterMode::ImportOnly, false, true),
            (AdapterMode::ExportOnly, true, false),
            (AdapterMode::Bidirectional, true, true),
            (AdapterMode::DryRun, true, false),
        ];

        for (mode, can_read_core, can_write_core) in cases {
            assert_eq!(mode.allows_core_reads(), can_read_core, "mode={mode}");
            assert_eq!(mode.allows_core_writes(), can_write_core, "mode={mode}");
        }
    }

    #[test]
    fn role_serde_wire_values_are_stable() {
        for (wire, role) in ROLE_CASES {
            let json = serde_json::to_string(role).unwrap();
            assert_eq!(json, format!("\"{wire}\""));
            assert_eq!(serde_json::from_str::<AdapterRole>(&json).unwrap(), *role);
        }

        assert!(serde_json::from_str::<AdapterRole>("\"owner\"").is_err());
    }

    #[test]
    fn mode_serde_wire_values_are_stable() {
        for (wire, mode) in MODE_CASES {
            let json = serde_json::to_string(mode).unwrap();
            assert_eq!(json, format!("\"{wire}\""));
            assert_eq!(serde_json::from_str::<AdapterMode>(&json).unwrap(), *mode);
        }

        assert!(serde_json::from_str::<AdapterMode>("\"unsafe_full_access\"").is_err());
    }
}
