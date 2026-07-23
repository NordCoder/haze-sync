//! Configuration loading for the Google Drive adapter.
//!
//! `HAZE_GDRIVE_MODE` is the single authoritative mode input. The legacy
//! `HAZE_GDRIVE_DRY_RUN` boolean is accepted only as a fail-closed compatibility
//! assertion that the selected mode is `dry_run`.

use crate::error::ConfigError;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const ENV_SERVER_URL: &str = "HAZE_GDRIVE_SERVER_URL";
pub const ENV_ADAPTER_TOKEN: &str = "HAZE_GDRIVE_ADAPTER_TOKEN";
pub const ENV_ADAPTER_TOKEN_FILE: &str = "HAZE_GDRIVE_ADAPTER_TOKEN_FILE";
pub const ENV_DRIVE_ROOT_FOLDER_ID: &str = "HAZE_GDRIVE_ROOT_FOLDER_ID";
pub const ENV_OAUTH_TOKEN_FILE: &str = "HAZE_GDRIVE_OAUTH_TOKEN_FILE";
pub const ENV_ADAPTER_MODE: &str = "HAZE_GDRIVE_MODE";
pub const ENV_DRY_RUN: &str = "HAZE_GDRIVE_DRY_RUN";
pub const ENV_POLL_INTERVAL_SECONDS: &str = "HAZE_GDRIVE_POLL_INTERVAL_SECONDS";
pub const ENV_FULL_SCAN_INTERVAL_SECONDS: &str = "HAZE_GDRIVE_FULL_SCAN_INTERVAL_SECONDS";
pub const ENV_MAX_DELETES_PER_RUN: &str = "HAZE_GDRIVE_MAX_DELETES_PER_RUN";
pub const ENV_MAX_DELETE_RATIO_PERCENT: &str = "HAZE_GDRIVE_MAX_DELETE_RATIO_PERCENT";

const ENV_ADAPTER_TOKEN_SOURCE: &str = "HAZE_GDRIVE_ADAPTER_TOKEN|HAZE_GDRIVE_ADAPTER_TOKEN_FILE";
const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 60;
const DEFAULT_FULL_SCAN_INTERVAL_SECONDS: u64 = 3_600;
const DEFAULT_MAX_DELETES_PER_RUN: u32 = 10;
const DEFAULT_MAX_DELETE_RATIO_PERCENT: u8 = 10;

#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn from_raw(key: &'static str, raw: impl Into<String>) -> Result<Self, ConfigError> {
        let value = raw.into().trim().to_owned();
        if value.is_empty() {
            return Err(ConfigError::invalid(key, "secret value must not be empty"));
        }
        Ok(Self(value))
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted-secret>")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted-secret>")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretPath(PathBuf);

impl SecretPath {
    pub fn from_raw(key: &'static str, raw: impl Into<String>) -> Result<Self, ConfigError> {
        let raw = raw.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ConfigError::invalid(
                key,
                "secret file path must not be empty",
            ));
        }
        let path = PathBuf::from(trimmed);
        if !path.is_absolute() {
            return Err(ConfigError::invalid(
                key,
                "secret file path must be absolute",
            ));
        }
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Debug for SecretPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted-secret-path>")
    }
}

impl fmt::Display for SecretPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted-secret-path>")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdapterMode {
    Disabled,
    #[default]
    DryRun,
    ReadOnly,
    ImportOnly,
    ExportOnly,
    Bidirectional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeCapabilities {
    pub provider_reads: bool,
    pub core_reads: bool,
    pub core_writes: bool,
    pub provider_writes: bool,
    pub provider_trash: bool,
    pub durable_state_mutation: bool,
}

