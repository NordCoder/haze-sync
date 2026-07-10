use super::*;
use chrono::TimeZone;
use serde_json::json;

fn response_snapshot() -> Value {
    json!({
        "status_code": 201,
        "headers": {
            "content-type": "application/json",
            "x-revision-id": "rev_124"
        },
        "body": {
            "status": "accepted",
            "path": "Projects/Haze/plan.md"
        }
    })
}

fn row(request_hash: &Sha256) -> IdempotencyRecordRow {
    IdempotencyRecordRow {
        adapter_id: "iphone-anna".to_owned(),
        idempotency_key: "iphone:op-1".to_owned(),
        request_hash: request_hash.to_string(),
        response_json: response_snapshot(),
        created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
    }
}

#[test]
fn compare_request_fingerprint_detects_same_request() {
    let hash = Sha256::parse(&"a".repeat(64)).unwrap();
    assert_eq!(
        compare_request_fingerprint(&row(&hash), &hash).unwrap(),
        IdempotencyRequestComparison::SameRequest
    );
}

#[test]
fn compare_request_fingerprint_detects_different_request() {
    let stored = Sha256::parse(&"a".repeat(64)).unwrap();
    let incoming = Sha256::parse(&"b".repeat(64)).unwrap();
    assert_eq!(
        compare_request_fingerprint(&row(&stored), &incoming).unwrap(),
        IdempotencyRequestComparison::DifferentRequest
    );
}

#[test]
fn existing_record_outcomes_match_core_replay_categories() {
    let stored = Sha256::parse(&"a".repeat(64)).unwrap();
    let incoming = Sha256::parse(&"b".repeat(64)).unwrap();
    let record = row(&stored);

    assert_eq!(
        outcome_from_existing_record(record.clone(), &stored).unwrap(),
        IdempotencyRepositoryOutcome::ReplaySameRequest {
            record: record.clone()
        }
    );
    assert_eq!(
        outcome_from_existing_record(record.clone(), &incoming).unwrap(),
        IdempotencyRepositoryOutcome::ConflictDifferentRequest { record }
    );
}

#[test]
fn core_compatible_response_snapshot_is_preserved_verbatim() {
    let snapshot = response_snapshot();
    let input = IdempotencyRecordInput::new(
        AdapterId::parse("iphone-anna").unwrap(),
        "iphone:op-1",
        Sha256::parse(&"a".repeat(64)).unwrap(),
        snapshot.clone(),
    )
    .unwrap();

    assert_eq!(input.response_json(), &snapshot);
    assert_eq!(input.response_json()["status_code"], 201);
    assert_eq!(
        input.response_json()["headers"]["content-type"],
        "application/json"
    );
    assert!(input.response_json()["headers"]
        .get("authorization")
        .is_none());
    assert!(input.response_json()["headers"].get("set-cookie").is_none());
}

#[test]
fn invalid_key_is_rejected() {
    assert_eq!(
        IdempotencyRecordInput::new(
            AdapterId::parse("iphone-anna").unwrap(),
            "contains space",
            Sha256::parse(&"a".repeat(64)).unwrap(),
            response_snapshot(),
        )
        .unwrap_err(),
        IdempotencyRepositoryError::InvalidKey
    );
}

#[test]
fn invalid_stored_hash_is_rejected_without_exposing_row_values() {
    let mut invalid = row(&Sha256::parse(&"a".repeat(64)).unwrap());
    invalid.request_hash = "not-a-hash".to_owned();

    let error = compare_request_fingerprint(&invalid, &Sha256::parse(&"b".repeat(64)).unwrap())
        .unwrap_err();
    let displayed = error.to_string();

    assert_eq!(error, IdempotencyRepositoryError::InvalidStoredRequestHash);
    assert!(!displayed.contains("not-a-hash"));
    assert!(!displayed.contains("iphone:op-1"));
}
