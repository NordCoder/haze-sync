//! Typed server configuration values and env-based parsing helpers.

use super::{env, ConfigError};
use std::{collections::HashMap, env as std_env, fmt, net::SocketAddr, path::PathBuf, str::FromStr};

/// Complete server config primitive set for later runtime wiring.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerConfig {
    /// TCP listen address. Parsing only; not wired into server startup here.
    pub listen_addr: SocketAddr,
    /// Database URL placeholder config.
    pub database: DatabaseConfig,
    /// Content-addressed object store root path config.
    pub object_store: ObjectStoreConfig,
    /// Built-in worktree root path and adapter mode config.
    pub worktree: WorktreeConfig,
    /// Adapter rollout modes known to the server/API layer.
    pub adapter_modes: AdapterModesConfig,
}

impl ServerConfig {
    /// Loads config from process environment variables.
    ///
    /// This function does not read `.env` files, log values, open sockets,
    /// connect to databases, or touch the filesystem.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std_env::var(name).ok())
    }

    /// Loads config from a supplied key/value iterator for deterministic tests.
    pub fn from_env_map<I, K, V>(vars: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let vars: HashMap<String, String> = vars
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();

        Self::from_lookup(|name| vars.get(name).cloned())
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let listen_addr = parse_or_default(
            env::HAZE_SYNC_LISTEN_ADDR,
            lookup(env::HAZE_SYNC_LISTEN_ADDR),
            env::DEFAULT_LISTEN_ADDR,
            parse_socket_addr,
        )?;
        let database_url = required_non_empty(
            env::HAZE_SYNC_DATABASE_URL,
            lookup(env::HAZE_SYNC_DATABASE_URL),
        )?;
        let object_store_root = PathBuf::from(value_or_default(
            lookup(env::HAZE_SYNC_OBJECT_STORE_PATH),
            env::DEFAULT_OBJECT_STORE_PATH,
        ));
        let worktree_root = PathBuf::from(value_or_default(
            lookup(env::HAZE_SYNC_WORKTREE_PATH),
            env::DEFAULT_WORKTREE_PATH,
        ));
        let worktree_mode = parse_or_default(
            env::HAZE_SYNC_WORKTREE_ADAPTER_MODE,
            lookup(env::HAZE_SYNC_WORKTREE_ADAPTER_MODE),
            env::DEFAULT_ADAPTER_MODE,
            AdapterMode::from_str,
        )?;
        let gdrive_mode = parse_or_default(
            env::HAZE_GDRIVE_ADAPTER_MODE,
            lookup(env::HAZE_GDRIVE_ADAPTER_MODE),
            env::DEFAULT_ADAPTER_MODE,
            AdapterMode::from_str,
        )?;
        let obsidian_mode = parse_or_default(
            env::HAZE_OBSIDIAN_ADAPTER_MODE,
            lookup(env::HAZE_OBSIDIAN_ADAPTER_MODE),
            env::DEFAULT_ADAPTER_MODE,
            AdapterMode::from_str,
        )?;

        Ok(Self {
            listen_addr,
            database: DatabaseConfig {
                database_url: DatabaseUrl::new(database_url)?,
            },
            object_store: ObjectStoreConfig {
                root: object_store_root,
            },
            worktree: WorktreeConfig {
                root: worktree_root,
                mode: worktree_mode,
            },
            adapter_modes: AdapterModesConfig {
                gdrive_adapter: gdrive_mode,
                worktree_adapter: worktree_mode,
                obsidian_plugin: obsidian_mode,
            },
        })
    }
}

/// Database config wrapper. The URL may contain credentials and is redacted in formatting.
#[derive(Clone, Eq, PartialEq)]
pub struct DatabaseConfig {
    /// Required PostgreSQL database URL placeholder.
    pub database_url: DatabaseUrl,
}

impl fmt::Debug for DatabaseConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DatabaseConfig")
            .field("database_url", &self.database_url)
            .finish()
    }
}

/// Redacted database URL wrapper.
#[derive(Clone, Eq, PartialEq)]
pub struct DatabaseUrl(String);

