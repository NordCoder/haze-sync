//! Passive contracts for future administrative Worktree HTTP surfaces.
//!
//! This module provides pure authorization, response-building, and submission
//! shapes only. It does not register routes, access app state, inspect files,
//! submit runtime work, poll completion, retry, wait, or expose private details.

use std::{error::Error, fmt};

use crate::{
    auth::{AdapterPrincipal, AdapterRole},
    contracts::errors::{ErrorResponse, PublicError, PublicErrorCode},
    dto::worktree::{
        WorktreeStatusBuildError, WorktreeStatusPlatformParts, WorktreeStatusResponse,
        WorktreeStatusSafeParts, WorktreeSyncOnceRequest, WorktreeSyncOnceResponse,
        WorktreeSyncOnceSubmissionStatus,
    },
};

/// Recommended future status endpoint. This constant does not register a route.
pub const WORKTREE_STATUS_ROUTE: &str = "/v1/admin/worktree/status";
/// Recommended future manual submission endpoint. This constant does not register a route.
pub const WORKTREE_SYNC_ONCE_ROUTE: &str = "/v1/admin/worktree/sync-once";

/// HTTP status for a missing verified principal in future Server wiring.
pub const HTTP_STATUS_UNAUTHORIZED: u16 = 401;
/// HTTP status for an authenticated non-admin principal.
pub const HTTP_STATUS_FORBIDDEN: u16 = 403;

/// Pure admin-role requirement for Worktree administrative requests.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorktreeAdminAuthRequirement;

impl WorktreeAdminAuthRequirement {
    /// Validate an already verified API role without token lookup or middleware.
    pub fn validate_role(self, role: AdapterRole) -> Result<(), WorktreeRouteError> {
        if role.can_admin() {
            Ok(())
        } else {
            Err(WorktreeRouteError::ForbiddenRole)
        }
    }

    /// Validate an already verified principal without exposing its identity.
    pub fn validate_principal(
        self,
        principal: &AdapterPrincipal,
    ) -> Result<(), WorktreeRouteError> {
        self.validate_role(principal.role())
    }
}

/// Bodyless sync-once request paired with an already verified admin principal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedWorktreeSyncOnceRequest {
    request: WorktreeSyncOnceRequest,
    principal: AdapterPrincipal,
}

impl AuthenticatedWorktreeSyncOnceRequest {
    /// Deterministic empty public request marker.
    #[must_use]
    pub const fn request(&self) -> WorktreeSyncOnceRequest {
        self.request
    }

    /// Already verified administrative principal supplied by Server auth wiring.
    #[must_use]
    pub const fn principal(&self) -> &AdapterPrincipal {
        &self.principal
    }
}

/// Require an already verified admin principal for a sync-once submission.
///
/// The helper performs no token verification and submits no runtime work.
pub fn authorize_worktree_sync_once_request(
    request: WorktreeSyncOnceRequest,
    principal: Option<&AdapterPrincipal>,
) -> Result<AuthenticatedWorktreeSyncOnceRequest, WorktreeRouteError> {
    let principal = principal
        .cloned()
        .ok_or(WorktreeRouteError::MissingAdapterPrincipal)?;
    WorktreeAdminAuthRequirement.validate_principal(&principal)?;

    Ok(AuthenticatedWorktreeSyncOnceRequest { request, principal })
}

/// Build a status response from already-sanitized public values.
#[must_use]
pub const fn worktree_status_response(
    parts: WorktreeStatusSafeParts,
) -> WorktreeStatusResponse {
    WorktreeStatusResponse::from_safe_parts(parts)
}

/// Build a status response while checking a platform-width watcher-hint count.
pub fn try_worktree_status_response(
    parts: WorktreeStatusPlatformParts,
) -> Result<WorktreeStatusResponse, WorktreeStatusBuildError> {
    WorktreeStatusResponse::try_from_platform_parts(parts)
}

/// Build a coarse submission-only response from a sanitized Server outcome.
#[must_use]
pub const fn worktree_sync_once_response(
    status: WorktreeSyncOnceSubmissionStatus,
) -> WorktreeSyncOnceResponse {
    WorktreeSyncOnceResponse::submitted(status)
}

/// Sanitized authorization errors for future Worktree admin handlers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorktreeRouteError {
    /// Server auth wiring did not supply a verified principal.
    MissingAdapterPrincipal,
    /// The verified principal does not have the admin role.
    ForbiddenRole,
}

impl WorktreeRouteError {
    /// Intended HTTP status for future Server mapping.
    #[must_use]
    pub const fn http_status_code(self) -> u16 {
        match self {
            Self::MissingAdapterPrincipal => HTTP_STATUS_UNAUTHORIZED,
            Self::ForbiddenRole => HTTP_STATUS_FORBIDDEN,
        }
    }