impl AdapterMode {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "disabled" => Some(Self::Disabled),
            "dry_run" => Some(Self::DryRun),
            "read_only" => Some(Self::ReadOnly),
            "import_only" => Some(Self::ImportOnly),
            "export_only" => Some(Self::ExportOnly),
            "bidirectional" => Some(Self::Bidirectional),
            _ => None,
        }
    }

    pub const fn capabilities(self) -> ModeCapabilities {
        match self {
            Self::Disabled => ModeCapabilities {
                provider_reads: false,
                core_reads: false,
                core_writes: false,
                provider_writes: false,
                provider_trash: false,
                durable_state_mutation: false,
            },
            Self::DryRun | Self::ReadOnly => ModeCapabilities {
                provider_reads: true,
                core_reads: true,
                core_writes: false,
                provider_writes: false,
                provider_trash: false,
                durable_state_mutation: false,
            },
            Self::ImportOnly => ModeCapabilities {
                provider_reads: true,
                core_reads: true,
                core_writes: true,
                provider_writes: false,
                provider_trash: false,
                durable_state_mutation: true,
            },
            Self::ExportOnly => ModeCapabilities {
                provider_reads: true,
                core_reads: true,
                core_writes: false,
                provider_writes: true,
                provider_trash: true,
                durable_state_mutation: true,
            },
            Self::Bidirectional => ModeCapabilities {
                provider_reads: true,
                core_reads: true,
                core_writes: true,
                provider_writes: true,
                provider_trash: true,
                durable_state_mutation: true,
            },
        }
    }

    pub const fn is_dry_run_mode(self) -> bool {
        matches!(self, Self::DryRun)
    }

    pub const fn permits_provider_reads(self) -> bool {
        self.capabilities().provider_reads
    }

    pub const fn permits_core_reads(self) -> bool {
        self.capabilities().core_reads
    }

    pub const fn permits_core_writes(self) -> bool {
        self.capabilities().core_writes
    }

    pub const fn permits_provider_writes(self) -> bool {
        self.capabilities().provider_writes
    }

    pub const fn permits_provider_trash(self) -> bool {
        self.capabilities().provider_trash
    }

    pub const fn permits_durable_state_mutation(self) -> bool {
        self.capabilities().durable_state_mutation
    }
}

impl fmt::Display for AdapterMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Disabled => "disabled",
            Self::DryRun => "dry_run",
            Self::ReadOnly => "read_only",
            Self::ImportOnly => "import_only",
            Self::ExportOnly => "export_only",
            Self::Bidirectional => "bidirectional",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeIntervals {
    pub poll_interval: Duration,
    pub full_scan_interval: Duration,
}

impl RuntimeIntervals {
    pub fn new(poll_interval: Duration, full_scan_interval: Duration) -> Result<Self, ConfigError> {
        if poll_interval.is_zero() {
            return Err(ConfigError::invalid(
                ENV_POLL_INTERVAL_SECONDS,
                "poll interval must be greater than zero seconds",
            ));
        }
        if full_scan_interval.is_zero() {
            return Err(ConfigError::invalid(
                ENV_FULL_SCAN_INTERVAL_SECONDS,
                "full scan interval must be greater than zero seconds",
            ));
        }
        if full_scan_interval < poll_interval {
            return Err(ConfigError::invalid(
                ENV_FULL_SCAN_INTERVAL_SECONDS,
                "full scan interval must be greater than or equal to poll interval",
            ));
        }
        Ok(Self {
            poll_interval,
            full_scan_interval,
        })
    }

    pub fn poll_interval_seconds(&self) -> u64 {
        self.poll_interval.as_secs()
    }

