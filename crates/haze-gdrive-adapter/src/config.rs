//! Configuration loading for the Google Drive adapter.
//!
//! This module intentionally supports only explicit environment/file inputs. It
//! never formats secret values or secret file paths into public strings.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterMode {
    Disabled,
    ReadOnly,
    ImportOnly,
    ExportOnly,
    Bidirectional,
    DryRun,
}

impl AdapterMode {
    fn parse(raw: &str) -> Option<Self> {
        let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "disabled" => Some(Self::Disabled),
            "read_only" | "readonly" => Some(Self::ReadOnly),
            "import_only" | "importonly" => Some(Self::ImportOnly),
            "export_only" | "exportonly" => Some(Self::ExportOnly),
            "bidirectional" => Some(Self::Bidirectional),
            "dry_run" | "dryrun" => Some(Self::DryRun),
            _ => None,
        }
    }

    pub const fn is_dry_run_mode(self) -> bool {
        matches!(self, Self::DryRun)
    }
}

impl Default for AdapterMode {
    fn default() -> Self {
        Self::DryRun
    }
}

impl fmt::Display for AdapterMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Disabled => "disabled",
            Self::ReadOnly => "read_only",
            Self::ImportOnly => "import_only",
            Self::ExportOnly => "export_only",
            Self::Bidirectional => "bidirectional",
            Self::DryRun => "dry_run",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeIntervals {
    pub poll_interval: Duration,
    pub full_scan_interval: Duration,
}

impl RuntimeIntervals {
    pub fn new(
        poll_interval: Duration,
        full_scan_interval: Duration,
    ) -> Result<Self, ConfigError> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterConfig {
    pub server_url: String,
    pub adapter_token: SecretString,
    pub drive_root_folder_id: String,
    pub oauth_token_path: SecretPath,
    pub mode: AdapterMode,
    pub dry_run: bool,
    pub intervals: RuntimeIntervals,
    pub delete_safety: DeleteSafetyConfig,
}

impl AdapterConfig {
    pub fn load_from_env() -> Result<Self, ConfigError> {
        Self::load_from_source(&EnvConfigSource)
    }

    pub fn load_from_source(source: &impl ConfigSource) -> Result<Self, ConfigError> {
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

        let mode = match optional_value(source, ENV_ADAPTER_MODE) {
            Some(raw) => AdapterMode::parse(&raw).ok_or_else(|| {
                ConfigError::invalid(
                    ENV_ADAPTER_MODE,
                    "mode must be disabled, read_only, import_only, export_only, bidirectional, or dry_run",
                )
            })?,
            None => AdapterMode::default(),
        };

        let dry_run = match optional_value(source, ENV_DRY_RUN) {
            Some(raw) => parse_bool(ENV_DRY_RUN, &raw)?,
            None => mode.is_dry_run_mode(),
        };

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

        let max_deletes_per_run = parse_optional_u32(
            source,
            ENV_MAX_DELETES_PER_RUN,
            DEFAULT_MAX_DELETES_PER_RUN,
        )?;
        let max_delete_ratio_percent = parse_optional_u8(
            source,
            ENV_MAX_DELETE_RATIO_PERCENT,
            DEFAULT_MAX_DELETE_RATIO_PERCENT,
        )?;
        let delete_safety =
            DeleteSafetyConfig::new(max_deletes_per_run, max_delete_ratio_percent)?;

        Ok(Self {
            server_url,
            adapter_token,
            drive_root_folder_id,
            oauth_token_path,
            mode,
            dry_run,
            intervals,
            delete_safety,
        })
    }
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
    let direct_token = optional_value(source, ENV_ADAPTER_TOKEN);
    let token_file = optional_value(source, ENV_ADAPTER_TOKEN_FILE);

    match (direct_token, token_file) {
        (Some(_), Some(_)) => Err(ConfigError::ambiguous_secret_source(ENV_ADAPTER_TOKEN_SOURCE)),
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
    let has_scheme = server_url.starts_with("http://") || server_url.starts_with("https://");
    if !has_scheme || server_url.chars().any(char::is_whitespace) {
        return Err(ConfigError::invalid(
            ENV_SERVER_URL,
            "server URL must be an http:// or https:// URL without whitespace",
        ));
    }
    Ok(())
}

fn validate_drive_root_folder_id(drive_root_folder_id: &str) -> Result<(), ConfigError> {
    let has_path_separator =
        drive_root_folder_id.contains('/') || drive_root_folder_id.contains('\\');
    if has_path_separator || drive_root_folder_id.chars().any(char::is_whitespace) {
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
    match optional_value(source, key) {
        Some(raw) => raw
            .parse::<u64>()
            .map_err(|_| ConfigError::invalid(key, "value must be an unsigned integer")),
        None => Ok(default_value),
    }
}

fn parse_optional_u32(
    source: &impl ConfigSource,
    key: &'static str,
    default_value: u32,
) -> Result<u32, ConfigError> {
    match optional_value(source, key) {
        Some(raw) => raw
            .parse::<u32>()
            .map_err(|_| ConfigError::invalid(key, "value must be an unsigned integer")),
        None => Ok(default_value),
    }
}

fn parse_optional_u8(
    source: &impl ConfigSource,
    key: &'static str,
    default_value: u8,
) -> Result<u8, ConfigError> {
    match optional_value(source, key) {
        Some(raw) => raw
            .parse::<u8>()
            .map_err(|_| ConfigError::invalid(key, "value must be an unsigned integer")),
        None => Ok(default_value),
    }
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
            source.insert(ENV_SERVER_URL, "https://sync.example.test");
            source.insert(ENV_ADAPTER_TOKEN, "adapter-token");
            source.insert(ENV_DRIVE_ROOT_FOLDER_ID, "driveRoot123");
            source.insert(ENV_OAUTH_TOKEN_FILE, "/run/secrets/gdrive-oauth-token.json");
            source
                .existing_files
                .insert("/run/secrets/gdrive-oauth-token.json".to_owned());
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
            let safe_key = path.to_string_lossy().into_owned();
            self.secret_files
                .get(&safe_key)
                .cloned()
                .ok_or_else(|| ConfigError::secret_file_read_failed(key))
        }

        fn secret_path_is_file(
            &self,
            _key: &'static str,
            path: &Path,
        ) -> Result<bool, ConfigError> {
            let safe_key = path.to_string_lossy().into_owned();
            Ok(self.existing_files.contains(&safe_key))
        }
    }

