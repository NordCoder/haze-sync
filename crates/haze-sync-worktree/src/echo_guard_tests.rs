use crate::*;
use haze_sync_common::{ContentHash, RevisionId, VaultPath};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

static NEXT_TEST_ROOT_ID: AtomicU64 = AtomicU64::new(0);

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let id = NEXT_TEST_ROOT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "haze-sync-echo-guard-{name}-{}-{id}",
            std::process::id()
        ));
        remove_dir_if_exists(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn config(&self) -> WorktreeConfig {
        WorktreeConfig::new(self.path.clone()).unwrap()
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        remove_dir_if_exists(&self.path);
    }
}

fn remove_dir_if_exists(path: &Path) {
    match fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => panic!("failed to clear echo-guard test root: {error}"),
    }
}

fn path(input: &str) -> VaultPath {
    VaultPath::parse(input).unwrap()
}

fn revision(input: &str) -> RevisionId {
    RevisionId::parse(input).unwrap()
}

fn hash(byte: u8) -> ContentHash {
    ContentHash::from_bytes([byte; 32])
}

fn marker(vault_path: &str, revision_id: &str, content_hash: ContentHash) -> WorktreeEchoMarker {
    WorktreeEchoMarker {
        vault_path: path(vault_path),
        revision_id: revision(revision_id),
        content_hash,
    }
}

#[test]
fn durable_materializer_marker_suppresses_once_and_is_consumed() {
    let root = TempRoot::new("durable-consume");
    let config = root.config();
    let bytes = b"hello".to_vec();
    let content_hash = ContentHash::parse(
        "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    )
    .unwrap();
    let revision_id = revision("rev_echo_1");
    let mut applied = WorktreeStateSnapshot::new();
    WorktreeMaterializer::new(config.clone())
        .materialize(
            &mut applied,
            WorktreeMaterializationRequest {
                vault_path: path("Notes/a.md"),
                revision_id: revision_id.clone(),
                content_hash,
                bytes,
            },
        )
        .unwrap();
    let scan = WorktreeScanner::new(config.clone()).scan().unwrap();
    let snapshot = scan
        .files
        .iter()
        .find(|snapshot| snapshot.vault_path == path("Notes/a.md"))
        .unwrap();

    let policy = WorktreeEchoGuardPolicy::new(Duration::from_secs(3600), 16).unwrap();
    let mut guard = WorktreeEchoGuard::new(policy);
    let now = SystemTime::now();
    let loaded = guard.load_runtime_markers(&config, now).unwrap();

    assert_eq!(loaded.loaded, 1);
    assert!(matches!(
        guard
            .consume_for_observation(
                &snapshot.vault_path,
                Some(&revision_id),
                Some(content_hash),
                Some(snapshot.content_hash),
                now,
            )
            .unwrap(),
        WorktreeEchoDecision::Suppressed(_)
    ));
    assert!(guard.is_empty());
    assert_eq!(fs::read_dir(config.echo_dir()).unwrap().count(), 0);
    assert_eq!(
        guard
            .consume_for_observation(
                &snapshot.vault_path,
                Some(&revision_id),
                Some(content_hash),
                Some(snapshot.content_hash),
                now,
            )
            .unwrap(),
        WorktreeEchoDecision::NoMarker
    );
}

#[test]
fn in_memory_marker_expires_and_cannot_suppress_later_observation() {
    let policy = WorktreeEchoGuardPolicy::new(Duration::from_secs(60), 4).unwrap();
    let mut guard = WorktreeEchoGuard::new(policy);
    let written_at = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
    guard
        .record(marker("a.md", "rev_expiring", hash(1)), written_at, written_at)
        .unwrap();

    let now = written_at + Duration::from_secs(61);
    assert_eq!(guard.expire(now).unwrap(), 1);
    assert_eq!(guard.len(), 0);
    assert_eq!(
        guard
            .consume_for_observation(
                &path("a.md"),
                Some(&revision("rev_expiring")),
                Some(hash(1)),
                Some(hash(1)),
                now,
            )
            .unwrap(),
        WorktreeEchoDecision::NoMarker
    );
}

#[test]
fn capacity_bound_evicts_oldest_path_marker() {
    let policy = WorktreeEchoGuardPolicy::new(Duration::from_secs(3600), 2).unwrap();
    let mut guard = WorktreeEchoGuard::new(policy);
    let base = SystemTime::UNIX_EPOCH + Duration::from_secs(100);

    guard
        .record(marker("one.md", "rev_one", hash(1)), base, base)
        .unwrap();
    guard
        .record(
            marker("two.md", "rev_two", hash(2)),
            base + Duration::from_secs(1),
            base + Duration::from_secs(1),
        )
        .unwrap();
    let summary = guard
        .record(
            marker("three.md", "rev_three", hash(3)),
            base + Duration::from_secs(2),
            base + Duration::from_secs(2),
        )
        .unwrap();

    assert_eq!(summary.evicted, 1);
    assert_eq!(guard.len(), 2);
    assert_eq!(
        guard
            .consume_for_observation(
                &path("one.md"),
                Some(&revision("rev_one")),
                Some(hash(1)),
                Some(hash(1)),
                base + Duration::from_secs(3),
            )
            .unwrap(),
        WorktreeEchoDecision::NoMarker
    );
}

#[test]
fn rejects_zero_ttl_or_capacity() {
    assert_eq!(
        WorktreeEchoGuardPolicy::new(Duration::ZERO, 1).unwrap_err(),
        WorktreeEchoGuardError::InvalidPolicy
    );
    assert_eq!(
        WorktreeEchoGuardPolicy::new(Duration::from_secs(1), 0).unwrap_err(),
        WorktreeEchoGuardError::InvalidPolicy
    );
}
