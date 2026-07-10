use haze_sync_common::{
    AdapterId, AdapterMode, AdapterRole, ConflictId, ContentHash, OperationId, RevisionId, Sha256,
    ValidationError, VaultPath,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const FIXTURE_JSON: &str = include_str!("../fixtures/common-primitives-v1.json");

#[derive(Debug, Deserialize)]
struct CompatibilityFixture {
    schema_version: u32,
    vault_paths: Vec<VaultPathFixture>,
    identifiers: IdentifierFixture,
    content_hashes: Vec<ContentHashFixture>,
    adapter_roles: Vec<String>,
    adapter_modes: Vec<AdapterModeFixture>,
    validation_errors: Vec<ValidationErrorFixture>,
}

#[derive(Debug, Deserialize)]
struct VaultPathFixture {
    input: String,
    canonical: String,
}

#[derive(Debug, Deserialize)]
struct IdentifierFixture {
    adapter_id: String,
    revision_id: String,
    operation_id: String,
    conflict_id: String,
}

#[derive(Debug, Deserialize)]
struct ContentHashFixture {
    input: String,
    canonical: String,
}

#[derive(Debug, Deserialize)]
struct AdapterModeFixture {
    wire: String,
    allows_core_reads: bool,
    allows_core_writes: bool,
}

#[derive(Debug, Deserialize)]
struct ValidationErrorFixture {
    wire: String,
    message: String,
}

fn load_fixture() -> CompatibilityFixture {
    serde_json::from_str(FIXTURE_JSON).expect("common V1 compatibility fixture must be valid JSON")
}

fn assert_serializes_to_wire<T: Serialize>(value: &T, expected: &str) {
    let actual = serde_json::to_string(value).expect("common primitive must serialize");
    assert_eq!(actual, format!("\"{expected}\""));
}

#[test]
fn fixture_schema_is_v1_and_contains_no_environment_or_secret_examples() {
    let fixture = load_fixture();
    assert_eq!(fixture.schema_version, 1);

    let lowercase = FIXTURE_JSON.to_ascii_lowercase();
    for forbidden in [
        "bearer ",
        "oauth",
        "access_token",
        "refresh_token",
        "client_secret",
        "postgres://",
        "mysql://",
        "/home/",
        "/users/",
        "http://",
        "https://",
        "localhost",
        "127.0.0.1",
    ] {
        assert!(
            !lowercase.contains(forbidden),
            "fixture contains forbidden environment/secret fragment {forbidden:?}"
        );
    }
}

#[test]
fn vault_path_examples_normalize_and_roundtrip() {
    for example in load_fixture().vault_paths {
        let path = VaultPath::parse(&example.input).expect("fixture path input must be valid");
        assert_eq!(path.as_str(), example.canonical);
        assert_serializes_to_wire(&path, &example.canonical);

        let input_json = serde_json::to_string(&example.input).unwrap();
        let decoded: VaultPath = serde_json::from_str(&input_json).unwrap();
        assert_eq!(decoded, path);
    }
}

#[test]
fn identifier_examples_match_stable_wire_values() {
    let identifiers = load_fixture().identifiers;

    let adapter_id = AdapterId::parse(&identifiers.adapter_id).unwrap();
    let revision_id = RevisionId::parse(&identifiers.revision_id).unwrap();
    let operation_id = OperationId::parse(&identifiers.operation_id).unwrap();
    let conflict_id = ConflictId::parse(&identifiers.conflict_id).unwrap();

    assert_serializes_to_wire(&adapter_id, &identifiers.adapter_id);
    assert_serializes_to_wire(&revision_id, &identifiers.revision_id);
    assert_serializes_to_wire(&operation_id, &identifiers.operation_id);
    assert_serializes_to_wire(&conflict_id, &identifiers.conflict_id);
}

#[test]
fn content_hash_examples_normalize_and_roundtrip() {
    for example in load_fixture().content_hashes {
        let hash: ContentHash = Sha256::parse(&example.input).unwrap();
        assert_eq!(hash.to_string(), example.canonical);
        assert_serializes_to_wire(&hash, &example.canonical);

        let input_json = serde_json::to_string(&example.input).unwrap();
        let decoded: ContentHash = serde_json::from_str(&input_json).unwrap();
        assert_eq!(decoded, hash);
    }
}

#[test]
fn adapter_role_examples_are_complete_and_stable() {
    const EXPECTED_ROLE_WIRES: &[&str] = &[
        "obsidian_plugin",
        "gdrive_adapter",
        "worktree_adapter",
        "admin",
        "readonly_agent",
    ];

    let roles = load_fixture().adapter_roles;
    let actual_wires = roles.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(actual_wires, EXPECTED_ROLE_WIRES);

    for wire in roles {
        let role = AdapterRole::from_str(&wire).unwrap();
        assert_eq!(role.as_str(), wire);
        assert_serializes_to_wire(&role, &wire);

        let json = serde_json::to_string(&wire).unwrap();
        assert_eq!(serde_json::from_str::<AdapterRole>(&json).unwrap(), role);
    }
}

#[test]
fn adapter_mode_examples_are_complete_and_stable() {
    const EXPECTED_MODE_WIRES: &[&str] = &[
        "disabled",
        "read_only",
        "import_only",
        "export_only",
        "bidirectional",
        "dry_run",
    ];

    let modes = load_fixture().adapter_modes;
    let actual_wires = modes
        .iter()
        .map(|example| example.wire.as_str())
        .collect::<Vec<_>>();
    assert_eq!(actual_wires, EXPECTED_MODE_WIRES);

    for example in modes {
        let mode = AdapterMode::from_str(&example.wire).unwrap();
        assert_eq!(mode.as_str(), example.wire);
        assert_eq!(mode.allows_core_reads(), example.allows_core_reads);
        assert_eq!(mode.allows_core_writes(), example.allows_core_writes);
        assert_serializes_to_wire(&mode, &example.wire);

        let json = serde_json::to_string(&example.wire).unwrap();
        assert_eq!(serde_json::from_str::<AdapterMode>(&json).unwrap(), mode);
    }
}

#[test]
fn validation_error_examples_are_complete_safe_and_stable() {
    const EXPECTED_ERROR_WIRES: &[&str] = &[
        "empty_path",
        "absolute_path",
        "path_traversal",
        "windows_drive_prefix",
        "windows_separator",
        "null_byte",
        "runtime_path",
        "invalid_percent_encoding",
        "invalid_hash_length",
        "invalid_hash_character",
        "invalid_identifier",
        "invalid_identifier_prefix",
        "invalid_adapter_role",
        "invalid_adapter_mode",
    ];

    let errors = load_fixture().validation_errors;
    let actual_wires = errors
        .iter()
        .map(|example| example.wire.as_str())
        .collect::<Vec<_>>();
    assert_eq!(actual_wires, EXPECTED_ERROR_WIRES);

    for example in errors {
        let json = serde_json::to_string(&example.wire).unwrap();
        let error: ValidationError = serde_json::from_str(&json).unwrap();

        assert_eq!(error.code(), example.wire);
        assert_eq!(error.message(), example.message);
        assert_serializes_to_wire(&error, &example.wire);
    }
}