    #[test]
    fn loads_required_config_with_safe_defaults() {
        let source = MemoryConfigSource::with_required_values();

        let config = AdapterConfig::load_from_source(&source).expect("config should load");

        assert_eq!(config.server_url, "https://sync.example.test");
        assert_eq!(config.drive_root_folder_id, "driveRoot123");
        assert_eq!(config.mode, AdapterMode::DryRun);
        assert!(config.dry_run);
        assert_eq!(config.intervals.poll_interval_seconds(), 60);
        assert_eq!(config.intervals.full_scan_interval_seconds(), 3_600);
        assert_eq!(config.delete_safety.max_deletes_per_run, 10);
        assert_eq!(config.delete_safety.max_delete_ratio_percent, 10);
    }

    #[test]
    fn loads_adapter_token_from_secret_file() {
        let mut source = MemoryConfigSource::with_required_values();
        source.values.remove(ENV_ADAPTER_TOKEN);
        source.insert(ENV_ADAPTER_TOKEN_FILE, "/run/secrets/gdrive-adapter-token");
        source.insert_secret_file("/run/secrets/gdrive-adapter-token", "token-from-file\n");

        let config = AdapterConfig::load_from_source(&source).expect("config should load");

        assert_eq!(config.adapter_token.expose_secret(), "token-from-file");
    }

    #[test]
    fn rejects_ambiguous_adapter_token_sources() {
        let mut source = MemoryConfigSource::with_required_values();
        source.insert(ENV_ADAPTER_TOKEN_FILE, "/run/secrets/gdrive-adapter-token");

        let error = AdapterConfig::load_from_source(&source).expect_err("config must fail");

        assert_eq!(error.category(), ConfigErrorCategory::AmbiguousSecretSource);
        assert!(!error.to_string().contains("adapter-token"));
        assert!(!error.to_string().contains("/run/secrets"));
    }

    #[test]
    fn rejects_relative_secret_paths_without_exposing_path() {
        let mut source = MemoryConfigSource::with_required_values();
        source.insert(ENV_OAUTH_TOKEN_FILE, "local-secret.json");

        let error = AdapterConfig::load_from_source(&source).expect_err("config must fail");
        let rendered = error.to_string();

        assert_eq!(error.category(), ConfigErrorCategory::Invalid);
        assert!(!rendered.contains("local-secret.json"));
    }

    #[test]
    fn rejects_invalid_mode() {
        let mut source = MemoryConfigSource::with_required_values();
        source.insert(ENV_ADAPTER_MODE, "sync-everything");

        let error = AdapterConfig::load_from_source(&source).expect_err("config must fail");

        assert_eq!(error.key(), ENV_ADAPTER_MODE);
        assert_eq!(error.category(), ConfigErrorCategory::Invalid);
    }

    #[test]
    fn redacts_secret_debug_and_display() {
        let secret = SecretString::from_raw(ENV_ADAPTER_TOKEN, "super-secret").expect("secret");
        let path = SecretPath::from_raw(ENV_OAUTH_TOKEN_FILE, "/run/secrets/oauth.json")
            .expect("secret path");

        assert_eq!(secret.to_string(), "<redacted-secret>");
        assert_eq!(format!("{secret:?}"), "<redacted-secret>");
        assert_eq!(path.to_string(), "<redacted-secret-path>");
        assert_eq!(format!("{path:?}"), "<redacted-secret-path>");
    }

    #[test]
    fn validates_delete_safety_ratio() {
        let mut source = MemoryConfigSource::with_required_values();
        source.insert(ENV_MAX_DELETE_RATIO_PERCENT, "101");

        let error = AdapterConfig::load_from_source(&source).expect_err("config must fail");

        assert_eq!(error.key(), ENV_MAX_DELETE_RATIO_PERCENT);
        assert_eq!(error.category(), ConfigErrorCategory::Invalid);
    }
}
