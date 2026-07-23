include!("postgres_contract_tests/support.rs");
include!("postgres_contract_tests/migrations.rs");
include!("postgres_contract_tests/cas_idempotency.rs");
include!("postgres_contract_tests/credentials_jobs.rs");
include!("postgres_contract_tests/runtime.rs");
include!("postgres_contract_tests/concurrency.rs");
include!("postgres_contract_tests/validation.rs");
include!("postgres_contract_tests/qa_corrections.rs");
include!("postgres_contract_tests/invalidation_serialization.rs");

#[allow(clippy::too_many_arguments)]
mod latest_qa_blockers {
    use super::*;

    include!("postgres_contract_tests/qa_blockers.rs");

    // The Storage PostgreSQL CI command selects ignored tests only. Keep one
    // ignored wrapper per matrix group so every regression is executed exactly
    // once and remains visible by name in the test output.
    #[test]
    #[ignore = "executed by the explicit Storage PostgreSQL contract-suite command"]
    fn ci_executes_generic_effective_writer_boundary_matrix() {
        generic_effective_writer_is_not_publicly_exported();
    }

    #[test]
    #[ignore = "executed by the explicit Storage PostgreSQL contract-suite command"]
    fn ci_executes_hosted_and_standalone_effective_authority_matrix() {
        hosted_effective_writer_cannot_bypass_standalone_runtime_authority();
    }

    #[test]
    #[ignore = "executed by the explicit Storage PostgreSQL contract-suite command"]
    fn ci_executes_ordered_runtime_report_matrix() {
        ordered_runtime_report_is_fail_closed_idempotent_and_rollback_safe();
    }

    #[test]
    #[ignore = "executed by the explicit Storage PostgreSQL contract-suite command"]
    fn ci_executes_multi_token_cas_namespace_matrix() {
        principal_credential_job_slot_and_evidence_cas_namespaces_are_exact();
    }
}

include!("postgres_contract_tests/suite.rs");
