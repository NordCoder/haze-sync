//! Full Drive subtree scan and safe Core import planning.
//!
//! The planner is deliberately side-effect free with respect to Core and
//! persistence. It reads provider metadata/content through `DriveProvider`,
//! consumes an injected mapping snapshot, and returns an in-memory plan.

use crate::config::AdapterMode;
use crate::drive::{
    normalize_drive_metadata, DriveEntryClassification, DriveEntryKind, DriveMetadata,
    DriveProvider, ProviderError, SupportedFileType, UnsupportedEntryReason,
};
use crate::hash::ContentSha256;
use crate::state::{GDriveMapping, SafeTimestamp, VaultPath};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportChangeKind {
    New,
    Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportExecution {
    Submit,
    DryRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreUploadRequest {
    pub path: VaultPath,
    pub base_revision_id: Option<String>,
    pub content_sha256: ContentSha256,
    pub size_bytes: u64,
    pub content: Vec<u8>,
}

impl CoreUploadRequest {
    #[must_use]
    pub fn verifies_content(&self) -> bool {
        self.size_bytes == self.content.len() as u64 && self.content_sha256.verifies(&self.content)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedImport {
    pub change: ImportChangeKind,
    pub execution: ImportExecution,
    pub provider_id: String,
    pub parent_id: String,
    pub provider_name: String,
    pub provider_md5_checksum: Option<String>,
    pub provider_modified_time: Option<String>,
    pub clear_delete_candidate: bool,
    pub request: CoreUploadRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedImport {
    pub change: ImportChangeKind,
    pub provider_id: String,
    pub path: VaultPath,
    pub mode: AdapterMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnchangedDriveEntry {
    pub provider_id: String,
    pub path: VaultPath,
    pub clear_delete_candidate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteCandidatePlan {
    pub provider_id: String,
    pub path: VaultPath,
    pub base_revision_id: Option<String>,
    pub detected_at: SafeTimestamp,
    pub previously_detected_at: Option<SafeTimestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanSkipReason {
    ProviderUnsupported(UnsupportedEntryReason),
    InvalidVaultPath,
    DuplicateProviderEntry,
    FolderCycle,
    PathCollision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedScanEntry {
    pub provider_id: String,
    pub path: Option<VaultPath>,
    pub reason: ScanSkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FullScanPlan {
    pub imports: Vec<PlannedImport>,
    pub skipped_imports: Vec<SkippedImport>,
    pub unchanged: Vec<UnchangedDriveEntry>,
    pub delete_candidates: Vec<DeleteCandidatePlan>,
    pub unsupported: Vec<SkippedScanEntry>,
}

impl FullScanPlan {
    #[must_use]
    pub fn has_immediate_delete_actions(&self) -> bool {
        false
    }
}

pub struct FullScanInput<'a> {
    pub root_folder_id: &'a str,
    pub mode: AdapterMode,
    pub dry_run: bool,
    pub observed_at: SafeTimestamp,
    pub mappings: &'a [GDriveMapping],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullScanError {
    InvalidRootFolder,
    DuplicateMappingDriveFileId,
    DuplicateMappingPath,
    Provider(ProviderError),
    ContentSizeMismatch,
    ContentHashVerificationFailed,
}

impl fmt::Display for FullScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRootFolder => formatter.write_str("scan root folder id must not be empty"),
            Self::DuplicateMappingDriveFileId => {
                formatter.write_str("mapping snapshot contains duplicate Drive file identity")
            }
            Self::DuplicateMappingPath => {
                formatter.write_str("mapping snapshot contains duplicate vault path")
            }
            Self::Provider(error) => write!(formatter, "full scan provider failure: {error}"),
            Self::ContentSizeMismatch => {
                formatter.write_str("downloaded content size does not match provider metadata")
            }
            Self::ContentHashVerificationFailed => {
                formatter.write_str("computed upload content hash verification failed")
            }
        }
    }
}

impl Error for FullScanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Provider(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ProviderError> for FullScanError {
    fn from(error: ProviderError) -> Self {
        Self::Provider(error)
    }
}

#[derive(Debug, Clone)]
struct CollectedFile {
    metadata: DriveMetadata,
    parent_id: String,
    path: VaultPath,
    file_type: SupportedFileType,
}

#[derive(Default)]
struct ScanCollection {
    supported_files: Vec<CollectedFile>,
    unsupported: Vec<SkippedScanEntry>,
    seen_provider_ids: BTreeSet<String>,
    visited_folder_ids: BTreeSet<String>,
}

pub fn plan_full_scan(
    provider: &impl DriveProvider,
    input: FullScanInput<'_>,
) -> Result<FullScanPlan, FullScanError> {
    let root_folder_id = input.root_folder_id.trim();
    if root_folder_id.is_empty() {
        return Err(FullScanError::InvalidRootFolder);
    }

    let indexes = MappingIndexes::new(input.mappings)?;
    let mut collection = ScanCollection::default();
    collection
        .visited_folder_ids
        .insert(root_folder_id.to_owned());
    collect_folder(
        provider,
        root_folder_id,
        &[],
        true,
        &mut collection,
    )?;

    let mut plan = FullScanPlan {
        unsupported: collection.unsupported,
        ..FullScanPlan::default()
    };
    plan_supported_files(
        provider,
        input.mode,
        input.dry_run,
        &indexes,
        collection.supported_files,
        &mut plan,
    )?;
    plan_missing_mappings(
        input.mappings,
        &collection.seen_provider_ids,
        &input.observed_at,
        &mut plan,
    );
    sort_plan(&mut plan);
    Ok(plan)
}

struct MappingIndexes<'a> {
    by_drive_file_id: BTreeMap<String, &'a GDriveMapping>,
}

impl<'a> MappingIndexes<'a> {
    fn new(mappings: &'a [GDriveMapping]) -> Result<Self, FullScanError> {
        let mut by_drive_file_id = BTreeMap::new();
        let mut paths = BTreeSet::new();

        for mapping in mappings {
            if by_drive_file_id
                .insert(mapping.drive_file_id.clone(), mapping)
                .is_some()
            {
                return Err(FullScanError::DuplicateMappingDriveFileId);
            }
            if !paths.insert(mapping.vault_path.clone()) {
                return Err(FullScanError::DuplicateMappingPath);
            }
        }

        Ok(Self { by_drive_file_id })
    }

    fn by_provider_id(&self, provider_id: &str) -> Option<&'a GDriveMapping> {
        self.by_drive_file_id.get(provider_id).copied()
    }
}

fn collect_folder(
    provider: &impl DriveProvider,
    folder_id: &str,
    parent_segments: &[String],
    ancestor_path_valid: bool,
    collection: &mut ScanCollection,
) -> Result<(), FullScanError> {
    let mut children = provider.list_children(folder_id)?;
    children.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });

    for metadata in children {
        let mut raw_segments = parent_segments.to_vec();
        raw_segments.push(metadata.name.clone());
        let normalized_path = if ancestor_path_valid {
            normalize_common_compatible_path(&raw_segments).ok()
        } else {
            None
        };

        if !collection.seen_provider_ids.insert(metadata.id.clone()) {
            collection.unsupported.push(SkippedScanEntry {
                provider_id: metadata.id,
                path: normalized_path,
                reason: ScanSkipReason::DuplicateProviderEntry,
            });
            continue;
        }

        if metadata.kind == DriveEntryKind::Folder {
            let folder_path_valid = normalized_path.is_some() && !metadata.name.trim().is_empty();
            if !folder_path_valid {
                collection.unsupported.push(SkippedScanEntry {
                    provider_id: metadata.id.clone(),
                    path: normalized_path,
                    reason: ScanSkipReason::InvalidVaultPath,
                });
            }

            if !collection.visited_folder_ids.insert(metadata.id.clone()) {
                collection.unsupported.push(SkippedScanEntry {
                    provider_id: metadata.id,
                    path: normalized_path,
                    reason: ScanSkipReason::FolderCycle,
                });
                continue;
            }

            collect_folder(
                provider,
                &metadata.id,
                &raw_segments,
                ancestor_path_valid && folder_path_valid,
                collection,
            )?;
            continue;
        }

        let normalized = normalize_drive_metadata(&metadata);
        let Some(path) = normalized_path else {
            collection.unsupported.push(SkippedScanEntry {
                provider_id: metadata.id,
                path: None,
                reason: ScanSkipReason::InvalidVaultPath,
            });
            continue;
        };

        match normalized.classification {
            DriveEntryClassification::Supported(file_type) => {
                collection.supported_files.push(CollectedFile {
                    metadata,
                    parent_id: folder_id.to_owned(),
                    path,
                    file_type,
                });
            }
            DriveEntryClassification::Unsupported(reason) => {
                collection.unsupported.push(SkippedScanEntry {
                    provider_id: metadata.id,
                    path: Some(path),
                    reason: ScanSkipReason::ProviderUnsupported(reason),
                });
            }
        }
    }

    Ok(())
}

fn plan_supported_files(
    provider: &impl DriveProvider,
    mode: AdapterMode,
    dry_run: bool,
    indexes: &MappingIndexes<'_>,
    files: Vec<CollectedFile>,
    plan: &mut FullScanPlan,
) -> Result<(), FullScanError> {
    let mut files_by_path: BTreeMap<VaultPath, Vec<CollectedFile>> = BTreeMap::new();
    for file in files {
        files_by_path
            .entry(file.path.clone())
            .or_default()
            .push(file);
    }

    for (path, mut path_files) in files_by_path {
        if path_files.len() > 1 {
            for file in path_files {
                plan.unsupported.push(SkippedScanEntry {
                    provider_id: file.metadata.id,
                    path: Some(path.clone()),
                    reason: ScanSkipReason::PathCollision,
                });
            }
            continue;
        }

        let file = path_files.pop().expect("path group contains one file");
        let mapping = indexes.by_provider_id(&file.metadata.id);
        let change = match mapping {
            None => Some(ImportChangeKind::New),
            Some(mapping) if mapping_matches_scan(mapping, &file) => None,
            Some(_) => Some(ImportChangeKind::Modified),
        };

        let Some(change) = change else {
            let mapping = mapping.expect("unchanged file has mapping");
            plan.unchanged.push(UnchangedDriveEntry {
                provider_id: file.metadata.id,
                path,
                clear_delete_candidate: mapping.is_delete_candidate(),
            });
            continue;
        };

        let Some(execution) = import_execution(mode, dry_run) else {
            plan.skipped_imports.push(SkippedImport {
                change,
                provider_id: file.metadata.id,
                path,
                mode,
            });
            continue;
        };

        let content = provider.download_file(&file.metadata.id)?;
        if let Some(expected_size) = file.metadata.size_bytes {
            if expected_size != content.len() as u64 {
                return Err(FullScanError::ContentSizeMismatch);
            }
        }

        let content_sha256 = ContentSha256::from_content(&content);
        let request = CoreUploadRequest {
            path,
            base_revision_id: mapping.and_then(|mapping| mapping.core_revision.clone()),
            content_sha256,
            size_bytes: content.len() as u64,
            content,
        };
        if !request.verifies_content() {
            return Err(FullScanError::ContentHashVerificationFailed);
        }

        plan.imports.push(PlannedImport {
            change,
            execution,
            provider_id: file.metadata.id,
            parent_id: file.parent_id,
            provider_name: file.metadata.name,
            provider_md5_checksum: file.metadata.md5_checksum,
            provider_modified_time: file.metadata.modified_time,
            clear_delete_candidate: mapping.is_some_and(GDriveMapping::is_delete_candidate),
            request,
        });
    }

    Ok(())
}

fn mapping_matches_scan(mapping: &GDriveMapping, file: &CollectedFile) -> bool {
    if mapping.vault_path != file.path
        || mapping.parent_id != file.parent_id
        || mapping.name != file.metadata.name
    {
        return false;
    }

    let mut compared_content_fact = false;
    if let Some(md5_checksum) = file.metadata.md5_checksum.as_deref() {
        compared_content_fact = true;
        if mapping.checksum.as_deref() != Some(md5_checksum) {
            return false;
        }
    }
    if let Some(modified_time) = file.metadata.modified_time.as_deref() {
        compared_content_fact = true;
        if mapping
            .drive_modified_time
            .as_ref()
            .map(SafeTimestamp::as_str)
            != Some(modified_time)
        {
            return false;
        }
    }

    compared_content_fact
}

fn plan_missing_mappings(
    mappings: &[GDriveMapping],
    seen_provider_ids: &BTreeSet<String>,
    observed_at: &SafeTimestamp,
    plan: &mut FullScanPlan,
) {
    for mapping in mappings {
        if seen_provider_ids.contains(&mapping.drive_file_id) {
            continue;
        }

        plan.delete_candidates.push(DeleteCandidatePlan {
            provider_id: mapping.drive_file_id.clone(),
            path: mapping.vault_path.clone(),
            base_revision_id: mapping.core_revision.clone(),
            detected_at: observed_at.clone(),
            previously_detected_at: mapping.delete_candidate_since.clone(),
        });
    }
}

fn import_execution(mode: AdapterMode, dry_run: bool) -> Option<ImportExecution> {
    match mode {
        AdapterMode::ImportOnly | AdapterMode::Bidirectional if dry_run => {
            Some(ImportExecution::DryRun)
        }
        AdapterMode::ImportOnly | AdapterMode::Bidirectional => Some(ImportExecution::Submit),
        AdapterMode::DryRun => Some(ImportExecution::DryRun),
        AdapterMode::Disabled | AdapterMode::ReadOnly | AdapterMode::ExportOnly => None,
    }
}

fn sort_plan(plan: &mut FullScanPlan) {
    plan.imports
        .sort_by(|left, right| left.request.path.cmp(&right.request.path));
    plan.skipped_imports
        .sort_by(|left, right| left.path.cmp(&right.path));
    plan.unchanged
        .sort_by(|left, right| left.path.cmp(&right.path));
    plan.delete_candidates
        .sort_by(|left, right| left.path.cmp(&right.path));
    plan.unsupported.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.provider_id.cmp(&right.provider_id))
    });
}

