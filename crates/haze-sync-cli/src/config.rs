#![allow(dead_code)]
//! CLI-local configuration and secret-source primitives.
//!
//! This module intentionally models where future network commands will read
//! operator configuration from without loading environment variables, opening
//! files, prompting stdin, calling OS keychains, or contacting the server. It
//! keeps raw secret-bearing values out of Debug/Display output by default.

use std::fmt;

const REDACTED: &str = "<redacted>";

static SERVER_URL_PRECEDENCE: [ConfigSource; 4] = [
    ConfigSource::CliFlag,
    ConfigSource::Environment,
    ConfigSource::ConfigFile,
    ConfigSource::BuiltInDefault,
];
static PROFILE_PRECEDENCE: [ConfigSource; 4] = [
    ConfigSource::CliFlag,
    ConfigSource::Environment,
    ConfigSource::ConfigFile,
    ConfigSource::BuiltInDefault,
];
static OUTPUT_FORMAT_PRECEDENCE: [ConfigSource; 4] = [
    ConfigSource::CliFlag,
    ConfigSource::Environment,
    ConfigSource::ConfigFile,
    ConfigSource::BuiltInDefault,
];
static TOKEN_SOURCE_PRECEDENCE: [ConfigSource; 4] = [
    ConfigSource::CliFlag,
    ConfigSource::Environment,
    ConfigSource::ConfigFile,
    ConfigSource::BuiltInDefault,
];

/// Config field with documented source precedence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigField {
    /// Server base URL for future network-backed commands.
    ServerUrl,
    /// Operator profile name.
    Profile,
    /// Human-readable or future machine-readable output mode.
    OutputFormat,
    /// Source from which an authentication token should be loaded.
    TokenSource,
}

/// Source category for CLI configuration values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigSource {
    /// Explicit CLI flag that selects a value or a safe source descriptor.
    CliFlag,
    /// Environment variable.
    Environment,
    /// CLI configuration file.
    ConfigFile,
    /// Built-in safe default.
    BuiltInDefault,
}

/// Return highest-to-lowest source precedence for a config field.
#[must_use]
pub const fn source_precedence(field: ConfigField) -> &'static [ConfigSource] {
    match field {
        ConfigField::ServerUrl => &SERVER_URL_PRECEDENCE,
        ConfigField::Profile => &PROFILE_PRECEDENCE,
        ConfigField::OutputFormat => &OUTPUT_FORMAT_PRECEDENCE,
        ConfigField::TokenSource => &TOKEN_SOURCE_PRECEDENCE,
    }
}

/// CLI output format selected by config.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OutputFormat {
    /// Stable human-oriented text output.
    #[default]
    Human,
    /// Future machine-readable JSON output.
    Json,
}

impl OutputFormat {
    /// Parse a safe output format name.
    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        match value.trim() {
            "human" | "text" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            "" => Err(ConfigError::EmptyOutputFormat),
            _ => Err(ConfigError::UnsupportedOutputFormat),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Human => "human",
            Self::Json => "json",
        };
        formatter.write_str(value)
    }
}

/// Safe profile identifier.
#[derive(Clone, Eq, PartialEq)]
pub struct ProfileName(String);

impl ProfileName {
    /// Parse a profile name suitable for CLI selection.
    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(ConfigError::EmptyProfileName);
        }
        if !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        }) {
            return Err(ConfigError::InvalidProfileName);
        }

        Ok(Self(value.to_owned()))
    }

    /// Return the safe profile identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ProfileName {
    fn default() -> Self {
        Self("default".to_owned())
    }
}

impl fmt::Debug for ProfileName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("ProfileName").field(&self.0).finish()
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Future server URL. The raw value is never exposed through Debug/Display.
#[derive(Clone, Eq, PartialEq)]
pub struct ServerUrl(String);

