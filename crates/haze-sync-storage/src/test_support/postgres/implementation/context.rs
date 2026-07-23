pub struct PostgresTestContext {
    url: TestDatabaseUrl,
    pool: PgPool,
    namespace: TestNamespace,
}

impl PostgresTestContext {
    /// Optional compatibility path for tests explicitly named as optional.
    pub async fn connect_from_env() -> PostgresTestResult<Option<Self>> {
        let Some(url) = TestDatabaseUrl::from_env()? else {
            return Ok(None);
        };
        Self::connect(url).await.map(Some)
    }

    pub async fn connect_required_from_env() -> PostgresTestResult<Self> {
        Self::connect(TestDatabaseUrl::require_from_env()?).await
    }

    pub async fn prepare_from_env() -> PostgresTestResult<Self> {
        let context = Self::connect_required_from_env().await?;
        context.apply_migrations().await?;
        Ok(context)
    }

    pub async fn connect(url: TestDatabaseUrl) -> PostgresTestResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url.as_sensitive_str())
            .await
            .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
        Ok(Self {
            url,
            pool,
            namespace: TestNamespace::new("storage-pg"),
        })
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[must_use]
    pub const fn url(&self) -> &TestDatabaseUrl {
        &self.url
    }

    #[must_use]
    pub const fn namespace(&self) -> &TestNamespace {
        &self.namespace
    }

    pub async fn apply_migrations(&self) -> PostgresTestResult<()> {
        apply_storage_migrations(&self.pool).await
    }

    pub async fn clean_storage_tables(&self) -> PostgresTestResult<()> {
        clean_storage_tables(&self.pool).await
    }
}

impl fmt::Debug for PostgresTestContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostgresTestContext")
            .field("url", &self.url)
            .field("namespace", &self.namespace)
            .finish_non_exhaustive()
    }
}

pub async fn connect_required_test_database_from_env() -> PostgresTestResult<PostgresTestContext> {
    PostgresTestContext::connect_required_from_env().await
}

pub async fn connect_test_database_from_env() -> PostgresTestResult<Option<PostgresTestContext>> {
    PostgresTestContext::connect_from_env().await
}

pub async fn prepare_test_database_from_env() -> PostgresTestResult<PostgresTestContext> {
    PostgresTestContext::prepare_from_env().await
}
