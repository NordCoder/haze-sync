//! Safe readiness checks for server runtime dependencies.
//!
//! Readiness reports are deterministic JSON payloads that avoid raw database
//! URLs, filesystem paths, provider payloads, stack traces, and secrets.

use crate::{
    config::{ObjectStoreConfig, ServerConfig},
    db::ping_pg_pool,
};
use serde::Serialize;
use sqlx::PgPool;
use std::{fmt, fs, io, path::Path, path::PathBuf};

/// Aggregate status for GET /ready.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessOverallStatus {
    /// Every required component is ready.
    Ready,
    /// At least one required component is disabled or unhealthy.
    NotReady,
}

/// Public component status for GET /ready.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessComponentStatus {
    /// The component was configured and passed its check.
    Ready,
    /// The component was configured but failed its check.
    NotReady,
    /// The component is not configured for runtime-backed readiness.
    Disabled,
}

impl ReadinessComponentStatus {
    const fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Safe public component readiness payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReadinessComponent {
    /// Stable component name.
    pub name: &'static str,
    /// Stable component status.
    pub status: ReadinessComponentStatus,
    /// Stable machine-readable component code.
    pub code: &'static str,
    /// Safe path-free and secret-free component message.
    pub message: &'static str,
}

impl ReadinessComponent {
    const fn ready(name: &'static str, code: &'static str, message: &'static str) -> Self {
        Self {
            name,
            status: ReadinessComponentStatus::Ready,
            code,
            message,
        }
    }

    const fn not_ready(name: &'static str, code: &'static str, message: &'static str) -> Self {
        Self {
            name,
            status: ReadinessComponentStatus::NotReady,
            code,
            message,
        }
    }

    const fn disabled(name: &'static str, code: &'static str, message: &'static str) -> Self {
        Self {
            name,
            status: ReadinessComponentStatus::Disabled,
            code,
            message,
        }
    }
}

/// Safe JSON response returned by GET /ready.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReadinessReport {
    /// Overall status derived from required component statuses.
    pub status: ReadinessOverallStatus,
    /// Deterministically ordered component checks.
    pub components: Vec<ReadinessComponent>,
}

impl ReadinessReport {
    /// Build a readiness report from deterministic component results.
    #[must_use]
    pub fn new(components: Vec<ReadinessComponent>) -> Self {
        let status = if components.iter().all(|component| component.status.is_ready()) {
            ReadinessOverallStatus::Ready
        } else {
            ReadinessOverallStatus::NotReady
        };

        Self { status, components }
    }

    /// Return true when all required components are ready.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self.status, ReadinessOverallStatus::Ready)
    }
}

/// Runtime readiness dependencies owned by the server.
#[derive(Clone)]
pub struct ReadinessState {
    database: DatabaseReadiness,
    object_store: ObjectStoreReadiness,
}

impl ReadinessState {
    /// Safe dependency-free state used by the route shell and tests.
    #[must_use]
    pub const fn dependency_free_not_ready() -> Self {
        Self {
            database: DatabaseReadiness::Disabled,
            object_store: ObjectStoreReadiness::Disabled,
        }
    }

    /// Build readiness state from optional runtime dependencies.
    #[must_use]
    pub fn from_optional(
        database_pool: Option<PgPool>,
        object_store: Option<ObjectStoreConfig>,
    ) -> Self {
        Self {
            database: database_pool.map_or(DatabaseReadiness::Disabled, DatabaseReadiness::Pool),
            object_store: object_store.map_or(ObjectStoreReadiness::Disabled, |config| {
                ObjectStoreReadiness::Configured { root: config.root }
            }),
        }
    }

    /// Build readiness state from server config and a caller-owned PostgreSQL pool.
    #[must_use]
    pub fn from_config(config: &ServerConfig, database_pool: PgPool) -> Self {
        Self {
            database: DatabaseReadiness::Pool(database_pool),
            object_store: ObjectStoreReadiness::Configured {
                root: config.object_store.root.clone(),
            },
        }
    }

    /// Check all runtime readiness components.
    pub async fn check(&self) -> ReadinessReport {
        ReadinessReport::new(vec![
            self.database.check().await,
            self.object_store.check(),
        ])
    }
}

impl fmt::Debug for ReadinessState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReadinessState")
            .field("database", &self.database)
            .field("object_store", &self.object_store)
            .finish()
    }
}

/// Database dependency used by readiness checks.
#[derive(Clone)]
pub enum DatabaseReadiness {
    /// No pool was supplied to readiness.
    Disabled,
    /// Caller-owned PostgreSQL pool to ping.
    Pool(PgPool),
}

impl DatabaseReadiness {
    async fn check(&self) -> ReadinessComponent {
        match self {
            Self::Disabled => ReadinessComponent::disabled(
                "database",
                "database_not_configured",
                "database readiness is not configured",
            ),
            Self::Pool(pool) => match ping_pg_pool(pool).await {
                Ok(()) => ReadinessComponent::ready(
                    "database",
                    "database_ready",
                    "database connectivity check passed",
                ),
                Err(_error) => ReadinessComponent::not_ready(
                    "database",
                    "database_unavailable",
                    "database connectivity check failed",
                ),
            },
        }
    }
}

