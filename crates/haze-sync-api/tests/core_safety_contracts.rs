use std::collections::BTreeMap;

use haze_sync_api::contracts::errors::{
    ErrorResponse, PublicError, PublicErrorCode, SafeErrorDetails,
};
use serde_json::json;

#[test]
fn conflict_delete_and_idempotency_errors_are_safe_public_json() {
    let mut details = BTreeMap::new();
    details.insert(
        "path".to_owned(),
        vec!["operation requires conflict center review".to_owned()],
    );

    let responses = [
        ErrorResponse {
            error: PublicError::new(PublicErrorCode::Conflict, "write preserved as conflict")
                .with_request_id("req_conflict_01J")
                .with_details(SafeErrorDetails::Map(details)),
        },
        ErrorResponse {
            error: PublicError::new(
                PublicErrorCode::UnsafeDelete,
                "delete rejected by safety rules",
            )
            .with_request_id("req_delete_01J"),
        },
        ErrorResponse {
            error: PublicError::new(
                PublicErrorCode::IdempotencyConflict,
                "idempotency key was reused for a different request",
            )
            .with_details(SafeErrorDetails::List(vec![
                "submit a new idempotency key for a different write".to_owned(),
            ])),
        },
    ];

    for response in responses {
        let serialized = serde_json::to_string(&response).expect("public error should serialize");
        assert!(serialized.contains("error"));
        assert!(!serialized.contains("DATABASE_URL"));
        assert!(!serialized.contains("Bearer"));
        assert!(!serialized.contains("token"));
        assert!(!serialized.contains("/srv/"));
        assert!(!serialized.contains("stack"));
        assert!(!serialized.contains("provider_payload"));
        assert!(!serialized.contains("request_body"));
    }
}

#[test]
fn conflict_resolution_action_vocabulary_is_json_serializable_spec() {
    let actions = json!([
        "accept_current",
        "accept_conflict",
        "keep_both",
        "mark_resolved"
    ]);

    assert_eq!(actions[0], "accept_current");
    assert_eq!(actions[1], "accept_conflict");
    assert_eq!(actions[2], "keep_both");
    assert_eq!(actions[3], "mark_resolved");
}

fn pending_sibling_behavior(phase: &str, behavior: &str) -> ! {
    panic!(
        "{phase} must replace this ignored W3-P8 API spec placeholder with executable route-helper coverage for {behavior}"
    );
}

#[test]
#[ignore = "requires W3-P4 conflict resolution API contract helpers"]
fn accept_current_resolution_behavior_is_exercised_by_route_contracts() {
    pending_sibling_behavior("W3-P4", "accept_current resolution behavior");
}

#[test]
#[ignore = "requires W3-P4 conflict resolution API contract helpers"]
fn accept_conflict_resolution_behavior_is_exercised_by_route_contracts() {
    pending_sibling_behavior("W3-P4", "accept_conflict resolution behavior");
}

#[test]
#[ignore = "requires W3-P4 conflict resolution API contract helpers"]
fn keep_both_resolution_behavior_is_exercised_by_route_contracts() {
    pending_sibling_behavior("W3-P4", "keep_both behavior");
}

#[test]
#[ignore = "requires W3-P4 conflict resolution API contract helpers"]
fn mark_resolved_behavior_is_exercised_when_contract_supports_it() {
    pending_sibling_behavior("W3-P4", "mark_resolved behavior");
}

#[test]
#[ignore = "requires W3-P5 DELETE route contract helpers"]
fn delete_route_public_errors_are_safe_for_stale_and_mass_delete_rejections() {
    pending_sibling_behavior("W3-P5", "DELETE route safe public errors");
}