fn normalize_common_compatible_path(segments: &[String]) -> Result<VaultPath, ()> {
    let raw = segments.join("/");
    let decoded = decode_percent_sequences(&raw)?;
    let decoded = decoded.as_ref();

    if decoded.is_empty()
        || decoded.as_bytes().contains(&0)
        || has_windows_drive_prefix(decoded)
        || decoded.contains('\\')
        || decoded.starts_with('/')
        || decoded == "~"
        || decoded.starts_with("~/")
    {
        return Err(());
    }

    let mut normalized_segments = Vec::new();
    for segment in decoded.split('/') {
        match segment {
            "" | "." => {}
            ".." => return Err(()),
            safe_segment => normalized_segments.push(safe_segment),
        }
    }
    if normalized_segments.is_empty() {
        return Err(());
    }

    let normalized = normalized_segments.join("/");
    if is_reserved_runtime_path(&normalized) {
        return Err(());
    }

    Ok(VaultPath::from_common_normalized(normalized))
}

fn decode_percent_sequences(input: &str) -> Result<Cow<'_, str>, ()> {
    if !input.as_bytes().contains(&b'%') {
        return Ok(Cow::Borrowed(input));
    }

    let input_bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(input_bytes.len());
    let mut index = 0;
    while index < input_bytes.len() {
        if input_bytes[index] == b'%' {
            if index + 2 >= input_bytes.len() {
                return Err(());
            }
            let high = percent_nibble(input_bytes[index + 1])?;
            let low = percent_nibble(input_bytes[index + 2])?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(input_bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).map(Cow::Owned).map_err(|_| ())
}

fn percent_nibble(byte: u8) -> Result<u8, ()> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(()),
    }
}