impl fmt::Debug for DatabaseReadiness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => formatter.write_str("DatabaseReadiness::Disabled"),
            Self::Pool(_pool) => formatter.write_str("DatabaseReadiness::Pool([REDACTED])"),
        }
    }
}

/// Object-store dependency used by readiness checks.
#[derive(Clone, Eq, PartialEq)]
pub enum ObjectStoreReadiness {
    /// No object-store root was supplied to readiness.
    Disabled,
    /// Configured object-store root to inspect.
    Configured { root: PathBuf },
}

impl ObjectStoreReadiness {
    fn check(&self) -> ReadinessComponent {
        match self {
            Self::Disabled => ReadinessComponent::disabled(
                "object_store",
                "object_store_not_configured",
                "object store readiness is not configured",
            ),
            Self::Configured { root } => check_object_store_root(root),
        }
    }
}

impl fmt::Debug for ObjectStoreReadiness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => formatter.write_str("ObjectStoreReadiness::Disabled"),
            Self::Configured { .. } => {
                formatter.write_str("ObjectStoreReadiness::Configured([REDACTED])")
            }
        }
    }
}

fn check_object_store_root(root: &Path) -> ReadinessComponent {
    if root.as_os_str().is_empty() {
        return ReadinessComponent::not_ready(
            "object_store",
            "object_store_root_missing",
            "object store root is not configured",
        );
    }

    match fs::metadata(root) {
        Ok(metadata) if metadata.is_dir() => ReadinessComponent::ready(
            "object_store",
            "object_store_ready",
            "object store root is available",
        ),
        Ok(_metadata) => ReadinessComponent::not_ready(
            "object_store",
            "object_store_root_not_directory",
            "object store root is not a directory",
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => ReadinessComponent::not_ready(
            "object_store",
            "object_store_root_unavailable",
            "object store root is unavailable",
        ),
        Err(_error) => ReadinessComponent::not_ready(
            "object_store",
            "object_store_root_inspection_failed",
            "object store root could not be inspected",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after Unix epoch")
                .as_nanos();
            let path = env::temp_dir().join(format!(
                "haze-sync-readiness-{name}-{}-{timestamp}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test directory should be created");

            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[tokio::test]
    async fn dependency_free_state_is_safe_not_ready() {
        let report = ReadinessState::dependency_free_not_ready().check().await;
        let json = serde_json::to_value(&report).expect("readiness should serialize");

        assert_eq!(json["status"], "not_ready");
        assert_eq!(json["components"][0]["name"], "database");
        assert_eq!(json["components"][0]["status"], "disabled");
        assert_eq!(json["components"][1]["name"], "object_store");
        assert_eq!(json["components"][1]["status"], "disabled");
        assert!(!json.to_string().contains("postgres://"));
        assert!(!json.to_string().contains("secret"));
        assert!(!json.to_string().contains("/srv/"));
    }

    #[tokio::test]
    async fn object_store_ready_still_requires_database() {
        let dir = TestDir::new("object-ready");
        let report = ReadinessState::from_optional(
            None,
            Some(ObjectStoreConfig {
                root: dir.path.clone(),
            }),
        )
        .check()
        .await;

        assert_eq!(report.status, ReadinessOverallStatus::NotReady);
        assert_eq!(report.components[0].status, ReadinessComponentStatus::Disabled);
        assert_eq!(report.components[1].status, ReadinessComponentStatus::Ready);
    }

    #[tokio::test]
    async fn missing_object_store_root_is_redacted() {
        let sensitive_root = PathBuf::from("/srv/haze-sync/objects/super-secret-root");
        let report = ReadinessState::from_optional(
            None,
            Some(ObjectStoreConfig {
                root: sensitive_root,
            }),
        )
        .check()
        .await;
        let json = serde_json::to_string(&report).expect("readiness should serialize");

        assert_eq!(report.status, ReadinessOverallStatus::NotReady);
        assert_eq!(report.components[1].code, "object_store_root_unavailable");
        assert!(!json.contains("/srv/haze-sync"));
        assert!(!json.contains("super-secret-root"));
        assert!(!json.contains("stack"));
    }

    #[tokio::test]
    async fn file_instead_of_object_store_directory_is_safe() {
        let dir = TestDir::new("object-file");
        let file_path = dir.path.join("not-a-directory");
        fs::write(&file_path, b"not a directory").expect("test file should be written");

        let report = ReadinessState::from_optional(
            None,
            Some(ObjectStoreConfig { root: file_path }),
        )
        .check()
        .await;
        let json = serde_json::to_string(&report).expect("readiness should serialize");
        let dir_path = dir.path.to_string_lossy();

        assert_eq!(report.components[1].code, "object_store_root_not_directory");
        assert!(!json.contains("not-a-directory"));
        assert!(!json.contains(dir_path.as_ref()));
    }

    #[test]
    fn debug_output_is_redacted() {
        let state = ReadinessState::from_optional(
            None,
            Some(ObjectStoreConfig {
                root: PathBuf::from("/srv/haze-sync/objects/super-secret-root"),
            }),
        );
        let rendered = format!("{state:?}");

        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains("/srv/haze-sync"));
        assert!(!rendered.contains("super-secret-root"));
    }
}
