//! Environment and safety helpers for real PostgreSQL storage tests.

use super::TestSupportError;
use std::{env, fmt};

/// Required environment variable for real PostgreSQL storage tests.
pub const HAZE_SYNC_TEST_DATABASE_URL_ENV: &str = "HAZE_SYNC_TEST_DATABASE_URL";

/// General application database variable that test support deliberately ignores.
///
/// The constant remains available for callers that need to explain the policy,
/// but storage test helpers never read this variable implicitly.
pub const DATABASE_URL_ENV: &str = "DATABASE_URL";

const SAFE_QUERY_PARAMETERS: &[&str] = &[
    "application_name",
    "connect_timeout",
    "sslcert",
    "sslkey",
    "sslmode",
    "sslrootcert",
];

/// Source label for an explicitly supplied test database URL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestDatabaseUrlSource {
    /// URL came from `HAZE_SYNC_TEST_DATABASE_URL`.
    HazeSyncTestDatabaseUrl,
    /// URL was supplied explicitly by a caller that labels it as `DATABASE_URL`.
    ///
    /// Storage test helpers do not read `DATABASE_URL` automatically.
    DatabaseUrl,
}

impl TestDatabaseUrlSource {
    /// Returns the source label.
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
/// Formatting this value always redacts the original URL and database name. Use
/// `as_sensitive_str` only when passing the URL to a database client.
#[derive(Clone, Eq, PartialEq)]
pub struct TestDatabaseUrl {
    raw_url: String,
    source: TestDatabaseUrlSource,
    database_name: String,
}

impl TestDatabaseUrl {
    /// Discovers an optional URL from `HAZE_SYNC_TEST_DATABASE_URL` only.
    ///
    /// This helper is intended for configuration inspection. Executable
    /// integration tests should use [`Self::require_from_env`] so missing
    /// configuration fails explicitly rather than being reported as a pass.
    pub fn from_env() -> Result<Option<Self>, TestSupportError> {
        parse_optional_config_value(
            TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
            read_test_database_url_env()?,
        )
    }

    /// Requires an explicit `HAZE_SYNC_TEST_DATABASE_URL` value.
    ///
    /// `DATABASE_URL` is intentionally ignored to prevent destructive test
    /// helpers from accidentally targeting an application or production database.
    pub fn require_from_env() -> Result<Self, TestSupportError> {
        require_config_value(
            TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
            read_test_database_url_env()?,
        )
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

    /// Returns the source label.
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
        format!("{scheme}://<redacted>/<test-database>")
    }
}

impl fmt::Debug for TestDatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TestDatabaseUrl")
            .field("source", &self.source.name())
            .field("url", &self.redacted())
            .finish()
    }
}

impl fmt::Display for TestDatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.redacted())
    }
}

fn read_test_database_url_env() -> Result<Option<String>, TestSupportError> {
    match env::var(HAZE_SYNC_TEST_DATABASE_URL_ENV) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(TestSupportError::EnvironmentVariableNotUnicode {
            name: HAZE_SYNC_TEST_DATABASE_URL_ENV,
        }),
    }
}

fn parse_optional_config_value(
    source: TestDatabaseUrlSource,
    value: Option<String>,
) -> Result<Option<TestDatabaseUrl>, TestSupportError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }

    TestDatabaseUrl::parse_explicit(source, value).map(Some)
}

fn require_config_value(
    source: TestDatabaseUrlSource,
    value: Option<String>,
) -> Result<TestDatabaseUrl, TestSupportError> {
    let value = value.ok_or(TestSupportError::MissingTestDatabaseUrl)?;
    if value.trim().is_empty() {
        return Err(TestSupportError::MissingTestDatabaseUrl);
    }

    TestDatabaseUrl::parse_explicit(source, value)
}

fn extract_safe_database_name(raw_url: &str) -> Result<String, TestSupportError> {
    if raw_url.trim().len() != raw_url.len() || raw_url.chars().any(char::is_control) {
        return Err(TestSupportError::InvalidDatabaseUrl);
    }

    let (scheme, rest) = raw_url
        .split_once("://")
        .ok_or(TestSupportError::InvalidDatabaseUrl)?;
    if scheme != "postgres" && scheme != "postgresql" {
        return Err(TestSupportError::UnsupportedDatabaseUrlScheme);
    }
    if rest.contains('#') {
        return Err(TestSupportError::InvalidDatabaseUrl);
    }

    let (without_query, query) = rest
        .split_once('?')
        .map_or((rest, None), |(path, query)| (path, Some(query)));
    if let Some(query) = query {
        validate_safe_query(query)?;
    }

    let (_, database_name) = without_query
        .split_once('/')
        .ok_or(TestSupportError::MissingDatabaseName)?;

    if database_name.is_empty() {
        return Err(TestSupportError::MissingDatabaseName);
    }
    if database_name.contains('/')
        || database_name.contains('%')
        || !database_name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
    {
        return Err(TestSupportError::InvalidDatabaseUrl);
    }

    validate_test_database_name(database_name)?;
    Ok(database_name.to_owned())
}