fn has_windows_drive_prefix(input: &str) -> bool {
    let bytes = input.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn is_reserved_runtime_path(normalized: &str) -> bool {
    let first_segment = normalized.split('/').next().unwrap_or_default();
    matches!(
        first_segment,
        "_haze_runtime" | "_haze_tmp" | "state" | "logs" | "trash"
    ) || normalized.ends_with(".tmp")
        || normalized.ends_with(".part")
        || normalized.ends_with(".swp")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::{
        DriveMetadata, FakeDriveProvider, MIME_GOOGLE_DOC, MIME_GOOGLE_FOLDER, MIME_TEXT_MARKDOWN,
    };

    fn timestamp(value: &str) -> SafeTimestamp {
        SafeTimestamp::new(value).expect("timestamp")
    }

    fn mapping(
        path: &str,
        provider_id: &str,
        parent_id: &str,
        name: &str,
    ) -> GDriveMapping {
        GDriveMapping::new(
            VaultPath::new(path).expect("path"),
            provider_id,
            parent_id,
            name,
        )
        .expect("mapping")
    }

    #[test]
    fn full_scan_plans_new_modified_missing_and_unsupported_entries() {
        let notes_folder =
            DriveMetadata::new_special("folder-notes", "Notes", MIME_GOOGLE_FOLDER)
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
        let unchanged_file =
            DriveMetadata::new_file("unchanged", "unchanged.md", MIME_TEXT_MARKDOWN)
                .with_parent("folder-notes")
                .with_md5_checksum("md5-same")
                .with_modified_time("2026-07-10T08:02:00Z");
        let google_doc =
            DriveMetadata::new_special("doc", "draft", MIME_GOOGLE_DOC).with_parent("folder-notes");
        let invalid_path =
            DriveMetadata::new_file("invalid", "../escape.md", MIME_TEXT_MARKDOWN)
                .with_parent("root");
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

        let plan = plan_full_scan(
            &provider,
            FullScanInput {
                root_folder_id: "root",
                mode: AdapterMode::ImportOnly,
                dry_run: false,
                observed_at: timestamp("2026-07-10T09:00:00Z"),
                mappings: &mappings,
            },
        )
        .expect("scan plan");

        assert_eq!(plan.imports.len(), 2);
        let new_import = plan
            .imports
            .iter()
            .find(|import| import.change == ImportChangeKind::New)
            .expect("new import");
        assert_eq!(new_import.execution, ImportExecution::Submit);
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
    fn mode_rules_prevent_downloads_when_import_is_not_allowed() {
        let metadata = DriveMetadata::new_file("new", "new.md", MIME_TEXT_MARKDOWN)
            .with_parent("root");
        let provider = FakeDriveProvider::new().with_child("root", metadata);

        let plan = plan_full_scan(
            &provider,
            FullScanInput {
                root_folder_id: "root",
                mode: AdapterMode::ExportOnly,
                dry_run: false,
                observed_at: timestamp("2026-07-10T09:00:00Z"),
                mappings: &[],
            },
        )
        .expect("scan plan");

        assert!(plan.imports.is_empty());
        assert_eq!(plan.skipped_imports.len(), 1);
        assert_eq!(plan.skipped_imports[0].change, ImportChangeKind::New);
        assert_eq!(plan.skipped_imports[0].mode, AdapterMode::ExportOnly);
    }

    #[test]
    fn dry_run_prepares_hash_verified_request_without_submit_execution() {
        let metadata = DriveMetadata::new_file("new", "new.md", MIME_TEXT_MARKDOWN)
            .with_parent("root")
            .with_size_bytes(7);
        let provider = FakeDriveProvider::new()
            .with_child("root", metadata)
            .with_content("new", b"preview".to_vec());

        let plan = plan_full_scan(
            &provider,
            FullScanInput {
                root_folder_id: "root",
                mode: AdapterMode::Bidirectional,
                dry_run: true,
                observed_at: timestamp("2026-07-10T09:00:00Z"),
                mappings: &[],
            },
        )
        .expect("scan plan");

        assert_eq!(plan.imports.len(), 1);
        assert_eq!(plan.imports[0].execution, ImportExecution::DryRun);
        assert_eq!(
            plan.imports[0].request.content_sha256.to_string(),
            "sha256:2ca02c68a5b77e0808d3b7b8288dffbfa3d60f21bec42d7d0c6a2134a11c15e2"
        );
        assert!(plan.imports[0].request.verifies_content());
    }

    #[test]
    fn common_compatible_normalization_detects_path_collisions_before_download() {
        let first = DriveMetadata::new_file("first", "Notes/a.md", MIME_TEXT_MARKDOWN)
            .with_parent("root");
        let second = DriveMetadata::new_file("second", "Notes//a.md", MIME_TEXT_MARKDOWN)
            .with_parent("root");
        let provider = FakeDriveProvider::new()
            .with_child("root", first)
            .with_child("root", second);

        let plan = plan_full_scan(
            &provider,
            FullScanInput {
                root_folder_id: "root",
                mode: AdapterMode::ImportOnly,
                dry_run: false,
                observed_at: timestamp("2026-07-10T09:00:00Z"),
                mappings: &[],
            },
        )
        .expect("scan plan");

        assert!(plan.imports.is_empty());
        assert_eq!(
            plan.unsupported
                .iter()
                .filter(|entry| entry.reason == ScanSkipReason::PathCollision)
                .count(),
            2
        );
    }

    #[test]
    fn size_mismatch_aborts_plan_before_any_core_side_effect() {
        let metadata = DriveMetadata::new_file("file", "file.md", MIME_TEXT_MARKDOWN)
            .with_parent("root")
            .with_size_bytes(100);
        let provider = FakeDriveProvider::new()
            .with_child("root", metadata)
            .with_content("file", b"short".to_vec());

        let error = plan_full_scan(
            &provider,
            FullScanInput {
                root_folder_id: "root",
                mode: AdapterMode::ImportOnly,
                dry_run: false,
                observed_at: timestamp("2026-07-10T09:00:00Z"),
                mappings: &[],
            },
        )
        .expect_err("size mismatch must fail");

        assert_eq!(error, FullScanError::ContentSizeMismatch);
    }

    #[test]
    fn folder_cycle_is_skipped_safely() {
        let cycle = DriveMetadata::new_special("root", "cycle", MIME_GOOGLE_FOLDER)
            .with_parent("root");
        let provider = FakeDriveProvider::new().with_child("root", cycle);

        let plan = plan_full_scan(
            &provider,
            FullScanInput {
                root_folder_id: "root",
                mode: AdapterMode::ImportOnly,
                dry_run: false,
                observed_at: timestamp("2026-07-10T09:00:00Z"),
                mappings: &[],
            },
        )
        .expect("cycle is skipped");

        assert!(plan
            .unsupported
            .iter()
            .any(|entry| entry.reason == ScanSkipReason::FolderCycle));
    }
}
