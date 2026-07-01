//! Local content-addressed object store primitives.
//!
//! Blob paths are derived only from validated SHA-256 values and never from
//! vault paths or provider payloads. Bytes are written to an internal temporary
//! directory, verified against the expected SHA-256, and then committed as an
//! immutable blob under the canonical `sha256/<prefix>/<full-hex>` layout.

use haze_sync_common::ContentHash;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256 as Sha256Hasher};
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const HASH_ALGORITHM_DIR: &str = "sha256";
const TEMP_DIR_NAME: &str = "tmp";
const HASH_PREFIX_HEX_LEN: usize = 2;
const READ_BUFFER_SIZE: usize = 64 * 1024;

/// Result type returned by object store operations.
pub type ObjectStoreResult<T> = Result<T, ObjectStoreError>;

/// Minimal object store operations required by the Haze Sync storage boundary.
pub trait ObjectStore {
    /// Stores `bytes` as `expected_hash` after verifying the written content.
    fn put_bytes(
        &self,
        expected_hash: ContentHash,
        bytes: &[u8],
    ) -> ObjectStoreResult<ObjectMetadata>;

    /// Reads and verifies the blob addressed by `hash`.
    fn get_bytes(&self, hash: ContentHash) -> ObjectStoreResult<Vec<u8>>;

    /// Returns true only when the committed blob exists and verifies successfully.
    fn exists(&self, hash: ContentHash) -> ObjectStoreResult<bool>;

    /// Returns verified committed blob metadata, or `None` when it is absent.
    fn stat(&self, hash: ContentHash) -> ObjectStoreResult<Option<ObjectMetadata>>;
}

/// Verified metadata for a committed object-store blob.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObjectMetadata {
    hash: ContentHash,
    size_bytes: u64,
}

impl ObjectMetadata {
    #[must_use]
    pub const fn new(hash: ContentHash, size_bytes: u64) -> Self {
        Self { hash, size_bytes }
    }

    #[must_use]
    pub const fn hash(&self) -> ContentHash {
        self.hash
    }

    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
}

/// Filesystem-backed content-addressed object store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalObjectStore {
    root: PathBuf,
}

impl LocalObjectStore {
    /// Creates a local object store rooted at a caller-provided directory.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Caller-configured object store root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Canonical relative path for a committed blob.
    #[must_use]
    pub fn relative_blob_path(hash: ContentHash) -> PathBuf {
        let hex = hash.as_hex();
        PathBuf::from(HASH_ALGORITHM_DIR)
            .join(&hex[..HASH_PREFIX_HEX_LEN])
            .join(hex)
    }

    fn blob_path(&self, hash: ContentHash) -> PathBuf {
        self.root.join(Self::relative_blob_path(hash))
    }

    fn temp_dir(&self) -> PathBuf {
        self.root.join(TEMP_DIR_NAME)
    }

    fn ensure_parent_dirs(&self, final_path: &Path) -> ObjectStoreResult<()> {
        let parent = final_path.parent().ok_or(ObjectStoreError::InvalidLayout)?;
        fs::create_dir_all(parent)
            .map_err(|source| ObjectStoreError::io("create_blob_parent", source))
    }

    fn write_temp_blob(
        &self,
        expected_hash: ContentHash,
        bytes: &[u8],
    ) -> ObjectStoreResult<PathBuf> {
        let tmp_dir = self.temp_dir();
        fs::create_dir_all(&tmp_dir)
            .map_err(|source| ObjectStoreError::io("create_temp_dir", source))?;

        let temp_path = tmp_dir.join(temp_blob_name(expected_hash));
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|source| ObjectStoreError::io("create_temp_blob", source))?;

        temp_file
            .write_all(bytes)
            .map_err(|source| ObjectStoreError::io("write_temp_blob", source))?;
        temp_file
            .sync_all()
            .map_err(|source| ObjectStoreError::io("sync_temp_blob", source))?;
        drop(temp_file);

        Ok(temp_path)
    }

    fn commit_temp_blob(
        &self,
        temp_path: &Path,
        final_path: &Path,
        expected_hash: ContentHash,
    ) -> ObjectStoreResult<ObjectMetadata> {
        if let Some(metadata) = self.stat(expected_hash)? {
            remove_temp_file(temp_path)?;
            return Ok(metadata);
        }

        fs::hard_link(temp_path, final_path).map_err(|source| {
            if source.kind() == io::ErrorKind::AlreadyExists {
                ObjectStoreError::AlreadyCommitted
            } else {
                ObjectStoreError::io("commit_blob", source)
            }
        })?;

        remove_temp_file(temp_path)?;
        self.stat(expected_hash)?
            .ok_or(ObjectStoreError::MissingBlob {
                hash: expected_hash,
            })
    }

    fn remove_temp_after_error(&self, temp_path: &Path) {
        let _ = fs::remove_file(temp_path);
    }
}

