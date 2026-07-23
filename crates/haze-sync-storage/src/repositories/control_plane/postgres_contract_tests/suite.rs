#[test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL and full control-plane PostgreSQL suite"]
fn complete_control_plane_postgres_contract_suite() {
    fresh_schema_applies_complete_control_migration();
    sequential_migration_preserves_existing_storage_and_maps_legacy_credentials();
    malformed_or_ambiguous_legacy_input_rolls_back_the_whole_forward_migration();
    explicit_transaction_rollback_removes_control_ddl_without_touching_base_data();
    migration_replay_fails_deterministically_without_changing_committed_rows();
    stale_cas_and_caller_rollback_leave_no_partial_control_change();
    idempotency_and_immutable_terminal_records_are_enforced();
    credential_slot_audit_and_savepoint_composition_are_transactional();
    runtime_epoch_report_permit_and_uncertain_effect_primitives_are_fail_closed();
    expired_runtime_takeover_converts_open_permits_to_reconciliation_evidence();
    concurrent_maintenance_cas_allows_exactly_one_commit();
}
