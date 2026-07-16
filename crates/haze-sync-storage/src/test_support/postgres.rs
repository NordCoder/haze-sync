//! Real PostgreSQL helpers for storage integration tests.
//!
//! These helpers are compiled for storage crate tests and downstream harnesses
//! that explicitly enable `test-support`. They require a validated dedicated test
//! database URL and never create, drop, or select a database implicitly.

use super::{TestDatabaseUrl, TestNamespace, TestSupportError};
use crate::schema::table_names;
#[cfg(test)]
use crate::schema::INITIAL_MIGRATIONS;
use sqlx::{postgres::PgPoolOptions, Executor, PgPool, Postgres, Transaction};
use std::fmt;

include!("postgres/implementation.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{connect_required_test_database_from_env, unique_test_id};

    include!("postgres/tests.rs");
}
