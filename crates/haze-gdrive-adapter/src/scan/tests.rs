use super::*;
use crate::drive::{
    DriveMetadata, FakeDriveProvider, MIME_GOOGLE_DOC, MIME_GOOGLE_FOLDER, MIME_TEXT_MARKDOWN,
};

const OBSERVED_AT: &str = "2026-07-10T09:00:00Z";

fn timestamp(value: &str) -> SafeTimestamp {
    SafeTimestamp::new(value).expect("timestamp")
}

fn mapping(path: &str, provider_id: &str, parent_id: &str, name: &str) -> GDriveMapping {
    GDriveMapping::new(
        VaultPath::new(path).expect("path"),
        provider_id,
        parent_id,
        name,
    )
    .expect("mapping")
}

fn scan(
    provider: &impl DriveProvider,
    mode: AdapterMode,
    dry_run: bool,
    mappings: &[GDriveMapping],
) -> Result<FullScanPlan, FullScanError> {
    plan_full_scan(
        provider,
        FullScanInput {
            root_folder_id: "root",
            mode,
            dry_run,
            observed_at: timestamp(OBSERVED_AT),
            mappings,
        },
    )
}

#[test]
fn full_scan_plans_new_modified_missing_and_unsupported_entries() {
    let notes_folder = DriveMetadata::new_special("folder-notes", "Notes", MIME_GOOGLE_FOLDER)
        .with_parent("root");
    let new_file = DriveMetadata::new_file("new", "new.md", MIME_TEXT_MARKDOWN)
        .with_parent("folder-notes")
        .with_size_bytes(3)
        .with_md5_checksum("md5-new")
        .with_modified_time("2026-07-10T08:00:00Z");
    let modified_file = DriveMetadata::new_file("modified", "modified.md", MIME_TEXT_MARKDOWN)
        .with_parent("folder-notes")
        .with_size_bytes(8)
        .with_md5_checksum("md5-current")
        .with_modified_time("2026-07-10T08:01:00Z");
    let unchanged_file = DriveMetadata::new_file("unchanged", "unchanged.md", MIME_TEXT_MARKDOWN)
        .with_parent("folder-notes")
        .with_md5_checksum("md5-same")
        .with_modified_time("2026-07-10T08:02:00Z");
    let google_doc =
        DriveMetadata::new_special("doc", "draft", MIME_GOOGLE_DOC).with_parent("folder-notes");
    let invalid_path =
        DriveMetadata::new_file("invalid", "../escape.md", MIME_TEXT_MARKDOWN).with_parent("root");
    let provider = FakeDriveProvider::new()
        .with_child("root", notes_folder)
        .with_child("root", invalid_path)
        .with_child("folder-notes", new_file)
        .with_child("folder-notes", modified_file)
        .with_child("folder-notes", unchanged_file)
        .with_child("folder-notes", google_doc)
        .with_content("new", b"new".to_vec())
        .with_content("modified", b"modified".to_vec());

    let mut modified_mapping = mapping(
        "Notes/modified.md",
        "modified",
        "folder-notes",
        "modified.md",
    );
    modified_mapping.checksum = Some("md5-old".to_owned());
    modified_mapping.drive_modified_time = Some(timestamp("2026-07-09T08:01:00Z"));
    modified_mapping.core_revision = Some("rev-modified".to_owned());

    let mut unchanged_mapping = mapping(
        "Notes/unchanged.md",
        "unchanged",
        "folder-notes",
        "unchanged.md",
    );
    unchanged_mapping.checksum = Some("md5-same".to_owned());
    unchanged_mapping.drive_modified_time = Some(timestamp("2026-07-10T08:02:00Z"));
    unchanged_mapping.delete_candidate_since = Some(timestamp("2026-07-09T08:00:00Z"));

    let mut missing_mapping = mapping(
        "Notes/missing.md",
        "missing",
        "folder-notes",
        "missing.md",
    );
    missing_mapping.core_revision = Some("rev-missing".to_owned());
    let mappings = [modified_mapping, unchanged_mapping, missing_mapping];

    let plan = scan(&provider, AdapterMode::ImportOnly, false, &mappings).expect("scan plan");

    assert_eq!(plan.imports.len(), 2);
    let new_import = plan
        .imports
        .iter()
        .find(|import| import.change == ImportChangeKind::New)
        .expect("new import");
    assert_eq!(new_import.execution, ImportExecution::Submit);
    assert_eq!(new_import.file_type, SupportedFileType::Markdown);
    assert_eq!(new_import.request.path.as_str(), "Notes/new.md");
    assert_eq!(new_import.request.base_revision_id, None);
    assert!(new_import.request.verifies_content());

    let modified_import = plan
        .imports
        .iter()
        .find(|import| import.change == ImportChangeKind::Modified)
        .expect("modified import");
    assert_eq!(
        modified_import.request.base_revision_id.as_deref(),
        Some("rev-modified")
    );
    assert_eq!(modified_import.request.content, b"modified".to_vec());

    assert_eq!(plan.unchanged.len(), 1);
    assert!(plan.unchanged[0].clear_delete_candidate);
    assert_eq!(plan.delete_candidates.len(), 1);
    assert_eq!(plan.delete_candidates[0].path.as_str(), "Notes/missing.md");
    assert_eq!(
        plan.delete_candidates[0].base_revision_id.as_deref(),
        Some("rev-missing")
    );
    assert!(!plan.has_immediate_delete_actions());
    assert!(plan.unsupported.iter().any(|entry| {
        entry.reason
            == ScanSkipReason::ProviderUnsupported(
                UnsupportedEntryReason::GoogleWorkspaceDocument,
            )
    }));
    assert!(plan
        .unsupported
        .iter()
        .any(|entry| entry.reason == ScanSkipReason::InvalidVaultPath));
}