impl ObjectStore for LocalObjectStore {
    fn put_bytes(
        &self,
        expected_hash: ContentHash,
        bytes: &[u8],
    ) -> ObjectStoreResult<ObjectMetadata> {
        if let Some(metadata) = self.stat(expected_hash)? {
            return Ok(metadata);
        }

        let actual_hash = hash_bytes(bytes);
        if actual_hash != expected_hash {
            return Err(ObjectStoreError::HashMismatch {
                expected: expected_hash,
                actual: actual_hash,
            });
        }

        let final_path = self.blob_path(expected_hash);
        self.ensure_parent_dirs(&final_path)?;
        let temp_path = self.write_temp_blob(expected_hash, bytes)?;

        let written_hash = match hash_file(&temp_path) {
            Ok(hash) => hash,
            Err(error) => {
                self.remove_temp_after_error(&temp_path);
                return Err(error);
            }
        };

        if written_hash != expected_hash {
            self.remove_temp_after_error(&temp_path);
            return Err(ObjectStoreError::HashMismatch {
                expected: expected_hash,
                actual: written_hash,
            });
        }

        match self.commit_temp_blob(&temp_path, &final_path, expected_hash) {
            Ok(metadata) => Ok(metadata),
            Err(ObjectStoreError::AlreadyCommitted) => {
                self.remove_temp_after_error(&temp_path);
                self.stat(expected_hash)?
                    .ok_or(ObjectStoreError::MissingBlob {
                        hash: expected_hash,
                    })
            }
            Err(error) => {
                self.remove_temp_after_error(&temp_path);
                Err(error)
            }
        }
    }

    fn get_bytes(&self, hash: ContentHash) -> ObjectStoreResult<Vec<u8>> {
        let path = self.blob_path(hash);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                return Err(ObjectStoreError::MissingBlob { hash });
            }
            Err(source) => return Err(ObjectStoreError::io("read_blob", source)),
        };

        let actual_hash = hash_bytes(&bytes);
        if actual_hash != hash {
            return Err(ObjectStoreError::StoredBlobMismatch { hash });
        }

        Ok(bytes)
    }

    fn exists(&self, hash: ContentHash) -> ObjectStoreResult<bool> {
        self.stat(hash).map(|metadata| metadata.is_some())
    }

    fn stat(&self, hash: ContentHash) -> ObjectStoreResult<Option<ObjectMetadata>> {
        let path = self.blob_path(hash);
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(ObjectStoreError::io("stat_blob", source)),
        };

        if !metadata.is_file() {
            return Err(ObjectStoreError::UnexpectedObjectType);
        }

        let actual_hash = hash_file(&path)?;
        if actual_hash != hash {
            return Err(ObjectStoreError::StoredBlobMismatch { hash });
        }

        Ok(Some(ObjectMetadata::new(hash, metadata.len())))
    }
}

/// Safe object store errors. Display output intentionally excludes filesystem paths.
#[derive(Debug)]
pub enum ObjectStoreError {
    /// Content bytes did not match the caller-provided expected SHA-256.
    HashMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },
    /// A committed blob exists at the hash path but its bytes do not verify.
    StoredBlobMismatch { hash: ContentHash },
    /// The committed blob is absent.
    MissingBlob { hash: ContentHash },
    /// Object store path derivation produced an invalid internal layout.
    InvalidLayout,
    /// A committed object-store entry is not a regular file.
    UnexpectedObjectType,
    /// A concurrent writer committed the blob before this write finalized.
    AlreadyCommitted,
    /// Filesystem operation failed. The operation label is safe and path-free.
    Io {
        operation: &'static str,
        source: io::Error,
    },
}

impl ObjectStoreError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::HashMismatch { .. } => "content_hash_mismatch",
            Self::StoredBlobMismatch { .. } => "stored_blob_hash_mismatch",
            Self::MissingBlob { .. } => "missing_blob",
            Self::InvalidLayout => "invalid_object_store_layout",
            Self::UnexpectedObjectType => "unexpected_object_store_entry_type",
            Self::AlreadyCommitted => "blob_already_committed",
            Self::Io { .. } => "object_store_io_error",
        }
    }

    fn io(operation: &'static str, source: io::Error) -> Self {
        Self::Io { operation, source }
    }
}

impl fmt::Display for ObjectStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::HashMismatch { .. } => "content bytes do not match expected SHA-256",
            Self::StoredBlobMismatch { .. } => "stored blob bytes do not match requested SHA-256",
            Self::MissingBlob { .. } => "content blob is missing",
            Self::InvalidLayout => "object store layout is invalid",
            Self::UnexpectedObjectType => "object store entry is not a regular blob file",
            Self::AlreadyCommitted => "content blob was already committed",
            Self::Io { .. } => "object store filesystem operation failed",
        })
    }
}