    pub fn full_scan_interval_seconds(&self) -> u64 {
        self.full_scan_interval.as_secs()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteSafetyConfig {
    pub max_deletes_per_run: u32,
    pub max_delete_ratio_percent: u8,
}

impl DeleteSafetyConfig {
    pub fn new(
        max_deletes_per_run: u32,
        max_delete_ratio_percent: u8,
    ) -> Result<Self, ConfigError> {
        if max_delete_ratio_percent > 100 {
            return Err(ConfigError::invalid(
                ENV_MAX_DELETE_RATIO_PERCENT,
                "delete ratio must be between 0 and 100 percent",
            ));
        }
        Ok(Self {
            max_deletes_per_run,
            max_delete_ratio_percent,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AdapterConfig {
    pub server_url: String,
    pub adapter_token: SecretString,
    pub drive_root_folder_id: String,
    pub oauth_token_path: SecretPath,
    pub mode: AdapterMode,
    pub intervals: RuntimeIntervals,
    pub delete_safety: DeleteSafetyConfig,
}

impl fmt::Debug for AdapterConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdapterConfig")
            .field("server_url", &"<redacted-server-endpoint>")
            .field("adapter_token", &self.adapter_token)
            .field("drive_root_folder_id", &"<redacted-provider-root>")
            .field("oauth_token_path", &self.oauth_token_path)
            .field("mode", &self.mode)
            .field("intervals", &self.intervals)
            .field("delete_safety", &self.delete_safety)
            .finish()
    }
}

impl AdapterConfig {
    pub fn load_from_env() -> Result<Self, ConfigError> {
        Self::load_from_source(&EnvConfigSource)
    }

    pub const fn is_dry_run(&self) -> bool {
        self.mode.is_dry_run_mode()
    }

    pub fn load_from_source(source: &impl ConfigSource) -> Result<Self, ConfigError> {
        let mode = load_mode(source)?;
        let server_url = required_value(source, ENV_SERVER_URL)?;
        validate_server_url(&server_url)?;
        let adapter_token = load_adapter_token(source)?;
        let drive_root_folder_id = required_value(source, ENV_DRIVE_ROOT_FOLDER_ID)?;
        validate_drive_root_folder_id(&drive_root_folder_id)?;
        let oauth_token_path = SecretPath::from_raw(
            ENV_OAUTH_TOKEN_FILE,
            required_value(source, ENV_OAUTH_TOKEN_FILE)?,
        )?;
        if !source.secret_path_is_file(ENV_OAUTH_TOKEN_FILE, oauth_token_path.as_path())? {
            return Err(ConfigError::invalid(
                ENV_OAUTH_TOKEN_FILE,
                "secret file path must identify a readable file",
            ));
        }
        let poll_interval = parse_optional_u64(
            source,
            ENV_POLL_INTERVAL_SECONDS,
            DEFAULT_POLL_INTERVAL_SECONDS,
        )?;
        let full_scan_interval = parse_optional_u64(
            source,
            ENV_FULL_SCAN_INTERVAL_SECONDS,
            DEFAULT_FULL_SCAN_INTERVAL_SECONDS,
        )?;
        let intervals = RuntimeIntervals::new(
            Duration::from_secs(poll_interval),
            Duration::from_secs(full_scan_interval),
        )?;
        let max_deletes_per_run =
            parse_optional_u32(source, ENV_MAX_DELETES_PER_RUN, DEFAULT_MAX_DELETES_PER_RUN)?;
        let max_delete_ratio_percent = parse_optional_u8(
            source,
            ENV_MAX_DELETE_RATIO_PERCENT,
            DEFAULT_MAX_DELETE_RATIO_PERCENT,
        )?;
        let delete_safety = DeleteSafetyConfig::new(max_deletes_per_run, max_delete_ratio_percent)?;
        Ok(Self {
            server_url,
            adapter_token,
            drive_root_folder_id,
            oauth_token_path,
            mode,
            intervals,
            delete_safety,
        })
    }
}

fn load_mode(source: &impl ConfigSource) -> Result<AdapterMode, ConfigError> {
    let mode = match optional_value(source, ENV_ADAPTER_MODE) {
        Some(raw) => AdapterMode::parse(&raw).ok_or_else(|| {
            ConfigError::invalid(
                ENV_ADAPTER_MODE,
                "mode must be disabled, dry_run, read_only, import_only, export_only, or bidirectional",
            )
        })?,
        None => AdapterMode::default(),
    };

    if let Some(raw) = optional_value(source, ENV_DRY_RUN) {
        let legacy_dry_run = parse_bool(ENV_DRY_RUN, &raw)?;
        if !legacy_dry_run || !mode.is_dry_run_mode() {
            return Err(ConfigError::invalid(
                ENV_DRY_RUN,
                "legacy dry-run compatibility is valid only as true with mode=dry_run",
            ));
        }
    }
    Ok(mode)
}

pub trait ConfigSource {
    fn value(&self, key: &'static str) -> Option<String>;
    fn read_secret_file(&self, key: &'static str, path: &Path) -> Result<String, ConfigError>;
    fn secret_path_is_file(&self, key: &'static str, path: &Path) -> Result<bool, ConfigError>;
}

pub struct EnvConfigSource;

impl ConfigSource for EnvConfigSource {
    fn value(&self, key: &'static str) -> Option<String> {
        env::var(key).ok()
    }

    fn read_secret_file(&self, key: &'static str, path: &Path) -> Result<String, ConfigError> {
        fs::read_to_string(path).map_err(|_| ConfigError::secret_file_read_failed(key))
    }

    fn secret_path_is_file(&self, _key: &'static str, path: &Path) -> Result<bool, ConfigError> {
        Ok(path.is_file())
    }
}

fn load_adapter_token(source: &impl ConfigSource) -> Result<SecretString, ConfigError> {
    match (
        optional_value(source, ENV_ADAPTER_TOKEN),
        optional_value(source, ENV_ADAPTER_TOKEN_FILE),
    ) {
        (Some(_), Some(_)) => Err(ConfigError::ambiguous_secret_source(
            ENV_ADAPTER_TOKEN_SOURCE,
        )),
        (Some(token), None) => SecretString::from_raw(ENV_ADAPTER_TOKEN, token),
        (None, Some(path)) => {
            let secret_path = SecretPath::from_raw(ENV_ADAPTER_TOKEN_FILE, path)?;
            let token = source.read_secret_file(ENV_ADAPTER_TOKEN_FILE, secret_path.as_path())?;
            SecretString::from_raw(ENV_ADAPTER_TOKEN_FILE, token)
        }
        (None, None) => Err(ConfigError::missing(
            ENV_ADAPTER_TOKEN_SOURCE,
            "adapter token must be provided directly or through a secret file",
        )),
    }
}

fn required_value(source: &impl ConfigSource, key: &'static str) -> Result<String, ConfigError> {
    optional_value(source, key).ok_or_else(|| ConfigError::missing(key, "value is required"))
}

fn optional_value(source: &impl ConfigSource, key: &'static str) -> Option<String> {
    source
        .value(key)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn validate_server_url(server_url: &str) -> Result<(), ConfigError> {
    let valid_scheme = server_url.starts_with("http://") || server_url.starts_with("https://");
    if !valid_scheme || server_url.chars().any(char::is_whitespace) {
        return Err(ConfigError::invalid(
            ENV_SERVER_URL,
            "server URL must be an http:// or https:// URL without whitespace",
        ));
    }
    Ok(())
}

fn validate_drive_root_folder_id(value: &str) -> Result<(), ConfigError> {
    if value.contains('/') || value.contains('\\') || value.chars().any(char::is_whitespace) {
        return Err(ConfigError::invalid(
            ENV_DRIVE_ROOT_FOLDER_ID,
            "Drive root folder id must not contain whitespace or path separators",
        ));
    }
    Ok(())
}

fn parse_bool(key: &'static str, raw: &str) -> Result<bool, ConfigError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::invalid(
            key,
            "boolean value must be true/false, yes/no, on/off, or 1/0",
        )),
    }
}

fn parse_optional_u64(
    source: &impl ConfigSource,
    key: &'static str,
    default_value: u64,
) -> Result<u64, ConfigError> {
    optional_value(source, key)
        .map(|raw| {
            raw.parse::<u64>()
                .map_err(|_| ConfigError::invalid(key, "value must be an unsigned integer"))
        })
        .unwrap_or(Ok(default_value))
}

fn parse_optional_u32(
    source: &impl ConfigSource,
    key: &'static str,
    default_value: u32,
) -> Result<u32, ConfigError> {
    optional_value(source, key)
        .map(|raw| {
            raw.parse::<u32>()
                .map_err(|_| ConfigError::invalid(key, "value must be an unsigned integer"))
        })
        .unwrap_or(Ok(default_value))
}

fn parse_optional_u8(
    source: &impl ConfigSource,
    key: &'static str,
    default_value: u8,
) -> Result<u8, ConfigError> {
    optional_value(source, key)
        .map(|raw| {
            raw.parse::<u8>()
                .map_err(|_| ConfigError::invalid(key, "value must be an unsigned integer"))
        })
        .unwrap_or(Ok(default_value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ConfigErrorCategory;
    use std::collections::{BTreeMap, BTreeSet};

    #[derive(Default)]
    struct MemoryConfigSource {
        values: BTreeMap<&'static str, String>,
        secret_files: BTreeMap<String, String>,
        existing_files: BTreeSet<String>,
    }

    impl MemoryConfigSource {
        fn with_required_values() -> Self {
            let mut source = Self::default();
            source.insert(ENV_SERVER_URL, "https://sentinel-endpoint.example.test");
            source.insert(ENV_ADAPTER_TOKEN, "sentinel-adapter-token");
            source.insert(ENV_DRIVE_ROOT_FOLDER_ID, "sentinel-provider-root");
            source.insert(ENV_OAUTH_TOKEN_FILE, "/run/secrets/sentinel-oauth.json");
            source
                .existing_files
                .insert("/run/secrets/sentinel-oauth.json".to_owned());
            source
        }

        fn insert(&mut self, key: &'static str, value: impl Into<String>) {
            self.values.insert(key, value.into());
        }

        fn insert_secret_file(&mut self, path: impl Into<String>, value: impl Into<String>) {
            self.secret_files.insert(path.into(), value.into());
        }
    }

    impl ConfigSource for MemoryConfigSource {
        fn value(&self, key: &'static str) -> Option<String> {
            self.values.get(key).cloned()
        }

        fn read_secret_file(&self, key: &'static str, path: &Path) -> Result<String, ConfigError> {
            self.secret_files
                .get(&path.to_string_lossy().into_owned())
                .cloned()
                .ok_or_else(|| ConfigError::secret_file_read_failed(key))
        }

        fn secret_path_is_file(
            &self,
            _key: &'static str,
            path: &Path,
        ) -> Result<bool, ConfigError> {
            Ok(self
                .existing_files
                .contains(&path.to_string_lossy().into_owned()))
        }
    }

    #[test]
    fn loads_required_config_with_safe_defaults() {
        let config =
            AdapterConfig::load_from_source(&MemoryConfigSource::with_required_values()).unwrap();
        assert_eq!(config.mode, AdapterMode::DryRun);
        assert!(config.is_dry_run());
        assert_eq!(config.intervals.poll_interval_seconds(), 60);
        assert_eq!(config.intervals.full_scan_interval_seconds(), 3_600);
    }

    #[test]
    fn parses_every_authoritative_mode_and_keeps_capabilities_closed() {
        for (raw, expected) in [
            ("disabled", AdapterMode::Disabled),
            ("dry_run", AdapterMode::DryRun),
            ("read_only", AdapterMode::ReadOnly),
            ("import_only", AdapterMode::ImportOnly),
            ("export_only", AdapterMode::ExportOnly),
            ("bidirectional", AdapterMode::Bidirectional),
        ] {
            let mut source = MemoryConfigSource::with_required_values();
            source.insert(ENV_ADAPTER_MODE, raw);
            let config = AdapterConfig::load_from_source(&source).unwrap();
            assert_eq!(config.mode, expected);
        }
        for mode in [AdapterMode::DryRun, AdapterMode::ReadOnly] {
            assert!(mode.permits_core_reads());
            assert!(!mode.permits_durable_state_mutation());
        }
    }

    #[test]
    fn legacy_dry_run_is_only_a_consistent_assertion() {
        let mut accepted = MemoryConfigSource::with_required_values();
        accepted.insert(ENV_ADAPTER_MODE, "dry_run");
        accepted.insert(ENV_DRY_RUN, "true");
        assert!(AdapterConfig::load_from_source(&accepted).is_ok());

        for (mode, legacy) in [
            ("disabled", "true"),
            ("read_only", "true"),
            ("import_only", "true"),
            ("export_only", "true"),
            ("bidirectional", "true"),
            ("dry_run", "false"),
        ] {
            let mut source = MemoryConfigSource::with_required_values();
            source.insert(ENV_ADAPTER_MODE, mode);
            source.insert(ENV_DRY_RUN, legacy);
            let error = AdapterConfig::load_from_source(&source).unwrap_err();
            assert_eq!(error.key(), ENV_DRY_RUN);
            assert_eq!(error.category(), ConfigErrorCategory::Invalid);
        }
    }

    #[test]
    fn loads_adapter_token_from_secret_file_and_rejects_ambiguous_sources() {
        let mut file_source = MemoryConfigSource::with_required_values();
        file_source.values.remove(ENV_ADAPTER_TOKEN);
        file_source.insert(ENV_ADAPTER_TOKEN_FILE, "/run/secrets/adapter-token");
        file_source.insert_secret_file("/run/secrets/adapter-token", "token-from-file\n");
        assert_eq!(
            AdapterConfig::load_from_source(&file_source)
                .unwrap()
                .adapter_token
                .expose_secret(),
            "token-from-file"
        );

        let mut ambiguous = MemoryConfigSource::with_required_values();
        ambiguous.insert(ENV_ADAPTER_TOKEN_FILE, "/run/secrets/adapter-token");
        let error = AdapterConfig::load_from_source(&ambiguous).unwrap_err();
        assert_eq!(error.category(), ConfigErrorCategory::AmbiguousSecretSource);
        assert!(!error.to_string().contains("sentinel-adapter-token"));
        assert!(!error.to_string().contains("/run/secrets"));
    }

    #[test]
    fn config_debug_redacts_endpoint_provider_root_token_and_secret_path() {
        let config =
            AdapterConfig::load_from_source(&MemoryConfigSource::with_required_values()).unwrap();
        let rendered = format!("{config:?}");
        for sentinel in [
            "sentinel-endpoint.example.test",
            "sentinel-provider-root",
            "sentinel-adapter-token",
            "/run/secrets/sentinel-oauth.json",
        ] {
            assert!(!rendered.contains(sentinel));
        }
        assert!(rendered.contains("<redacted-server-endpoint>"));
        assert!(rendered.contains("<redacted-provider-root>"));
        assert!(rendered.contains("DryRun"));
    }

    #[test]
    fn validates_delete_safety_ratio() {
        let mut source = MemoryConfigSource::with_required_values();
        source.insert(ENV_MAX_DELETE_RATIO_PERCENT, "101");
        let error = AdapterConfig::load_from_source(&source).unwrap_err();
        assert_eq!(error.key(), ENV_MAX_DELETE_RATIO_PERCENT);
    }
}
