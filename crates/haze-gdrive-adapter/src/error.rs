//! Safe error types for the Google Drive adapter runtime.
//!
//! Error display text is intended for operator-facing output. It must not include
//! token values, secret file paths, raw provider payloads, or local absolute paths.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigErrorCategory {
    Missing,
    Invalid,
    AmbiguousSecretSource,
    SecretFileReadFailed,
}

impl fmt::Display for ConfigErrorCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Missing => "missing",
            Self::Invalid => "invalid",
            Self::AmbiguousSecretSource => "ambiguous_secret_source",
            Self::SecretFileReadFailed => "secret_file_read_failed",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    key: &'static str,
    category: ConfigErrorCategory,
    detail: &'static str,
}

impl ConfigError {
    pub const fn new(
        key: &'static str,
        category: ConfigErrorCategory,
        detail: &'static str,
    ) -> Self {
        Self {
            key,
            category,
            detail,
        }
    }

    pub const fn missing(key: &'static str, detail: &'static str) -> Self {
        Self::new(key, ConfigErrorCategory::Missing, detail)
    }

    pub const fn invalid(key: &'static str, detail: &'static str) -> Self {
        Self::new(key, ConfigErrorCategory::Invalid, detail)
    }

    pub const fn ambiguous_secret_source(key: &'static str) -> Self {
        Self::new(
            key,
            ConfigErrorCategory::AmbiguousSecretSource,
            "configure exactly one secret source",
        )
    }

    pub const fn secret_file_read_failed(key: &'static str) -> Self {
        Self::new(
            key,
            ConfigErrorCategory::SecretFileReadFailed,
            "secret file could not be read",
        )
    }

    pub const fn key(&self) -> &'static str {
        self.key
    }

    pub const fn category(&self) -> ConfigErrorCategory {
        self.category
    }

    pub const fn detail(&self) -> &'static str {
        self.detail
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "configuration key {} is {}: {}",
            self.key, self.category, self.detail
        )
    }
}

impl Error for ConfigError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeErrorCategory {
    Config,
}

impl fmt::Display for RuntimeErrorCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Config => "config",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Config(ConfigError),
}

impl RuntimeError {
    pub const fn category(&self) -> RuntimeErrorCategory {
        match self {
            Self::Config(_) => RuntimeErrorCategory::Config,
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "{} error: {}", self.category(), error),
        }
    }
}

impl Error for RuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
        }
    }
}

impl From<ConfigError> for RuntimeError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}