impl Error for ObjectStoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[must_use]
fn hash_bytes(bytes: &[u8]) -> ContentHash {
    let digest = Sha256Hasher::digest(bytes);
    content_hash_from_digest(&digest)
}

fn hash_file(path: &Path) -> ObjectStoreResult<ContentHash> {
    let mut file = File::open(path).map_err(|source| ObjectStoreError::io("open_blob", source))?;
    let mut hasher = Sha256Hasher::new();
    let mut buffer = [0_u8; READ_BUFFER_SIZE];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|source| ObjectStoreError::io("read_blob", source))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let digest = hasher.finalize();
    Ok(content_hash_from_digest(&digest))
}

fn content_hash_from_digest(digest: &[u8]) -> ContentHash {
    let mut bytes = [0_u8; 32];
    bytes.copy_from_slice(digest);
    ContentHash::from_bytes(bytes)
}

fn temp_blob_name(expected_hash: ContentHash) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!(
        "blob-{}-{}-{timestamp}.tmp",
        expected_hash.as_hex(),
        std::process::id()
    )
}

fn remove_temp_file(path: &Path) -> ObjectStoreResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(ObjectStoreError::io("remove_temp_blob", source)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    struct TestRoot {
        path: PathBuf,
    }

    impl TestRoot {
        fn new(name: &str) -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after Unix epoch")
                .as_nanos();
            let mut path = env::temp_dir();
            path.push(format!(
                "haze-sync-object-store-{name}-{}-{timestamp}",
                std::process::id()
            ));
            Self { path }
        }

        fn store(&self) -> LocalObjectStore {
            LocalObjectStore::new(self.path.clone())
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn writes_and_reads_blob() {
        let root = TestRoot::new("write-read");
        let store = root.store();
        let bytes = b"immutable blob bytes";
        let hash = hash_bytes(bytes);

        let metadata = store
            .put_bytes(hash, bytes)
            .expect("blob should be stored");
        let loaded = store.get_bytes(hash).expect("blob should be readable");

        assert_eq!(metadata.hash(), hash);
        assert_eq!(metadata.size_bytes(), bytes.len() as u64);
        assert_eq!(loaded.as_slice(), bytes);
    }

    #[test]
    fn re_put_same_blob_is_idempotent() {
        let root = TestRoot::new("idempotent");
        let store = root.store();
        let bytes = b"same content";
        let hash = hash_bytes(bytes);

        let first = store
            .put_bytes(hash, bytes)
            .expect("initial put should succeed");
        let second = store
            .put_bytes(hash, bytes)
            .expect("duplicate put should succeed");

        assert_eq!(first, second);
        assert_eq!(store.get_bytes(hash).unwrap().as_slice(), bytes);
    }

    #[test]
    fn hash_mismatch_is_rejected_without_committed_blob() {
        let root = TestRoot::new("mismatch");
        let store = root.store();
        let expected_hash = hash_bytes(b"expected bytes");
        let actual_bytes = b"actual bytes";

        let error = store
            .put_bytes(expected_hash, actual_bytes)
            .expect_err("mismatched bytes should be rejected");

        assert_eq!(error.code(), "content_hash_mismatch");
        assert!(!store.exists(expected_hash).unwrap());
    }

    #[test]
    fn object_path_uses_sha256_prefix_layout() {
        let bytes = b"prefix layout fixture";
        let hash = hash_bytes(bytes);
        let hex = hash.as_hex();

        assert_eq!(
            LocalObjectStore::relative_blob_path(hash),
            PathBuf::from("sha256").join(&hex[..2]).join(&hex)
        );
    }

    #[test]
    fn exists_returns_true_and_false_correctly() {
        let root = TestRoot::new("exists");
        let store = root.store();
        let bytes = b"present blob";
        let present_hash = hash_bytes(bytes);
        let missing_hash = hash_bytes(b"missing blob");

        assert!(!store.exists(present_hash).unwrap());
        assert!(!store.exists(missing_hash).unwrap());

        store
            .put_bytes(present_hash, bytes)
            .expect("put should succeed");

        assert!(store.exists(present_hash).unwrap());
        assert!(!store.exists(missing_hash).unwrap());
    }

    #[test]
    fn temp_files_are_not_committed_blobs() {
        let root = TestRoot::new("temp-not-committed");
        let store = root.store();
        let bytes = b"temporary content";
        let hash = hash_bytes(bytes);
        let tmp_dir = store.temp_dir();

        fs::create_dir_all(&tmp_dir).expect("tmp dir should be creatable");
        fs::write(tmp_dir.join("blob-manual.tmp"), bytes).expect("tmp file should be writable");

        assert!(!store.exists(hash).unwrap());
        assert_eq!(store.get_bytes(hash).unwrap_err().code(), "missing_blob");
        assert!(!store.blob_path(hash).exists());
    }
}
