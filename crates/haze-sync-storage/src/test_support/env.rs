//! Environment and safety helpers for real PostgreSQL storage tests.

use super::TestSupportError;
use std::{env, fmt};

/// Preferred environment variable for real PostgreSQL storage tests.
pub const HAZE_SYNC_TEST_DATABASE_URL_ENV: &str = "HAZE_SYNC_TEST_DATABASE_URL";

/// Fallback environment variable accepted only after the same safety checks.
pub const DATABASE_URL_ENV: &str = "DATABASE_URL";

/// Source environment variable for a test database URL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestDatabaseUrlSource {
    /// URL came from `HAZE_SYNC_TEST_DATABASE_URL`.
    HazeSyncTestDatabaseUrl,
    /// URL came from `DATABASE_URL`.
    DatabaseUrl,
}

impl TestDatabaseUrlSource {
    /// Returns the environment variable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::HazeSyncTestDatabaseUrl => HAZE_SYNC_TEST_DATABASE_URL_ENV,
            Self::DatabaseUrl => DATABASE_URL_ENV,
        }
    }
}

/// A validated PostgreSQL URL for test-only database helpers.
///
/// Formatting this value always redacts the original URL. Use
/// `as_sensitive_str` only when passing the URL to a database client.
#[derive(Clone, Eq, PartialEq)]
pub struct TestDatabaseUrl {
    raw_url: String,
    source: TestDatabaseUrlSource,
    database_name: String,
}

impl TestDatabaseUrl {
    /// Reads `HAZE_SYNC_TEST_DATABASE_URL`, falling back to `DATABASE_URL` when
    /// the preferred variable is unset or empty.
    pub fn from_env() -> Result<Option<Self>, TestSupportError> {
        for source in [
            TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
            TestDatabaseUrlSource::DatabaseUrl,
        ] {
            match env::var(source.name()) {
                Ok(value) if value.trim().is_empty() => continue,
                Ok(value) => return Self::parse_explicit(source, value).map(Some),
                Err(env::VarError::NotPresent) => continue,
                Err(env::VarError::NotUnicode(_)) => {
                    return Err(TestSupportError::EnvironmentVariableNotUnicode {
                        name: source.name(),
                    });
                }
            }
        }

        Ok(None)
    }

    /// Parses and validates an explicitly supplied test database URL.
    pub fn parse_explicit(
        source: TestDatabaseUrlSource,
        raw_url: impl Into<String>,
    ) -> Result<Self, TestSupportError> {
        let raw_url = raw_url.into();
        let database_name = extract_safe_database_name(&raw_url)?;

        Ok(Self {
            raw_url,
            source,
            database_name,
        })
    }

    /// Returns the sensitive raw URL for database client construction only.
    #[must_use]
    pub fn as_sensitive_str(&self) -> &str {
        &self.raw_url
    }

    /// Returns the source environment variable.
    #[must_use]
    pub const fn source(&self) -> TestDatabaseUrlSource {
        self.source
    }

    /// Returns the validated database name.
    #[must_use]
    pub fn database_name(&self) -> &str {
        &self.database_name
    }

    /// Returns a redacted string safe for diagnostics.
    #[must_use]
    pub fn redacted(&self) -> String {
        let scheme = self
            .raw_url
            .split_once("://")
            .map_or("postgres", |(scheme, _)| scheme);
        format!("{scheme}://<redacted>/{}", self.database_name)
    }
}

impl fmt::Debug for TestDatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TestDatabaseUrl")
            .field("source", &self.source.name())
            .field("url", &self.redacted())
            .field("database_name", &self.database_name)
            .finish()
    }
}

impl fmt::Display for TestDatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.redacted())
    }
}

fn extract_safe_database_name(raw_url: &str) -> Result<String, TestSupportError> {
    let (scheme, rest) = raw_url
        .split_once("://")
        .ok_or(TestSupportError::InvalidDatabaseUrl)?;
    if scheme != "postgres" && scheme != "postgresql" {
        return Err(TestSupportError::UnsupportedDatabaseUrlScheme);
    }

    let without_fragment = rest.split('#').next().unwrap_or(rest);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    let path_start = without_query
        .find('/')
        .ok_or(TestSupportError::MissingDatabaseName)?;
    let database_name = without_query[path_start + 1..]
        .split('/')
        .next()
        .unwrap_or_default()
        .trim();

    if database_name.is_empty() {
        return Err(TestSupportError::MissingDatabaseName);
    }

    validate_test_database_name(database_name)?;
    Ok(database_name.to_owned())
}

fn validate_test_database_name(database_name: &str) -> Result<(), TestSupportError> {
    let lower = database_name.to_ascii_lowercase();
    let blocked_exact = [
        "postgres",
        "template0",
        "template1",
        "haze_sync",
        "haze-sync",
        "production",
        "prod",
        "main",
        "default",
        "primary",
        "live",
    ];

    let unsafe_name = blocked_exact.contains(&lower.as_str())
        || lower.contains("prod")
        || lower.contains("production")
        || lower.contains("live")
        || !lower.contains("test");

    if unsafe_name {
        return Err(TestSupportError::UnsafeDatabaseName {
            database_name: database_name.to_owned(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_redacts_test_database_url() {
        let raw = "postgres://haze_sync:super_secret@localhost:5432/haze_sync_test?sslmode=disable";
        let url = TestDatabaseUrl::parse_explicit(
            TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
            raw,
        )
        .expect("test URL should be accepted");

        assert_eq!(url.as_sensitive_str(), raw);
        assert_eq!(url.database_name(), "haze_sync_test");
        assert_eq!(url.source(), TestDatabaseUrlSource::HazeSyncTestDatabaseUrl);
        assert!(!format!("{url:?}").contains("super_secret"));
        assert!(!url.to_string().contains("super_secret"));
        assert!(url.to_string().contains("<redacted>"));
    }

    #[test]
    fn rejects_non_postgres_urls() {
        let result = TestDatabaseUrl::parse_explicit(
            TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
            "mysql://localhost/haze_sync_test",
        );

        assert_eq!(result, Err(TestSupportError::UnsupportedDatabaseUrlScheme));
    }

    #[test]
    fn rejects_database_without_test_marker() {
        let result = TestDatabaseUrl::parse_explicit(
            TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
            "postgres://localhost/haze_sync",
        );

        assert_eq!(
            result,
            Err(TestSupportError::UnsafeDatabaseName {
                database_name: "haze_sync".to_owned()
            })
        );
    }

    #[test]
    fn rejects_obviously_production_database_names() {
        let result = TestDatabaseUrl::parse_explicit(
            TestDatabaseUrlSource::DatabaseUrl,
            "postgresql://localhost/prod_test",
        );

        assert_eq!(
            result,
            Err(TestSupportError::UnsafeDatabaseName {
                database_name: "prod_test".to_owned()
            })
        );
    }
}
