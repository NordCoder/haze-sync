//! Explicit adapter identity configuration for authenticated Server routes.

use haze_sync_common::AdapterId;
use std::env;
use std::fmt;

use crate::error::ConfigError;

pub const ENV_ADAPTER_ID: &str = "HAZE_GDRIVE_ADAPTER_ID";

/// Validated adapter identity used only for authenticated route construction.
///
/// Formatting is redacted because the identity must not be emitted together with
/// bearer tokens, private request paths, or provider facts.
#[derive(Clone, PartialEq, Eq)]
pub struct AdapterIdentity(AdapterId);

impl AdapterIdentity {
    pub fn from_raw(raw: &str) -> Result<Self, ConfigError> {
        AdapterId::parse(raw)
            .map(Self)
            .map_err(|_| ConfigError::invalid(ENV_ADAPTER_ID, "adapter id is invalid"))
    }

    pub fn load_from_env() -> Result<Self, ConfigError> {
        Self::load_from_source(&EnvAdapterIdentitySource)
    }

    pub fn load_from_source(source: &impl AdapterIdentitySource) -> Result<Self, ConfigError> {
        let raw = source
            .adapter_id()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ConfigError::missing(ENV_ADAPTER_ID, "adapter id is required"))?;
        Self::from_raw(&raw)
    }

    /// Exposes the validated identity only for the matching authenticated route.
    #[must_use]
    pub fn expose_for_route(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for AdapterIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AdapterIdentity(<redacted>)")
    }
}

impl fmt::Display for AdapterIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted-adapter-identity>")
    }
}

pub trait AdapterIdentitySource {
    fn adapter_id(&self) -> Option<String>;
}

struct EnvAdapterIdentitySource;

impl AdapterIdentitySource for EnvAdapterIdentitySource {
    fn adapter_id(&self) -> Option<String> {
        env::var(ENV_ADAPTER_ID).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ConfigErrorCategory;

    struct MemoryIdentitySource(Option<String>);

    impl AdapterIdentitySource for MemoryIdentitySource {
        fn adapter_id(&self) -> Option<String> {
            self.0.clone()
        }
    }

    #[test]
    fn loads_only_an_accepted_adapter_id() {
        let identity = AdapterIdentity::load_from_source(&MemoryIdentitySource(Some(
            "gdrive-main".to_owned(),
        )))
        .expect("accepted adapter id");
        assert_eq!(identity.expose_for_route(), "gdrive-main");
    }

    #[test]
    fn missing_and_invalid_values_fail_closed_without_echoing_input() {
        let missing = AdapterIdentity::load_from_source(&MemoryIdentitySource(None))
            .expect_err("missing identity must fail");
        assert_eq!(missing.category(), ConfigErrorCategory::Missing);
        assert_eq!(missing.key(), ENV_ADAPTER_ID);

        let sentinel = "invalid adapter identity sentinel";
        let invalid =
            AdapterIdentity::load_from_source(&MemoryIdentitySource(Some(sentinel.to_owned())))
                .expect_err("invalid identity must fail");
        assert_eq!(invalid.category(), ConfigErrorCategory::Invalid);
        assert!(!invalid.to_string().contains(sentinel));
    }

    #[test]
    fn identity_formatting_is_always_redacted() {
        let sentinel = "gdrive-sentinel";
        let identity = AdapterIdentity::from_raw(sentinel).expect("identity");
        assert!(!format!("{identity:?}").contains(sentinel));
        assert!(!identity.to_string().contains(sentinel));
    }
}
