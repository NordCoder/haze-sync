//! Safe public errors for config parsing.

use std::{error::Error, fmt};

/// Configuration parsing error that never includes raw env values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// A required environment variable is absent or empty.
    MissingRequiredVar { name: &'static str },
    /// An environment variable was present but malformed.
    InvalidVar {
        name: &'static str,
        reason: &'static str,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredVar { name } => {
                write!(formatter, "missing required config variable {name}")
            }
            Self::InvalidVar { name, reason } => {
                write!(formatter, "invalid config variable {name}: {reason}")
            }
        }
    }
}

impl Error for ConfigError {}
