pub async fn lock_maintenance_control(
    transaction: &mut Transaction<'_, Postgres>,
) -> ControlPlaneResult<MaintenanceControlRow> {
    let query = format!(
        "select {MAINTENANCE_COLUMNS} from maintenance_control where singleton_id = 1 for update"
    );
    let row = sqlx::query(&query)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    maintenance_from_row(&row)
}

/// Applies an exact maintenance-generation CAS. Transition legality remains a Server decision.
pub async fn cas_update_maintenance_control(
    transaction: &mut Transaction<'_, Postgres>,
    expected_generation: i64,
    update: &MaintenanceControlUpdate,
) -> ControlPlaneResult<MaintenanceControlRow> {
    validate_non_negative(expected_generation)?;
    validate_non_negative(update.quiescence_evidence_version)?;
    validate_identifier(&update.state)?;
    let next_generation = advance(expected_generation)?;
    let query = format!(
        "update maintenance_control set maintenance_generation = $2, state = $3, \
         transition_operation_id = $4, transition_requested_by = $5, \
         transition_requested_at = $6, state_entered_at = $7, admission_fence_closed = $8, \
         admission_fence_closed_at = $9, quiescence_evidence_version = $10, \
         quiescence_evidence_id = $11, active_maintenance_job_id = $12, \
         safe_error_category = $13, updated_at = now() \
         where singleton_id = 1 and maintenance_generation = $1 returning {MAINTENANCE_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(expected_generation)
        .bind(next_generation)
        .bind(&update.state)
        .bind(&update.transition_operation_id)
        .bind(&update.transition_requested_by)
        .bind(update.transition_requested_at)
        .bind(update.state_entered_at)
        .bind(update.admission_fence_closed)
        .bind(update.admission_fence_closed_at)
        .bind(update.quiescence_evidence_version)
        .bind(&update.quiescence_evidence_id)
        .bind(&update.active_maintenance_job_id)
        .bind(&update.safe_error_category)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::MAINTENANCE_GENERATION))?;
    maintenance_from_row(&row)
}
