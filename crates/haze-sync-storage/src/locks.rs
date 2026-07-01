//! PostgreSQL transaction-scoped advisory locks for normalized vault paths.
//!
//! These helpers derive a deterministic lock key from a validated `VaultPath` and
//! execute `pg_advisory_xact_lock`. They do not create process-global locks or
//! provide in-memory correctness guarantees.

use crate::repositories::{map_sqlx_error, RepositoryResult};
use haze_sync_common::VaultPath;
use sha2::{Digest, Sha256 as Sha256Hasher};
use sqlx::{Executor, Postgres};

/// Deterministic PostgreSQL advisory-lock key for one normalized vault path.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PathLockKey(i64);

impl PathLockKey {
    /// Returns the signed 64-bit key used with PostgreSQL advisory locks.
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self.0
    }
}

/// Derives a deterministic lock key from a normalized vault path.
///
/// The key is the first 64 bits of SHA-256 over the normalized UTF-8 path,
/// interpreted as a big-endian signed integer. The hash is deterministic across
/// Rust and PostgreSQL versions and avoids relying on process-local hash state.
#[must_use]
pub fn path_lock_key(path: &VaultPath) -> PathLockKey {
    let digest = Sha256Hasher::digest(path.as_str().as_bytes());
    let mut key_bytes = [0_u8; 8];
    key_bytes.copy_from_slice(&digest[..8]);
    PathLockKey(i64::from_be_bytes(key_bytes))
}

/// Acquires a transaction-scoped PostgreSQL advisory lock for `path`.
///
/// Callers must pass an executor tied to the transaction that owns the critical
/// section. PostgreSQL releases `pg_advisory_xact_lock` automatically at
/// transaction end.
pub async fn lock_vault_path<'executor, ExecutorType>(
    executor: ExecutorType,
    path: &VaultPath,
) -> RepositoryResult<PathLockKey>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let key = path_lock_key(path);
    sqlx::query("select pg_advisory_xact_lock($1)")
        .bind(key.as_i64())
        .execute(executor)
        .await
        .map_err(map_sqlx_error)?;

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_key_is_deterministic_for_normalized_path() {
        let path = VaultPath::parse("Notes/a.md").unwrap();
        assert_eq!(path_lock_key(&path).as_i64(), 5_144_238_026_702_106_084);
        assert_eq!(path_lock_key(&path), path_lock_key(&path));
    }

    #[test]
    fn lock_key_uses_normalized_vault_path() {
        let canonical = VaultPath::parse("Notes/a.md").unwrap();
        let normalized = VaultPath::parse("./Notes//./a.md").unwrap();
        assert_eq!(canonical, normalized);
        assert_eq!(path_lock_key(&canonical), path_lock_key(&normalized));
    }

    #[test]
    fn different_paths_get_different_keys_for_common_case() {
        let first = VaultPath::parse("Notes/a.md").unwrap();
        let second = VaultPath::parse("Notes/b.md").unwrap();
        assert_ne!(path_lock_key(&first), path_lock_key(&second));
    }
}