fn validate_safe_query(query: &str) -> Result<(), TestSupportError> {
    if query.is_empty() {
        return Err(TestSupportError::InvalidDatabaseUrl);
    }

    let mut seen_keys = Vec::new();
    for parameter in query.split('&') {
        let (key, value) = parameter
            .split_once('=')
            .ok_or(TestSupportError::InvalidDatabaseUrl)?;
        if key.is_empty()
            || value.is_empty()
            || key.contains('%')
            || !SAFE_QUERY_PARAMETERS.contains(&key)
            || seen_keys.contains(&key)
        {
            return Err(TestSupportError::InvalidDatabaseUrl);
        }
        seen_keys.push(key);
    }

    Ok(())
}

fn validate_test_database_name(database_name: &str) -> Result<(), TestSupportError> {
    let lower = database_name.to_ascii_lowercase();
    let tokens: Vec<_> = lower
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();
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
    let blocked_token = tokens.iter().any(|token| {
        matches!(
            *token,
            "prod" | "production" | "live" | "main" | "primary" | "default"
        )
    });
    let has_test_marker = tokens.iter().any(|token| {
        *token == "test"
            || token.strip_prefix("test").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
            })
    });

    if blocked_exact.contains(&lower.as_str()) || blocked_token || !has_test_marker {
        return Err(TestSupportError::UnsafeDatabaseName);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_fully_redacts_test_database_url() {
        let raw = "postgres://haze_sync:placeholder_password@localhost:5432/haze_sync_test?sslmode=disable";
        let url =
            TestDatabaseUrl::parse_explicit(TestDatabaseUrlSource::HazeSyncTestDatabaseUrl, raw)
                .expect("test URL should be accepted");

        assert_eq!(url.as_sensitive_str(), raw);
        assert_eq!(url.database_name(), "haze_sync_test");
        assert_eq!(url.source(), TestDatabaseUrlSource::HazeSyncTestDatabaseUrl);
        assert!(!format!("{url:?}").contains("placeholder_password"));
        assert!(!url.to_string().contains("placeholder_password"));
        assert!(!url.to_string().contains("haze_sync_test"));
        assert_eq!(url.to_string(), "postgres://<redacted>/<test-database>");
    }

    #[test]
    fn required_configuration_rejects_missing_or_blank_values() {
        assert_eq!(
            require_config_value(TestDatabaseUrlSource::HazeSyncTestDatabaseUrl, None),
            Err(TestSupportError::MissingTestDatabaseUrl)
        );
        assert_eq!(
            require_config_value(
                TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
                Some("   ".to_owned())
            ),
            Err(TestSupportError::MissingTestDatabaseUrl)
        );
        assert_eq!(
            parse_optional_config_value(TestDatabaseUrlSource::HazeSyncTestDatabaseUrl, None),
            Ok(None)
        );
    }

    #[test]
    fn rejects_non_postgres_or_ambiguous_urls() {
        assert_eq!(
            TestDatabaseUrl::parse_explicit(
                TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
                "mysql://localhost/haze_sync_test"
            ),
            Err(TestSupportError::UnsupportedDatabaseUrlScheme)
        );
        assert_eq!(
            TestDatabaseUrl::parse_explicit(
                TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
                "postgres://localhost/haze_sync_test/extra"
            ),
            Err(TestSupportError::InvalidDatabaseUrl)
        );
        assert_eq!(
            TestDatabaseUrl::parse_explicit(
                TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
                "postgres://localhost/haze%5fsync%5ftest"
            ),
            Err(TestSupportError::InvalidDatabaseUrl)
        );
    }

    #[test]
    fn rejects_query_parameters_that_can_change_target_or_session() {
        for query in [
            "dbname=production_test",
            "options=-csearch_path%3Dpublic",
            "sslmode=disable&sslmode=require",
            "sslmode=",
            "",
        ] {
            let result = TestDatabaseUrl::parse_explicit(
                TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
                format!("postgres://localhost/haze_sync_test?{query}"),
            );

            assert_eq!(result, Err(TestSupportError::InvalidDatabaseUrl));
        }
    }

    #[test]
    fn accepts_safe_tls_and_connection_query_parameters() {
        let url = TestDatabaseUrl::parse_explicit(
            TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
            "postgres://localhost/haze_sync_test?sslmode=require&connect_timeout=5",
        )
        .unwrap();

        assert_eq!(url.database_name(), "haze_sync_test");
    }

    #[test]
    fn rejects_database_without_standalone_test_marker() {
        for database_name in ["haze_sync", "contest", "latest", "attestation"] {
            let result = TestDatabaseUrl::parse_explicit(
                TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
                format!("postgres://localhost/{database_name}"),
            );

            assert_eq!(result, Err(TestSupportError::UnsafeDatabaseName));
        }
    }

    #[test]
    fn rejects_obviously_production_database_names_even_with_test_marker() {
        for database_name in ["prod_test", "production_test", "live_test", "test_primary"] {
            let result = TestDatabaseUrl::parse_explicit(
                TestDatabaseUrlSource::DatabaseUrl,
                format!("postgresql://localhost/{database_name}"),
            );

            assert_eq!(result, Err(TestSupportError::UnsafeDatabaseName));
        }
    }

    #[test]
    fn accepts_numbered_test_database_names() {
        let url = TestDatabaseUrl::parse_explicit(
            TestDatabaseUrlSource::HazeSyncTestDatabaseUrl,
            "postgres://localhost/haze_sync_test42",
        )
        .unwrap();

        assert_eq!(url.database_name(), "haze_sync_test42");
    }
}
