//! Adapter role and mode primitives.

use crate::ValidationError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// V1 adapter authorization role.
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

    #[test]
    fn role_parses_and_formats() {
        assert_eq!(
            AdapterRole::from_str("obsidian_plugin").unwrap(),
            AdapterRole::ObsidianPlugin
        );
        assert_eq!(AdapterRole::GdriveAdapter.to_string(), "gdrive_adapter");
        assert_eq!(
            AdapterRole::from_str("root").unwrap_err(),
            ValidationError::InvalidAdapterRole
        );
    }

    #[test]
    fn mode_parses_and_formats_all_scoped_modes() {
        let cases = [
            ("disabled", AdapterMode::Disabled),
            ("read_only", AdapterMode::ReadOnly),
            ("import_only", AdapterMode::ImportOnly),
            ("export_only", AdapterMode::ExportOnly),
            ("bidirectional", AdapterMode::Bidirectional),
            ("dry_run", AdapterMode::DryRun),
        ];

        for (wire, expected) in cases {
            assert_eq!(AdapterMode::from_str(wire).unwrap(), expected);
            assert_eq!(expected.to_string(), wire);
        }

        assert_eq!(
            AdapterMode::from_str("unsafe_full_access").unwrap_err(),
            ValidationError::InvalidAdapterMode
        );
    }

    #[test]
    fn serde_roundtrips() {
        let role_json = serde_json::to_string(&AdapterRole::WorktreeAdapter).unwrap();
        assert_eq!(role_json, "\"worktree_adapter\"");
        assert_eq!(
            serde_json::from_str::<AdapterRole>(&role_json).unwrap(),
            AdapterRole::WorktreeAdapter
        );

        let mode_json = serde_json::to_string(&AdapterMode::DryRun).unwrap();
        assert_eq!(mode_json, "\"dry_run\"");
        assert_eq!(
            serde_json::from_str::<AdapterMode>(&mode_json).unwrap(),
            AdapterMode::DryRun
        );
    }
}