impl DatabaseUrl {
    /// Creates a redacted database URL wrapper from a non-empty URL value.
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ConfigError::MissingRequiredVar {
                name: env::HAZE_SYNC_DATABASE_URL,
            });
        }

        Ok(Self(value))
    }

    /// Returns the sensitive URL value for later database wiring only.
    #[must_use]
    pub fn as_sensitive_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for DatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DatabaseUrl([REDACTED])")
    }
}

impl fmt::Display for DatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

/// Content-addressed object store root path config.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectStoreConfig {
    /// Root directory for content-addressed blobs.
    pub root: PathBuf,
}

/// Built-in worktree config primitives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeConfig {
    /// Root directory for the materialized VPS worktree.
    pub root: PathBuf,
    /// Worktree adapter rollout mode.
    pub mode: AdapterMode,
}

/// Adapter rollout modes for config parsing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AdapterMode {
    /// Adapter does nothing.
    Disabled,
    /// Adapter can read Core but not write.
    ReadOnly,
    /// Adapter imports external changes into Core only.
    ImportOnly,
    /// Adapter exports Core changes to external replica only.
    ExportOnly,
    /// Adapter can import and export.
    Bidirectional,
    /// Adapter computes a plan but does not apply it.
    DryRun,
}

impl AdapterMode {
    /// Returns whether this mode allows writes into Core.
    #[must_use]
    pub const fn allows_core_writes(self) -> bool {
        matches!(self, Self::ImportOnly | Self::Bidirectional)
    }

    /// Returns whether this mode allows reads from Core for export/materialization.
    #[must_use]
    pub const fn allows_core_reads(self) -> bool {
        matches!(
            self,
            Self::ReadOnly | Self::ExportOnly | Self::Bidirectional | Self::DryRun
        )
    }
}

impl fmt::Display for AdapterMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Disabled => "disabled",
            Self::ReadOnly => "read_only",
            Self::ImportOnly => "import_only",
            Self::ExportOnly => "export_only",
            Self::Bidirectional => "bidirectional",
            Self::DryRun => "dry_run",
        })
    }
}

impl FromStr for AdapterMode {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "disabled" => Ok(Self::Disabled),
            "read_only" => Ok(Self::ReadOnly),
            "import_only" => Ok(Self::ImportOnly),
            "export_only" => Ok(Self::ExportOnly),
            "bidirectional" => Ok(Self::Bidirectional),
            "dry_run" => Ok(Self::DryRun),
            _ => Err(ConfigError::InvalidVar {
                name: "adapter mode",
                reason: "unknown adapter mode",
            }),
        }
    }
}

/// Adapter mode grouping for all server-visible adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterModesConfig {
    /// Google Drive API adapter mode.
    pub gdrive_adapter: AdapterMode,
    /// Built-in VPS worktree adapter mode.
    pub worktree_adapter: AdapterMode,
    /// Obsidian plugin adapter mode.
    pub obsidian_plugin: AdapterMode,
}

fn value_or_default(value: Option<String>, default: &'static str) -> String {
    value.filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_non_empty(
    name: &'static str,
    value: Option<String>,
) -> Result<String, ConfigError> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::MissingRequiredVar { name }),
    }
}

fn parse_or_default<T>(
    name: &'static str,
    value: Option<String>,
    default: &'static str,
    parser: impl FnOnce(&str) -> Result<T, ConfigError>,
) -> Result<T, ConfigError> {
    let value = value_or_default(value, default);
    parser(&value).map_err(|_| ConfigError::InvalidVar {
        name,
        reason: "value could not be parsed",
    })
}

