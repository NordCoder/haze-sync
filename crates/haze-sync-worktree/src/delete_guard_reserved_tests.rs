use crate::*;
use haze_sync_common::{ContentHash, RevisionId, VaultPath};

fn path(input: &str) -> VaultPath {
    VaultPath::parse(input).unwrap()
}

fn revision(input: &str) -> RevisionId {
    RevisionId::parse(input).unwrap()
}

fn hash(byte: u8) -> ContentHash {
    ContentHash::from_bytes([byte; 32])
}

#[test]
fn unscoped_reserved_runtime_skip_does_not_hide_user_delete_candidates() {
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(path("deleted.md"), revision("rev_deleted"), hash(1));
    let scan = WorktreeScanResult {
        files: Vec::new(),
        skipped: vec![WorktreeScanSkipped {
            vault_path: None,
            reason: WorktreeScanSkipReason::ReservedPath,
        }],
    };

    let delete_scan = WorktreeDeleteScan::from_scan(&state, &scan).unwrap();

    assert!(delete_scan.is_complete());
    assert_eq!(delete_scan.candidates().len(), 1);
    assert_eq!(delete_scan.candidates()[0].vault_path, path("deleted.md"));
}