    /// Existing stable public error code used without adding Worktree internals.
    #[must_use]
    pub const fn public_code(self) -> PublicErrorCode {
        match self {
            Self::MissingAdapterPrincipal => PublicErrorCode::MissingToken,
            Self::ForbiddenRole => PublicErrorCode::ForbiddenRole,
        }
    }

    /// Convert to a secret-safe public error payload.
    #[must_use]
    pub fn to_public_error(self) -> PublicError {
        PublicError::new(self.public_code(), self.safe_message())
    }

    /// Convert to the shared public error envelope.
    #[must_use]
    pub fn to_error_response(self) -> ErrorResponse {
        ErrorResponse {
            error: self.to_public_error(),
        }
    }

    const fn safe_message(self) -> &'static str {
        match self {
            Self::MissingAdapterPrincipal => "Missing authenticated adapter principal",
            Self::ForbiddenRole => "Adapter role is not allowed to submit a Worktree cycle",
        }
    }
}

impl fmt::Display for WorktreeRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.safe_message())
    }
}

impl Error for WorktreeRouteError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::worktree::{
        WorktreeConfiguredMode, WorktreeHostLifecycle, WorktreeManualAvailability,
        WorktreeReadiness, WorktreeReadinessReason,
    };

    fn principal(role: AdapterRole) -> AdapterPrincipal {
        AdapterPrincipal::new("worktree-contract-fixture", role).unwrap()
    }

    #[test]
    fn route_names_are_documentation_only_and_stable() {
        assert_eq!(WORKTREE_STATUS_ROUTE, "/v1/admin/worktree/status");
        assert_eq!(WORKTREE_SYNC_ONCE_ROUTE, "/v1/admin/worktree/sync-once");
    }

    #[test]
    fn admin_principal_is_accepted_without_runtime_submission() {
        let admin = principal(AdapterRole::Admin);
        let authenticated = authorize_worktree_sync_once_request(
            WorktreeSyncOnceRequest::default(),
            Some(&admin),
        )
        .expect("admin principal should satisfy the passive contract");

        assert_eq!(authenticated.request(), WorktreeSyncOnceRequest::default());
        assert_eq!(authenticated.principal().role(), AdapterRole::Admin);
    }

    #[test]
    fn missing_and_non_admin_principals_map_to_safe_existing_errors() {
        let missing = authorize_worktree_sync_once_request(
            WorktreeSyncOnceRequest::default(),
            None,
        )
        .unwrap_err();
        assert_eq!(missing.http_status_code(), HTTP_STATUS_UNAUTHORIZED);
        assert_eq!(missing.public_code(), PublicErrorCode::MissingToken);

        let adapter = principal(AdapterRole::WorktreeAdapter);
        let forbidden = authorize_worktree_sync_once_request(
            WorktreeSyncOnceRequest::default(),
            Some(&adapter),
        )
        .unwrap_err();
        assert_eq!(forbidden.http_status_code(), HTTP_STATUS_FORBIDDEN);
        assert_eq!(forbidden.public_code(), PublicErrorCode::ForbiddenRole);

        let json = serde_json::to_string(&forbidden.to_error_response()).unwrap();
        assert_eq!(
            json,
            "{\"error\":{\"code\":\"forbidden_role\",\"message\":\"Adapter role is not allowed to submit a Worktree cycle\"}}"
        );
        assert!(!json.contains(adapter.adapter_id()));
        assert!(!json.contains("token"));
    }

    #[test]
    fn status_builder_preserves_supplied_readiness_and_informational_counts() {
        let response = worktree_status_response(WorktreeStatusSafeParts {
            configured_mode: WorktreeConfiguredMode::DryRun,
            host_lifecycle: WorktreeHostLifecycle::Running,
            readiness: WorktreeReadiness::Ready,
            readiness_reason: WorktreeReadinessReason::Running,
            cycles_completed: 4,
            cycles_failed: u64::MAX,
            cycle_in_progress: true,
            pending_watcher_hints: u64::MAX,
            manual_availability: WorktreeManualAvailability::Busy,
        });

        assert!(response.is_ready());
        assert_eq!(response.manual_availability, WorktreeManualAvailability::Busy);
        assert_eq!(response.cycles_failed, u64::MAX);
        assert_eq!(response.pending_watcher_hints, u64::MAX);
    }

    #[test]
    fn accepted_outcome_means_submission_only() {
        let response = worktree_sync_once_response(WorktreeSyncOnceSubmissionStatus::Accepted);
        assert!(response.was_accepted());
        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            "{\"status\":\"accepted\"}"
        );
    }
}