fn parse_socket_addr(value: &str) -> Result<SocketAddr, ConfigError> {
    value.parse().map_err(|_| ConfigError::InvalidVar {
        name: env::HAZE_SYNC_LISTEN_ADDR,
        reason: "expected host:port socket address",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env_config_happy_path() {
        let config = ServerConfig::from_env_map([
            (env::HAZE_SYNC_DATABASE_URL, "postgres://haze-sync.invalid/haze_sync"),
            (env::HAZE_SYNC_LISTEN_ADDR, "127.0.0.1:9090"),
            (env::HAZE_SYNC_OBJECT_STORE_PATH, "/srv/haze-sync/objects"),
            (env::HAZE_SYNC_WORKTREE_PATH, "/srv/haze-vault/worktree"),
            (env::HAZE_GDRIVE_ADAPTER_MODE, "import_only"),
            (env::HAZE_SYNC_WORKTREE_ADAPTER_MODE, "export_only"),
            (env::HAZE_OBSIDIAN_ADAPTER_MODE, "read_only"),
        ])
        .expect("config should parse");

        assert_eq!(config.listen_addr, "127.0.0.1:9090".parse().unwrap());
        assert_eq!(config.object_store.root, PathBuf::from("/srv/haze-sync/objects"));
        assert_eq!(config.worktree.root, PathBuf::from("/srv/haze-vault/worktree"));
        assert_eq!(config.adapter_modes.gdrive_adapter, AdapterMode::ImportOnly);
        assert_eq!(config.adapter_modes.worktree_adapter, AdapterMode::ExportOnly);
        assert_eq!(config.adapter_modes.obsidian_plugin, AdapterMode::ReadOnly);
    }

    #[test]
    fn requires_database_url() {
        let error = ServerConfig::from_env_map([(env::HAZE_SYNC_LISTEN_ADDR, "127.0.0.1:8080")])
            .expect_err("database URL should be required");

        assert_eq!(
            error,
            ConfigError::MissingRequiredVar {
                name: env::HAZE_SYNC_DATABASE_URL
            }
        );
    }

    #[test]
    fn uses_safe_defaults_aligned_with_env_example() {
        let config = ServerConfig::from_env_map([(
            env::HAZE_SYNC_DATABASE_URL,
            "postgres://haze-sync.invalid/haze_sync",
        )])
        .expect("config should parse with safe defaults");

        assert_eq!(config.listen_addr, "127.0.0.1:8080".parse().unwrap());
        assert_eq!(config.object_store.root, PathBuf::from("./data/objects"));
        assert_eq!(config.worktree.root, PathBuf::from("./data/worktree"));
        assert_eq!(config.adapter_modes.gdrive_adapter, AdapterMode::Disabled);
        assert_eq!(config.adapter_modes.worktree_adapter, AdapterMode::Disabled);
        assert_eq!(config.adapter_modes.obsidian_plugin, AdapterMode::Disabled);
    }

    #[test]
    fn invalid_listen_addr_does_not_echo_value() {
        let invalid_value = "invalid-listen-value";
        let error = ServerConfig::from_env_map([
            (env::HAZE_SYNC_DATABASE_URL, "postgres://haze-sync.invalid/haze_sync"),
            (env::HAZE_SYNC_LISTEN_ADDR, invalid_value),
        ])
        .expect_err("listen address should fail");

        let rendered = error.to_string();
        assert!(rendered.contains(env::HAZE_SYNC_LISTEN_ADDR));
        assert!(!rendered.contains(invalid_value));
    }

    #[test]
    fn database_url_formatting_is_redacted() {
        let raw_value = "postgres://user:password@postgres.invalid/haze_sync";
        let database_url = DatabaseUrl::new(raw_value).expect("database URL should parse");

        let debug = format!("{database_url:?}");
        let display = database_url.to_string();

        assert!(debug.contains("[REDACTED]"));
        assert_eq!(display, "[REDACTED]");
        assert!(!debug.contains(raw_value));
        assert!(!display.contains(raw_value));
    }

    #[test]
    fn adapter_mode_parse_and_display_roundtrip() {
        let mode = "bidirectional"
            .parse::<AdapterMode>()
            .expect("mode should parse");

        assert_eq!(mode, AdapterMode::Bidirectional);
        assert_eq!(mode.to_string(), "bidirectional");
        assert!(AdapterMode::ImportOnly.allows_core_writes());
        assert!(!AdapterMode::Disabled.allows_core_reads());
    }
}