impl ServerUrl {
    /// Parse an HTTP(S) server URL for future network commands.
    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(ConfigError::EmptyServerUrl);
        }
        if value.chars().any(char::is_whitespace) {
            return Err(ConfigError::InvalidServerUrl);
        }
        if !(value.starts_with("http://") || value.starts_with("https://")) {
            return Err(ConfigError::InvalidServerUrl);
        }
        if value.contains('@') {
            return Err(ConfigError::ServerUrlContainsCredentials);
        }

        Ok(Self(value.to_owned()))
    }

    /// Return the raw URL for future HTTP client wiring.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ServerUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("ServerUrl").field(&REDACTED).finish()
    }
}

/// Environment variable name used as a secret source selector.
#[derive(Clone, Eq, PartialEq)]
pub struct EnvVarName(String);

impl EnvVarName {
    /// Parse a portable environment variable name.
    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(ConfigError::EmptyEnvVarName);
        }

        let mut characters = value.chars();
        let Some(first) = characters.next() else {
            return Err(ConfigError::EmptyEnvVarName);
        };
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return Err(ConfigError::InvalidEnvVarName);
        }
        if !characters.all(|character| character == '_' || character.is_ascii_alphanumeric()) {
            return Err(ConfigError::InvalidEnvVarName);
        }

        Ok(Self(value.to_owned()))
    }

    /// Return the non-secret environment variable name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for EnvVarName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("EnvVarName").field(&self.0).finish()
    }
}

/// Config-file path used as a secret source selector.
#[derive(Clone, Eq, PartialEq)]
pub struct SecretFile(String);

impl SecretFile {
    /// Parse a file path descriptor without printing it later.
    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(ConfigError::EmptySecretFile);
        }
        if value.contains('\0') {
            return Err(ConfigError::InvalidSecretFile);
        }

        Ok(Self(value.to_owned()))
    }

    /// Return the raw path for future config loading code.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("SecretFile").field(&REDACTED).finish()
    }
}

/// OS-secret reference descriptor. This is only a reference, not a keychain call.
#[derive(Clone, Eq, PartialEq)]
pub struct OsSecretRef {
    service: String,
    account: String,
}

impl OsSecretRef {
    /// Parse a non-secret OS-secret service/account reference.
    pub fn parse(service: &str, account: &str) -> Result<Self, ConfigError> {
        let service = service.trim();
        let account = account.trim();
        if service.is_empty() || account.is_empty() {
            return Err(ConfigError::EmptyOsSecretRef);
        }
        if service.contains('\0') || account.contains('\0') {
            return Err(ConfigError::InvalidOsSecretRef);
        }

        Ok(Self {
            service: service.to_owned(),
            account: account.to_owned(),
        })
    }
}

impl fmt::Debug for OsSecretRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OsSecretRef")
            .field("service", &REDACTED)
            .field("account", &REDACTED)
            .finish()
    }
}

/// Safe descriptor for where a token should be loaded from.
#[derive(Clone, Eq, PartialEq)]
pub enum TokenSource {
    /// No token source configured.
    None,
    /// Read token from an environment variable in a future network phase.
    EnvVar(EnvVarName),
    /// Read token from a file in a future network phase.
    File(SecretFile),
    /// Read token from stdin in a future network phase.
    Stdin,
    /// Read token from an OS-secret integration in a future network phase.
    OsSecret(OsSecretRef),
}

impl TokenSource {
    /// Parse a safe token-source descriptor.
    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("none") {
            return Ok(Self::None);
        }
        if value.eq_ignore_ascii_case("stdin") {
            return Ok(Self::Stdin);
        }
        if let Some(name) = value.strip_prefix("env:") {
            return Ok(Self::EnvVar(EnvVarName::parse(name)?));
        }
        if let Some(path) = value.strip_prefix("file:") {
            return Ok(Self::File(SecretFile::parse(path)?));
        }
        if let Some(reference) = value.strip_prefix("os-secret:") {
            let Some((service, account)) = reference.split_once(':') else {
                return Err(ConfigError::InvalidOsSecretRef);
            };
            return Ok(Self::OsSecret(OsSecretRef::parse(service, account)?));
        }
        if value.starts_with("literal:")
            || value.starts_with("bearer:")
            || value.starts_with("token:")
        {
            return Err(ConfigError::InlineTokenValueRejected);
        }

