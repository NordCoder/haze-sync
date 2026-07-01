use haze_sync_common::ContentHash;
use haze_sync_storage::{LocalObjectStore, ObjectStore};
use sha2::{Digest, Sha256 as Sha256Hasher};
use std::{env, fs, path::PathBuf, time::SystemTime};

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "haze-sync-storage-fanin-{label}-{}-{timestamp}",
            std::process::id()
        ));

        Self { path }
    }

    fn store(&self) -> LocalObjectStore {
        LocalObjectStore::new(self.path.clone())
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn hash_bytes(bytes: &[u8]) -> ContentHash {
    let digest = Sha256Hasher::digest(bytes);
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(&digest[..]);
    ContentHash::from_bytes(hash)
}

#[test]
fn storage_object_store_accepts_common_content_hash() {
    let root = TempRoot::new("object-store");
    let store = root.store();
    let bytes = b"fan-in smoke content";
    let hash = hash_bytes(bytes);

    let metadata = store
        .put_bytes(hash, bytes)
        .expect("object store should accept matching hash");
    let loaded = store
        .get_bytes(hash)
        .expect("object store should read matching hash");

    assert_eq!(metadata.hash(), hash);
    assert_eq!(metadata.size_bytes(), bytes.len() as u64);
    assert_eq!(loaded, bytes);
    assert_eq!(store.exists(hash), Ok(true));
}

#[test]
fn storage_object_store_rejects_mismatched_common_hash() {
    let root = TempRoot::new("hash-mismatch");
    let store = root.store();
    let expected_hash = hash_bytes(b"expected bytes");

    let error = store
        .put_bytes(expected_hash, b"different bytes")
        .expect_err("mismatched content must be rejected");

    assert_eq!(error.code(), "content_hash_mismatch");
    assert_eq!(store.exists(expected_hash), Ok(false));
}