#[test]
fn non_import_modes_prevent_downloads() {
    let metadata =
        DriveMetadata::new_file("new", "new.md", MIME_TEXT_MARKDOWN).with_parent("root");
    let provider = FakeDriveProvider::new().with_child("root", metadata);

    for mode in [
        AdapterMode::Disabled,
        AdapterMode::ReadOnly,
        AdapterMode::ExportOnly,
    ] {
        let plan = scan(&provider, mode, false, &[]).expect("scan plan");

        assert!(plan.imports.is_empty());
        assert_eq!(plan.skipped_imports.len(), 1);
        assert_eq!(plan.skipped_imports[0].change, ImportChangeKind::New);
        assert_eq!(plan.skipped_imports[0].mode, mode);
    }
}

#[test]
fn dry_run_modes_prepare_hash_verified_request_without_submit_execution() {
    let metadata = DriveMetadata::new_file("new", "new.md", MIME_TEXT_MARKDOWN)
        .with_parent("root")
        .with_size_bytes(7);
    let provider = FakeDriveProvider::new()
        .with_child("root", metadata)
        .with_content("new", b"preview".to_vec());

    for (mode, dry_run) in [
        (AdapterMode::ImportOnly, true),
        (AdapterMode::Bidirectional, true),
        (AdapterMode::DryRun, false),
    ] {
        let plan = scan(&provider, mode, dry_run, &[]).expect("scan plan");

        assert_eq!(plan.imports.len(), 1);
        assert_eq!(plan.imports[0].execution, ImportExecution::DryRun);
        assert_eq!(
            plan.imports[0].request.content_sha256.to_string(),
            "sha256:5975cf1bba432391c94667f5886225f69377c0aa8b9fa21fddfb21c89bcf9092"
        );
        assert!(plan.imports[0].request.verifies_content());
    }
}

#[test]
fn common_compatible_normalization_detects_path_collisions_before_download() {
    let first = DriveMetadata::new_file("first", "a.md", MIME_TEXT_MARKDOWN).with_parent("root");
    let second =
        DriveMetadata::new_file("second", "a%2emd", MIME_TEXT_MARKDOWN).with_parent("root");
    let provider = FakeDriveProvider::new()
        .with_child("root", first)
        .with_child("root", second);
    let mapped = mapping("a.md", "previous", "root", "a.md");

    let plan = scan(&provider, AdapterMode::ImportOnly, false, &[mapped]).expect("scan plan");

    assert!(plan.imports.is_empty());
    assert_eq!(
        plan.unsupported
            .iter()
            .filter(|entry| entry.reason == ScanSkipReason::PathCollision)
            .count(),
        2
    );
    assert!(plan.delete_candidates.is_empty());
}

