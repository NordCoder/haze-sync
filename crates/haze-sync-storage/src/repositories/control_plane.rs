//! Passive PostgreSQL repositories for Stage 11 operational-control state.
//!
//! Every mutating function requires a caller-owned PostgreSQL transaction. The
//! helpers implement only row locking, exact compare-and-set, uniqueness,
//! immutable evidence insertion, and fact reads. They never choose a legal
//! maintenance transition, authorize a principal, start a runtime or job,
//! allocate an external side effect, or render a public DTO.

use crate::schema::control_plane::cas_namespaces;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{postgres::PgRow, Postgres, Row, Transaction};
use std::{error::Error, fmt};

include!("control_plane/types.rs");
include!("control_plane/qa_corrections_types.rs");
include!("control_plane/repository.rs");

#[cfg(test)]
#[path = "control_plane/tests.rs"]
mod tests;

#[cfg(all(test, feature = "test-support"))]
#[path = "control_plane/postgres_contract_tests.rs"]
mod postgres_contract_tests;
