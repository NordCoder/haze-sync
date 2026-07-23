use super::*;

const MIGRATION: &str =
    include_str!("../../../../../migrations/0012_operational_control_storage.sql");
const UNSAFE_FRAGMENTS: &[&str] = &[
    "postgres://",
    "password",
    "secret-token-value",
    "/srv/",
    "select * from",
    "stack backtrace",
    "raw provider payload",
];

#[test]
fn secret_wrappers_validate_and_redact() {
    let digest = SecretDigest::parse("A".repeat(64)).unwrap();
    assert_eq!(digest.as_str(), "a".repeat(64));
    assert_eq!(format!("{digest:?}"), "SecretDigest([REDACTED])");
    assert_eq!(digest.to_string(), "[REDACTED]");
    assert_eq!(
        SecretDigest::parse("short"),
        Err(ControlPlaneRepositoryError::InvalidDigest)
    );

    let verifier = VerifierMaterial::parse("argon2id-private-verifier").unwrap();
    assert_eq!(format!("{verifier:?}"), "VerifierMaterial([REDACTED])");
    assert_eq!(verifier.to_string(), "[REDACTED]");
    assert_eq!(
        VerifierMaterial::parse("line\nbreak"),
        Err(ControlPlaneRepositoryError::InvalidSecretMaterial)
    );
}

#[test]
fn safe_errors_never_echo_operational_details() {
    let errors = [
        ControlPlaneRepositoryError::InvalidIdentifier,
        ControlPlaneRepositoryError::InvalidDigest,
        ControlPlaneRepositoryError::InvalidSecretMaterial,
        ControlPlaneRepositoryError::InvalidGeneration,
        ControlPlaneRepositoryError::VersionOverflow,
        ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::MAINTENANCE_GENERATION,
        },
        ControlPlaneRepositoryError::MissingRecord,
        ControlPlaneRepositoryError::IdempotencyConflict,
        ControlPlaneRepositoryError::OperationInProgress,
        ControlPlaneRepositoryError::RuntimeLeaseConflict,
        ControlPlaneRepositoryError::StaleRuntimeEpoch,
        ControlPlaneRepositoryError::StaleRuntimeReport,
        ControlPlaneRepositoryError::RuntimeReportConflict,
        ControlPlaneRepositoryError::LegacyVerifierCreationForbidden,
        ControlPlaneRepositoryError::ImmutableRecord,
        ControlPlaneRepositoryError::DatabaseOperationFailed,
    ];
    for error in errors {
        let display = error.to_string();
        for text in [error.code(), error.message(), display.as_str()] {
            let lowered = text.to_ascii_lowercase();
            for fragment in UNSAFE_FRAGMENTS {
                assert!(!lowered.contains(fragment), "unsafe error text: {text}");
            }
        }
        assert!(std::error::Error::source(&error).is_none());
    }
}

#[test]
fn versions_advance_only_inside_supported_range() {
    assert_eq!(advance(0), Ok(1));
    assert_eq!(advance(42), Ok(43));
    assert_eq!(
        advance(-1),
        Err(ControlPlaneRepositoryError::InvalidGeneration)
    );
    assert_eq!(
        advance(i64::MAX),
        Err(ControlPlaneRepositoryError::VersionOverflow)
    );
}

#[test]
fn migration_contains_complete_safety_boundaries() {
    for fragment in [
        "create table maintenance_control",
        "create table quiescence_evidence",
        "create table principals",
        "create table credentials",
        "create table operational_jobs",
        "create table operational_audit_events",
        "create table gdrive_runtime_authorities",
        "create table gdrive_runtime_reports",
        "create table gdrive_mutation_permits",
        "create table gdrive_uncertain_effects",
        "legacy_sha256_v0",
        "credential_issuance_idempotency_immutable",
        "operational_jobs_terminal_immutable",
    ] {
        assert!(MIGRATION.contains(fragment), "missing SQL boundary {fragment}");
    }
    for forbidden in [
        "plaintext_credential",
        "raw_idempotency_key",
        "lease_token text",
        "confirmation_value",
        "oauth_access_token",
        "on delete cascade",
    ] {
        assert!(
            !MIGRATION.to_ascii_lowercase().contains(forbidden),
            "migration contains forbidden persistence surface {forbidden}"
        );
    }
}

#[test]
fn structured_metadata_validation_and_debug_are_secret_safe() {
    assert!(validate_safe_json(&serde_json::json!({
        "count": 3,
        "status": "passed",
        "vault_path": "Notes/safe.md",
        "plaintext_available": false
    }))
    .is_ok());
    for unsafe_value in [
        serde_json::json!({"plaintext_available": true}),
        serde_json::json!({"access_token": "opaque"}),
        serde_json::json!({"detail": "postgres://operator:password@db/control"}),
        serde_json::json!({"detail": "Bearer credential"}),
        serde_json::json!({"detail": "hs1.credential.secret"}),
    ] {
        assert_eq!(
            validate_safe_json(&unsafe_value),
            Err(ControlPlaneRepositoryError::InvalidSecretMaterial)
        );
    }

    let input = UncertainEffectInput {
        effect_id: "effect-a".into(),
        adapter_id: "adapter-a".into(),
        runtime_instance_id: "runtime-a".into(),
        standalone_runtime_epoch: 1,
        expected_runtime_lease_version: 1,
        lease_proof_digest: SecretDigest::parse("b".repeat(64)).unwrap(),
        permit_id: None,
        step_id: "step-a".into(),
        effect_identity_digest: SecretDigest::parse("a".repeat(64)).unwrap(),
        safe_metadata: serde_json::json!({"private-looking-value":"must-not-render"}),
    };
    let debug = format!("{input:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("must-not-render"));
}