        Err(ConfigError::UnsupportedTokenSource)
    }

    #[must_use]
    fn safe_label(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::EnvVar(_) => "env-var",
            Self::File(_) => "file",
            Self::Stdin => "stdin",
            Self::OsSecret(_) => "os-secret",
        }
    }
}

impl Default for TokenSource {
    fn default() -> Self {
        Self::None
    }
}

impl fmt::Debug for TokenSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenSource")
            .field("source", &self.safe_label())
            .field("value", &REDACTED)
            .finish()
    }
}

/// CLI-local config model for future network-backed commands.
#[derive(Clone, Eq, PartialEq)]
pub struct CliConfig {
    pub profile: ProfileName,
    pub server_url: Option<ServerUrl>,
    pub output_format: OutputFormat,
    pub token_source: TokenSource,
}

impl CliConfig {
    /// Create a CLI config model from validated values.
    #[must_use]
    pub fn new(
        profile: ProfileName,
        server_url: Option<ServerUrl>,
        output_format: OutputFormat,
        token_source: TokenSource,
    ) -> Self {
        Self {
            profile,
            server_url,
            output_format,
            token_source,
        }
    }

    /// Produce a report-safe summary.
    #[must_use]
    pub fn safe_summary(&self) -> SafeConfigSummary<'_> {
        SafeConfigSummary { config: self }
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            profile: ProfileName::default(),
            server_url: None,
            output_format: OutputFormat::default(),
            token_source: TokenSource::default(),
        }
    }
}

impl fmt::Debug for CliConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CliConfig")
            .field("profile", &self.profile)
            .field("server_url", &self.server_url.as_ref().map(|_url| REDACTED))
            .field("output_format", &self.output_format)
            .field("token_source", &self.token_source)
            .finish()
    }
}

/// Display helper for report-safe config summaries.
pub struct SafeConfigSummary<'a> {
    config: &'a CliConfig,
}

impl fmt::Display for SafeConfigSummary<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let server_url = if self.config.server_url.is_some() {
            "configured"
        } else {
            "unset"
        };
        write!(
            formatter,
            "profile={}, server_url={}, output_format={}, token_source={}",
            self.config.profile,
            server_url,
            self.config.output_format,
            self.config.token_source.safe_label()
        )
    }
}

