use haze_sync_api::{
    dto::{
        common::{ConflictPolicyDto, ConflictResolutionDto, ConflictStatusDto},
        primitives::TimestampDto,
    },
    routes::conflicts::{
        conflict_list_response_from_parts, parse_conflicts_query, parse_resolve_conflict_request,
        ConflictListRequestParts, ConflictRouteSummaryParts, ConflictsRouteError,
        ConflictsRouteErrorKind, ResolveConflictRequestParts,
    },
};
use haze_sync_common::{AdapterId, ConflictId, RevisionId, VaultPath};

fn conflict_id_value(value: &str) -> ConflictId {
    ConflictId::parse(value).unwrap()
}

fn revision_id_value(value: &str) -> RevisionId {
    RevisionId::parse(value).unwrap()
}

fn vault_path(value: &str) -> VaultPath {
    VaultPath::parse(value).unwrap()
}

fn resolve_request_parts(action: &str) -> ResolveConflictRequestParts<'_> {
    ResolveConflictRequestParts {
        conflict_id: "conf_01JTEST",
        resolution: Some(action),
        extra_fields: &[],
    }
}

#[test]
fn get_status_open_parses() {
    let request = parse_conflicts_query(ConflictListRequestParts {
        status: Some("open"),
    })
    .unwrap();

    assert_eq!(request.status, Some(ConflictStatusDto::Open));
    assert_eq!(request.to_dto().status, Some(ConflictStatusDto::Open));
}

#[test]
fn unsupported_status_rejected_safely() {
    let error = parse_conflicts_query(ConflictListRequestParts {
        status: Some("closed"),
    })
    .unwrap_err();

    assert_eq!(error.kind(), ConflictsRouteErrorKind::UnsupportedStatus);
    assert_eq!(error.status_code(), 400);
    assert_safe_error(&error, "closed");
}

#[test]
fn conflict_list_serialization_stable() {
    let response = conflict_list_response_from_parts(vec![ConflictRouteSummaryParts {
        conflict_id: conflict_id_value("conf_01JTEST"),
        original_path: vault_path("Projects/Haze/plan.md"),
        conflict_path: vault_path(
            "_haze_conflicts/open/Projects/Haze/plan.conflict.worktree.2026-07-01-2230.md",
        ),
        current_revision_id: revision_id_value("rev_01JCURRENT"),
        conflict_revision_id: Some(revision_id_value("rev_01JCONFLICT")),
        incoming_revision_id: Some(revision_id_value("rev_01JINCOMING")),
        source_adapter_id: AdapterId::parse("worktree-adapter").unwrap(),
        policy_applied: ConflictPolicyDto::PreserveBoth,
        status: ConflictStatusDto::Open,
        created_at: Some(TimestampDto::from("2026-07-01T22:30:00Z")),
        updated_at: Some(TimestampDto::from("2026-07-01T22:31:00Z")),
    }]);

    let json = serde_json::to_string(&response).unwrap();

    assert_eq!(
        json,
        concat!(
            "{\"conflicts\":[{",
            "\"conflict_id\":\"conf_01JTEST\",",
            "\"original_path\":\"Projects/Haze/plan.md\",",
            "\"conflict_path\":\"_haze_conflicts/open/Projects/Haze/",
            "plan.conflict.worktree.2026-07-01-2230.md\",",
            "\"current_revision_id\":\"rev_01JCURRENT\",",
            "\"conflict_revision_id\":\"rev_01JCONFLICT\",",
            "\"incoming_revision_id\":\"rev_01JINCOMING\",",
            "\"source_adapter_id\":\"worktree-adapter\",",
            "\"policy_applied\":\"preserve_both\",",
            "\"status\":\"open\",",
            "\"created_at\":\"2026-07-01T22:30:00Z\",",
            "\"updated_at\":\"2026-07-01T22:31:00Z\"",
            "}]}"
        )
    );
}

#[test]
fn resolve_accept_current_parses() {
    let request = parse_resolve_conflict_request(resolve_request_parts("accept_current")).unwrap();

    assert_eq!(request.conflict_id.as_str(), "conf_01JTEST");
    assert_eq!(request.resolution, ConflictResolutionDto::AcceptCurrent);
    assert_eq!(
        request.to_dto().resolution,
        ConflictResolutionDto::AcceptCurrent
    );
}

#[test]
fn resolve_accept_conflict_parses() {
    let request = parse_resolve_conflict_request(resolve_request_parts("accept_conflict")).unwrap();

    assert_eq!(request.resolution, ConflictResolutionDto::AcceptConflict);
}

#[test]
fn resolve_keep_both_parses() {
    let request = parse_resolve_conflict_request(resolve_request_parts("keep_both")).unwrap();

    assert_eq!(request.resolution, ConflictResolutionDto::KeepBoth);
}

#[test]
fn resolve_mark_resolved_parses() {
    let request = parse_resolve_conflict_request(resolve_request_parts("mark_resolved")).unwrap();

    assert_eq!(request.resolution, ConflictResolutionDto::MarkResolved);
}

#[test]
fn invalid_conflict_id_rejected_safely() {
    let error = parse_resolve_conflict_request(ResolveConflictRequestParts {
        conflict_id: "bad/path",
        resolution: Some("accept_current"),
        extra_fields: &[],
    })
    .unwrap_err();

    assert_eq!(error.kind(), ConflictsRouteErrorKind::InvalidConflictId);
    assert_eq!(error.status_code(), 400);
    assert_safe_error(&error, "bad/path");
}

#[test]
fn unsupported_action_rejected_safely() {
    let error = parse_resolve_conflict_request(ResolveConflictRequestParts {
        conflict_id: "conf_01JTEST",
        resolution: Some("delete_everything"),
        extra_fields: &[],
    })
    .unwrap_err();

    assert_eq!(error.kind(), ConflictsRouteErrorKind::UnsupportedResolution);
    assert_eq!(error.status_code(), 400);
    assert_safe_error(&error, "delete_everything");
}

#[test]
fn malformed_resolve_payload_rejected_safely() {
    let error = parse_resolve_conflict_request(ResolveConflictRequestParts {
        conflict_id: "conf_01JTEST",
        resolution: Some("accept_current"),
        extra_fields: &["provider_payload", "token"],
    })
    .unwrap_err();

    assert_eq!(error.kind(), ConflictsRouteErrorKind::InvalidResolvePayload);
    assert_eq!(error.status_code(), 400);
    assert_safe_error(&error, "provider_payload");
    assert_safe_error(&error, "token");
}

#[test]
fn placeholder_error_mappings_are_safe() {
    let not_found = ConflictsRouteError::not_found();
    let already_resolved = ConflictsRouteError::already_resolved();

    assert_eq!(not_found.status_code(), 404);
    assert_eq!(already_resolved.status_code(), 409);
    assert_safe_error(&not_found, "/srv/haze-vault/worktree");
    assert_safe_error(&already_resolved, "postgres://secret");
}

fn assert_safe_error(error: &ConflictsRouteError, forbidden_fragment: &str) {
    let json = serde_json::to_string(&error.error_response()).unwrap();

    assert!(!json.contains(forbidden_fragment));
    assert!(!json.contains("stack"));
    assert!(!json.contains("postgres://"));
    assert!(!json.contains("token"));
    assert!(!json.contains("provider_payload"));
}
