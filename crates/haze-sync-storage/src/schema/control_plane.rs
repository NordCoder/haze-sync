//! Stable metadata for the Stage 11 operational-control storage extension.
//!
//! This module names the forward migration and durable table families without
//! executing migrations or deciding any maintenance, authorization, runtime, or
//! operational-job policy.

/// Forward-only migration filenames added for operational control.
pub const REQUIRED_BASE_MIGRATION_HEAD: &str = "0011_gdrive_durable_state.sql";
pub const CURRENT_MIGRATION_HEAD: &str = "0012_operational_control_storage.sql";
pub const CONTROL_PLANE_MIGRATIONS: &[&str] = &[CURRENT_MIGRATION_HEAD];

/// Storage tables introduced by the Stage 11 control-plane migration.
pub mod table_names {
    pub const MAINTENANCE_CONTROL: &str = "maintenance_control";
    pub const ADAPTER_INVENTORY_STATE: &str = "adapter_inventory_state";
    pub const ADAPTER_INVENTORY: &str = "adapter_inventory";
    pub const ADAPTER_DESIRED_CONTROLS: &str = "adapter_desired_controls";
    pub const ADAPTER_EFFECTIVE_CONTROLS: &str = "adapter_effective_controls";
    pub const QUIESCENCE_EVIDENCE: &str = "quiescence_evidence";
    pub const QUIESCENCE_EVIDENCE_INVALIDATIONS: &str = "quiescence_evidence_invalidations";
    pub const QUIESCENCE_ADAPTER_SNAPSHOTS: &str = "quiescence_adapter_snapshots";
    pub const QUIESCENCE_RUNTIME_SNAPSHOTS: &str = "quiescence_runtime_snapshots";
    pub const PRINCIPALS: &str = "principals";
    pub const CREDENTIALS: &str = "credentials";
    pub const CREDENTIAL_ISSUANCE_IDEMPOTENCY: &str = "credential_issuance_idempotency";
    pub const OPERATIONAL_JOBS: &str = "operational_jobs";
    pub const OPERATIONAL_JOB_ADAPTER_GENERATIONS: &str =
        "operational_job_adapter_generations";
    pub const OPERATIONAL_EXECUTION_SLOTS: &str = "operational_execution_slots";
    pub const OPERATIONAL_IDEMPOTENCY: &str = "operational_idempotency";
    pub const OPERATIONAL_JOB_EVIDENCE: &str = "operational_job_evidence";
    pub const OPERATIONAL_AUDIT_EVENTS: &str = "operational_audit_events";
    pub const GDRIVE_RUNTIME_AUTHORITIES: &str = "gdrive_runtime_authorities";
    pub const GDRIVE_RUNTIME_REPORTS: &str = "gdrive_runtime_reports";
    pub const GDRIVE_MUTATION_PERMITS: &str = "gdrive_mutation_permits";
    pub const GDRIVE_UNCERTAIN_EFFECTS: &str = "gdrive_uncertain_effects";

    pub const ALL: &[&str] = &[
        MAINTENANCE_CONTROL,
        ADAPTER_INVENTORY_STATE,
        ADAPTER_INVENTORY,
        ADAPTER_DESIRED_CONTROLS,
        ADAPTER_EFFECTIVE_CONTROLS,
        QUIESCENCE_EVIDENCE,
        QUIESCENCE_EVIDENCE_INVALIDATIONS,
        QUIESCENCE_ADAPTER_SNAPSHOTS,
        QUIESCENCE_RUNTIME_SNAPSHOTS,
        PRINCIPALS,
        CREDENTIALS,
        CREDENTIAL_ISSUANCE_IDEMPOTENCY,
        OPERATIONAL_JOBS,
        OPERATIONAL_JOB_ADAPTER_GENERATIONS,
        OPERATIONAL_EXECUTION_SLOTS,
        OPERATIONAL_IDEMPOTENCY,
        OPERATIONAL_JOB_EVIDENCE,
        OPERATIONAL_AUDIT_EVENTS,
        GDRIVE_RUNTIME_AUTHORITIES,
        GDRIVE_RUNTIME_REPORTS,
        GDRIVE_MUTATION_PERMITS,
        GDRIVE_UNCERTAIN_EFFECTS,
    ];

    /// Live control families that ordinary product-data restore must exclude.
    pub const PRODUCT_RESTORE_EXCLUSIONS: &[&str] = ALL;
}

/// Named mutable compare-and-set namespaces owned by Storage primitives.
pub mod cas_namespaces {
    pub const MAINTENANCE_GENERATION: &str = "maintenance_generation";
    pub const ADAPTER_INVENTORY_GENERATION: &str = "adapter_inventory_generation";
    pub const ADAPTER_CONTROL_GENERATION: &str = "adapter_control_generation";
    pub const PRINCIPAL_VERSION: &str = "principal_version";
    pub const CREDENTIAL_SET_GENERATION: &str = "credential_set_generation";
    pub const CREDENTIAL_VERSION: &str = "credential_version";
    pub const JOB_VERSION: &str = "job_version";
    pub const EXECUTOR_FENCE: &str = "executor_fence";
    pub const SLOT_VERSION: &str = "slot_version";
    pub const RUNTIME_LEASE_VERSION: &str = "runtime_lease_version";
    pub const STANDALONE_RUNTIME_EPOCH: &str = "standalone_runtime_epoch";
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIGRATION: &str = include_str!("../../../../migrations/0012_operational_control_storage.sql");

    #[test]
    fn migration_metadata_names_the_exact_forward_file() {
        assert_eq!(REQUIRED_BASE_MIGRATION_HEAD, "0011_gdrive_durable_state.sql");
        assert_eq!(CONTROL_PLANE_MIGRATIONS, &[CURRENT_MIGRATION_HEAD]);
    }

    #[test]
    fn every_control_table_is_created_by_the_registered_migration() {
        for table in table_names::ALL {
            assert!(
                MIGRATION.contains(&format!("create table {table} (")),
                "registered control table {table} is missing from migration"
            );
        }
    }

    #[test]
    fn restore_exclusion_set_is_complete_and_deduplicated() {
        let mut names = table_names::PRODUCT_RESTORE_EXCLUSIONS.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), table_names::ALL.len());
    }

    #[test]
    fn migration_contains_legacy_and_immutability_guards() {
        for fragment in [
            "legacy_sha256_v0",
            "credentials_legacy_sha256_lookup_uq",
            "operational_audit_events_append_only",
            "operational_jobs_terminal_immutable",
            "gdrive_runtime_reports_immutable",
        ] {
            assert!(MIGRATION.contains(fragment), "missing migration guard {fragment}");
        }
    }
}
