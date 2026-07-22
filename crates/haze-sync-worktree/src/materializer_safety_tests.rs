#![cfg(unix)]

use crate::{
    WorktreeConfig, WorktreeMaterializationRequest, WorktreeMaterializer, WorktreeStateSnapshot,
};
use haze_sync_common::{ContentHash, RevisionId, VaultPath};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_ROOT_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn materializer_rejects_symlinked_parent_before_reading_outside_root() {
    use std::os::unix::fs::symlink;

    let test_root = temp_root("symlink-parent");
    let worktree_root = test_root.join("worktree");
    let outside_root = test_root.join("outside");
    remove_dir_if_exists(&test_root);
    fs::create_dir_all(&worktree_root).unwrap();
    fs::create_dir_all(&outside_root).unwrap();
    fs::write(outside_root.join("a.md"), b"outside").unwrap();
    symlink(&outside_root, worktree_root.join("Notes")).unwrap();

    let materializer = WorktreeMaterializer::new(WorktreeConfig::new(worktree_root).unwrap());
    let mut state = WorktreeStateSnapshot::new();
    let request = WorktreeMaterializationRequest {
        vault_path: VaultPath::parse("Notes/a.md").unwrap(),
        revision_id: RevisionId::parse("rev_remote").unwrap(),
        content_hash: ContentHash::parse(
            "b71199ebd070b36beab7317920c2c2f1d777df8d05e5527d8458fda57cb17a7a",
        )
        .unwrap(),
        bytes: b"remote".to_vec(),
    };

    let error = materializer.materialize(&mut state, request).unwrap_err();

    assert_eq!(error.code(), "unsafe_local_component");
    assert_eq!(fs::read(outside_root.join("a.md")).unwrap(), b"outside");
    assert!(state.is_empty());

    remove_dir_if_exists(&test_root);
}

fn temp_root(name: &str) -> PathBuf {
    let id = NEXT_TEMP_ROOT_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "haze-sync-materializer-safety-{name}-{}-{id}",
        std::process::id()
    ))
}

fn remove_dir_if_exists(path: &Path) {
    match fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => panic!("failed to clear materializer safety test root: {error}"),
    }
}