#[test]
fn provider_names_cannot_inject_vault_path_segments() {
    let raw_separator =
        DriveMetadata::new_file("raw", "nested/file.md", MIME_TEXT_MARKDOWN).with_parent("root");
    let encoded_separator =
        DriveMetadata::new_file("encoded", "nested%2Ffile.md", MIME_TEXT_MARKDOWN)
            .with_parent("root");
    let provider = FakeDriveProvider::new()
        .with_child("root", raw_separator)
        .with_child("root", encoded_separator);

    let plan = scan(&provider, AdapterMode::ImportOnly, false, &[]).expect("scan plan");

    assert!(plan.imports.is_empty());
    assert_eq!(
        plan.unsupported
            .iter()
            .filter(|entry| entry.reason == ScanSkipReason::InvalidVaultPath)
            .count(),
        2
    );
}

#[test]
fn mapped_identity_path_change_is_not_sent_as_content_modification() {
    let renamed = DriveMetadata::new_file("mapped", "renamed.md", MIME_TEXT_MARKDOWN)
        .with_parent("root")
        .with_md5_checksum("same-md5")
        .with_modified_time("2026-07-10T08:00:00Z");
    let provider = FakeDriveProvider::new().with_child("root", renamed);
    let mut mapped = mapping("old.md", "mapped", "root", "old.md");
    mapped.checksum = Some("same-md5".to_owned());
    mapped.drive_modified_time = Some(timestamp("2026-07-10T08:00:00Z"));
    mapped.core_revision = Some("rev-old".to_owned());

    let plan = scan(&provider, AdapterMode::ImportOnly, false, &[mapped]).expect("scan plan");

    assert!(plan.imports.is_empty());
    assert!(plan.delete_candidates.is_empty());
    assert!(plan.unsupported.iter().any(|entry| {
        entry.reason == ScanSkipReason::MappingIdentityPathChanged
            && entry.path.as_ref().is_some_and(|path| path.as_str() == "renamed.md")
    }));
}

#[test]
fn different_identity_on_mapped_path_blocks_import_and_delete_candidate() {
    let replacement =
        DriveMetadata::new_file("replacement", "same.md", MIME_TEXT_MARKDOWN).with_parent("root");
    let provider = FakeDriveProvider::new().with_child("root", replacement);
    let mut mapped = mapping("same.md", "previous", "root", "same.md");
    mapped.core_revision = Some("rev-current".to_owned());

    let plan = scan(&provider, AdapterMode::ImportOnly, false, &[mapped]).expect("scan plan");

    assert!(plan.imports.is_empty());
    assert!(plan.delete_candidates.is_empty());
    assert!(plan.unsupported.iter().any(|entry| {
        entry.reason == ScanSkipReason::MappingPathOccupiedByDifferentIdentity
            && entry.provider_id == "replacement"
    }));
}

#[test]
fn upload_request_debug_redacts_raw_content() {
    let content = vec![222, 173, 190, 239];
    let request = CoreUploadRequest {
        path: VaultPath::new("safe.bin").expect("path"),
        base_revision_id: Some("rev-safe".to_owned()),
        content_sha256: ContentSha256::from_content(&content),
        size_bytes: content.len() as u64,
        content,
    };

    let rendered = format!("{request:?}");

    assert!(rendered.contains("<redacted 4 bytes>"));
    assert!(!rendered.contains("[222, 173, 190, 239]"));
}

#[test]
fn size_mismatch_aborts_plan_before_any_core_side_effect() {
    let metadata = DriveMetadata::new_file("file", "file.md", MIME_TEXT_MARKDOWN)
        .with_parent("root")
        .with_size_bytes(100);
    let provider = FakeDriveProvider::new()
        .with_child("root", metadata)
        .with_content("file", b"short".to_vec());

    let error = scan(&provider, AdapterMode::ImportOnly, false, &[])
        .expect_err("size mismatch must fail");

    assert_eq!(error, FullScanError::ContentSizeMismatch);
}

#[test]
fn folder_cycle_is_skipped_safely() {
    let cycle =
        DriveMetadata::new_special("root", "cycle", MIME_GOOGLE_FOLDER).with_parent("root");
    let provider = FakeDriveProvider::new().with_child("root", cycle);

    let plan = scan(&provider, AdapterMode::ImportOnly, false, &[]).expect("cycle is skipped");

    assert!(plan
        .unsupported
        .iter()
        .any(|entry| entry.reason == ScanSkipReason::FolderCycle));
}