/// Safe configuration validation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigError {
    EmptyProfileName,
    InvalidProfileName,
    EmptyServerUrl,
    InvalidServerUrl,
    ServerUrlContainsCredentials,
    EmptyOutputFormat,
    UnsupportedOutputFormat,
    EmptyEnvVarName,
    InvalidEnvVarName,
    EmptySecretFile,
    InvalidSecretFile,
    EmptyOsSecretRef,
    InvalidOsSecretRef,
    InlineTokenValueRejected,
    UnsupportedTokenSource,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyProfileName => "profile name is empty",
            Self::InvalidProfileName => {
                "invalid profile name: use ASCII letters, digits, dots, underscores, or dashes"
            }
            Self::EmptyServerUrl => "server URL is empty",
            Self::InvalidServerUrl => "invalid server URL: expected http:// or https:// URL",
            Self::ServerUrlContainsCredentials => {
                "invalid server URL: embedded credentials are not allowed"
            }
            Self::EmptyOutputFormat => "output format is empty",
            Self::UnsupportedOutputFormat => "unsupported output format: expected human or json",
            Self::EmptyEnvVarName => "token environment variable name is empty",
            Self::InvalidEnvVarName => {
                "invalid token environment variable name: use portable environment variable syntax"
            }
            Self::EmptySecretFile => "token file path is empty",
            Self::InvalidSecretFile => "invalid token file path",
            Self::EmptyOsSecretRef => "OS secret reference is incomplete",
            Self::InvalidOsSecretRef => "invalid OS secret reference",
            Self::InlineTokenValueRejected => {
                "inline token values are not accepted; use env, file, stdin, or OS secret source"
            }
            Self::UnsupportedTokenSource => {
                "unsupported token source: expected env:<name>, file:<path>, stdin, os-secret:<service>:<account>, or none"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_source_precedence_is_explicit() {
        assert_eq!(
            source_precedence(ConfigField::ServerUrl),
            &[
                ConfigSource::CliFlag,
                ConfigSource::Environment,
                ConfigSource::ConfigFile,
                ConfigSource::BuiltInDefault
            ]
        );
        assert_eq!(
            source_precedence(ConfigField::TokenSource),
            &[
                ConfigSource::CliFlag,
                ConfigSource::Environment,
                ConfigSource::ConfigFile,
                ConfigSource::BuiltInDefault
            ]
        );
    }

    #[test]
    fn token_source_accepts_only_safe_source_descriptors() {
        assert!(matches!(
            TokenSource::parse("env:HAZE_SYNC_TOKEN").unwrap(),
            TokenSource::EnvVar(_)
        ));
        assert!(matches!(
            TokenSource::parse("file:/srv/haze-sync/token.txt").unwrap(),
            TokenSource::File(_)
        ));
        assert_eq!(TokenSource::parse("stdin").unwrap(), TokenSource::Stdin);
        assert!(matches!(
            TokenSource::parse("os-secret:haze-sync:operator").unwrap(),
            TokenSource::OsSecret(_)
        ));
        assert_eq!(TokenSource::parse("none").unwrap(), TokenSource::None);
    }

    #[test]
    fn inline_token_values_are_rejected_without_echo() {
        let inline_value = "literal:redacted-test-value";
        let error = TokenSource::parse(inline_value).unwrap_err();
        let message = error.to_string();

        assert_eq!(error, ConfigError::InlineTokenValueRejected);
        assert!(!message.contains("redacted-test-value"));
        assert!(!message.contains(inline_value));
    }

    #[test]
    fn config_debug_and_summary_redact_sensitive_sources() {
        let config = CliConfig::new(
            ProfileName::parse("dev").unwrap(),
            Some(ServerUrl::parse("https://sync.example.test/private").unwrap()),
            OutputFormat::Json,
            TokenSource::parse("file:/srv/haze-sync/token.txt").unwrap(),
        );

        let debug = format!("{config:?}");
        let summary = config.safe_summary().to_string();

        assert!(debug.contains(REDACTED));
        assert!(!debug.contains("https://sync.example.test"));
        assert!(!debug.contains("/srv/haze-sync"));
        assert!(!summary.contains("https://sync.example.test"));
        assert!(!summary.contains("/srv/haze-sync"));
        assert!(summary.contains("server_url=configured"));
        assert!(summary.contains("token_source=file"));
    }

    #[test]
    fn invalid_config_errors_do_not_echo_raw_values() {
        let unsafe_server_url = concat!("post", "gres", "://", "operator@localhost/db");
        let unsafe_file_path = concat!("C", ":", "\\", "Users", "\\", "operator", "\\", "token.txt");

        let server_error = ServerUrl::parse(unsafe_server_url).unwrap_err().to_string();
        let token_error = TokenSource::parse("bearer:redacted-test-value")
            .unwrap_err()
            .to_string();
        let file_debug = format!(
            "{:?}",
            TokenSource::parse(&format!("file:{unsafe_file_path}")).unwrap()
        );

        assert!(!server_error.contains("operator"));
        assert!(!server_error.contains("localhost"));
        assert!(!server_error.contains("db"));
        assert!(!token_error.contains("redacted-test-value"));
        assert!(!file_debug.contains("Users"));
        assert!(!file_debug.contains("operator"));
    }
}
