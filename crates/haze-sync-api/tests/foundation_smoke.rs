use haze_sync_api::{
    auth::{AdapterPrincipal, AdapterRole},
    dto::{
        files::{FileMetadataResponse, PutFileRequestMetadata},
        primitives::{AdapterIdDto, ContentSha256Dto, RevisionIdDto, TimestampDto, VaultPathDto},
    },
};
use haze_sync_common::{AdapterId, ContentHash, RevisionId, VaultPath};

fn sha256_zero_hash() -> ContentHash {
    ContentHash::parse("sha256:0000000000000000000000000000000000000000000000000000000000000000")
        .expect("fixture hash should parse")
}

#[test]
fn api_dtos_interoperate_with_common_domain_primitives() {
    let path = VaultPath::parse("./Notes//daily.md").expect("fixture path should parse");
    let adapter_id = AdapterId::parse("iphone-anna").expect("fixture adapter id should parse");
    let revision_id =
        RevisionId::parse("rev_01JFOUNDATION").expect("fixture revision should parse");
    let hash = sha256_zero_hash();

    let metadata = FileMetadataResponse {
        path: VaultPathDto::from(path.clone()),
        revision_id: RevisionIdDto::from(revision_id.clone()),
        content_sha256: ContentSha256Dto::from(hash),
        size_bytes: 42,
        updated_by: AdapterIdDto::from(adapter_id.clone()),
        updated_at: TimestampDto::from("2026-07-01T00:00:00Z"),
    };

    let json = serde_json::to_string(&metadata).expect("metadata should serialize");
    assert!(json.contains("Notes/daily.md"));
    assert_eq!(VaultPath::try_from(&metadata.path).unwrap(), path);
    assert_eq!(
        RevisionId::try_from(&metadata.revision_id).unwrap(),
        revision_id
    );
    assert_eq!(
        ContentHash::try_from(&metadata.content_sha256).unwrap(),
        hash
    );
    assert_eq!(
        AdapterId::try_from(&metadata.updated_by).unwrap(),
        adapter_id
    );
}

#[test]
fn write_metadata_preserves_explicit_unknown_base_contract() {
    let request = PutFileRequestMetadata {
        path: VaultPathDto::from(VaultPath::parse("Notes/new.md").unwrap()),
        base_revision_id: None,
        content_sha256: ContentSha256Dto::from(sha256_zero_hash()),
        size_bytes: Some(0),
    };

    let json = serde_json::to_value(&request).expect("request should serialize");
    assert!(json["base_revision_id"].is_null());
    assert_eq!(json["path"], "Notes/new.md");
}

#[test]
fn auth_principal_validates_common_adapter_id_shape() {
    let principal = AdapterPrincipal::new("worktree-adapter", AdapterRole::WorktreeAdapter)
        .expect("principal should parse");

    assert_eq!(principal.adapter_id(), "worktree-adapter");
    assert_eq!(
        principal.common_adapter_id(),
        &AdapterId::parse("worktree-adapter").unwrap()
    );
}
