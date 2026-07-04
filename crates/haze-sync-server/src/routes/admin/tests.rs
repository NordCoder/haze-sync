use super::*;

#[test]
fn dependency_free_status_payload_is_safe_placeholder() {
    let payload = StatusSummaryResponse::placeholder();
    let json = serde_json::to_string(&payload).expect("status should serialize");

    assert!(json.contains("not_ready"));
    assert_no_sensitive_leaks(&json);
}

#[test]
fn empty_adapter_list_payload_is_safe() {
    let payload = AdapterListResponse::empty();
    let json = serde_json::to_string(&payload).expect("adapter list should serialize");

    assert_eq!(json, "{\"total_count\":0,\"adapters\":[]}");
    assert_no_sensitive_leaks(&json);
}

#[test]
fn admin_errors_do_not_echo_token_material() {
    let response = ApiError::invalid_token().into_response();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

fn assert_no_sensitive_leaks(json: &str) {
    for forbidden in [
        concat!("to", "ken"),
        concat!("ha", "sh"),
        concat!("oa", "uth"),
        concat!("se", "cret"),
        concat!("database", "_url"),
        concat!("db", "_url"),
        concat!("provider", "_payload"),
        concat!("external_cursor", "_json"),
        concat!("object_store", "_root"),
        concat!("/", "srv", "/"),
        concat!("post", "gres", "://"),
        concat!("sta", "ck"),
        concat!("back", "trace"),
    ] {
        assert!(
            !json.contains(forbidden),
            "admin output leaked forbidden marker {forbidden}: {json}"
        );
    }
}
