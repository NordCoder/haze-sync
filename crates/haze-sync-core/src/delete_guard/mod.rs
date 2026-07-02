//! Deterministic mass-delete guard primitives.
//!
//! The guard is pure and side-effect free. It does not call providers, mutate
//! storage, create tombstones, or perform filesystem cleanup. Callers provide the
//! scoped run context and any manual unlock token/flag that was granted outside
//! this module.

use haze_sync_common::AdapterId;
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

const MAX_RUN_ID_LEN: usize = 128;

/// Conservative default maximum number of delete candidates in one adapter run.
pub const DEFAULT_MAX_DELETES_PER_RUN: u64 = 20;
/// Conservative default delete-ratio numerator.
pub const DEFAULT_MAX_DELETE_RATIO_NUMERATOR: u64 = 5;
/// Conservative default delete-ratio denominator.
pub const DEFAULT_MAX_DELETE_RATIO_DENOMINATOR: u64 = 100;

/// Rational ratio limit used instead of floats for deterministic comparisons and
/// serialized output.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeleteRatioLimit {
    numerator: u64,
    denominator: u64,
}

impl DeleteRatioLimit {
    /// Build a ratio limit. `0/positive` is allowed and blocks any non-zero
    /// delete count; a zero denominator is rejected.
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, DeleteGuardError> {
        if denominator == 0 {
            return Err(DeleteGuardError::InvalidDeleteRatioLimit);
        }

        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// Build a percentage ratio, for example `5` means `5%`.
    pub fn percent(percent: u64) -> Result<Self, DeleteGuardError> {
        Self::new(percent, 100)
    }

    /// Numerator used in serialized deterministic comparisons.
    #[must_use]
    pub const fn numerator(self) -> u64 {
        self.numerator
    }

    /// Denominator used in serialized deterministic comparisons.
    #[must_use]
    pub const fn denominator(self) -> u64 {
        self.denominator
    }

    /// Returns true when `delete_count / total_count` exceeds this limit.
    #[must_use]
    pub fn is_exceeded_by(self, delete_count: u64, total_count: u64) -> bool {
        if delete_count == 0 {
            return false;
        }

        if total_count == 0 {
            return true;
        }

        let delete_count = u128::from(delete_count);
        let total_count = u128::from(total_count);
        let numerator = u128::from(self.numerator);
        let denominator = u128::from(self.denominator);

        delete_count.saturating_mul(denominator) > total_count.saturating_mul(numerator)
    }
}

impl Default for DeleteRatioLimit {
    fn default() -> Self {
        Self {
            numerator: DEFAULT_MAX_DELETE_RATIO_NUMERATOR,
            denominator: DEFAULT_MAX_DELETE_RATIO_DENOMINATOR,
        }
    }
}

/// Delete-guard policy. Defaults are intentionally conservative.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeleteGuardPolicy {
    pub max_deletes_per_run: u64,
    pub max_delete_ratio_per_run: DeleteRatioLimit,
    pub require_manual_unlock_for_mass_delete: bool,
}

impl DeleteGuardPolicy {
    /// Build a policy with explicit thresholds.
    #[must_use]
    pub fn new(
        max_deletes_per_run: u64,
        max_delete_ratio_per_run: DeleteRatioLimit,
        require_manual_unlock_for_mass_delete: bool,
    ) -> Self {
        Self {
            max_deletes_per_run,
            max_delete_ratio_per_run,
            require_manual_unlock_for_mass_delete,
        }
    }
}

impl Default for DeleteGuardPolicy {
    fn default() -> Self {
        Self {
            max_deletes_per_run: DEFAULT_MAX_DELETES_PER_RUN,
            max_delete_ratio_per_run: DeleteRatioLimit::default(),
            require_manual_unlock_for_mass_delete: true,
        }
    }
}

/// Explicit adapter/run scope for a delete-guard evaluation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeleteRunScope {
    pub adapter_id: AdapterId,
    pub run_id: String,
}

impl DeleteRunScope {
    /// Validate and build a run scope.
    pub fn new(
        adapter_id: AdapterId,
        run_id: impl Into<String>,
    ) -> Result<Self, DeleteGuardError> {
        let run_id = run_id.into();
        validate_run_id(&run_id)?;
        Ok(Self { adapter_id, run_id })
    }
}

/// Manual unlock flag. An unlock is valid only for the exact adapter/run scope it
/// names and only for the threshold categories it explicitly covers.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum ManualDeleteUnlock {
    NotGranted,
    Scoped {
        scope: DeleteRunScope,
        allow_too_many_deletes: bool,
        allow_delete_ratio: bool,
    },
}

impl ManualDeleteUnlock {
    /// Create a scoped unlock covering both delete-count and delete-ratio blocks.
    #[must_use]
    pub fn scoped_for_all(scope: DeleteRunScope) -> Self {
        Self::Scoped {
            scope,
            allow_too_many_deletes: true,
            allow_delete_ratio: true,
        }
    }

    fn covers_too_many_deletes(&self, scope: &DeleteRunScope) -> bool {
        matches!(
            self,
            Self::Scoped {
                scope: unlock_scope,
                allow_too_many_deletes: true,
                ..
            } if unlock_scope == scope
        )
    }

    fn covers_delete_ratio(&self, scope: &DeleteRunScope) -> bool {
        matches!(
            self,
            Self::Scoped {
                scope: unlock_scope,
                allow_delete_ratio: true,
                ..
            } if unlock_scope == scope
        )
    }
}

impl Default for ManualDeleteUnlock {
    fn default() -> Self {
        Self::NotGranted
    }
}

/// Pure input to one delete-guard evaluation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeleteGuardInput {
    pub scope: DeleteRunScope,
    pub proposed_delete_count: u64,
    pub total_files_before_run: u64,
    pub manual_unlock: ManualDeleteUnlock,
}

impl DeleteGuardInput {
    /// Build an input with no manual unlock.
    #[must_use]
    pub fn without_manual_unlock(
        scope: DeleteRunScope,
        proposed_delete_count: u64,
        total_files_before_run: u64,
    ) -> Self {
        Self {
            scope,
            proposed_delete_count,
            total_files_before_run,
            manual_unlock: ManualDeleteUnlock::NotGranted,
        }
    }

    /// Attach a manual unlock flag to the input.
    #[must_use]
    pub fn with_manual_unlock(mut self, manual_unlock: ManualDeleteUnlock) -> Self {
        self.manual_unlock = manual_unlock;
        self
    }
}

/// Safe, deterministic guard outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum DeleteGuardDecision {
    Allowed,
    BlockedTooManyDeletes {
        proposed_delete_count: u64,
        max_deletes_per_run: u64,
    },
    BlockedDeleteRatio {
        proposed_delete_count: u64,
        total_files_before_run: u64,
        max_delete_ratio_per_run: DeleteRatioLimit,
    },
    BlockedRequiresManualUnlock {
        proposed_delete_count: u64,
        total_files_before_run: u64,
        reason: DeleteGuardBlockReason,
    },
}

/// Threshold category that caused a manual-unlock block.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteGuardBlockReason {
    TooManyDeletes,
    DeleteRatio,
}

/// Pure delete-guard evaluator.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeleteGuard {
    policy: DeleteGuardPolicy,
}

impl DeleteGuard {
    /// Build a guard from a policy.
    #[must_use]
    pub fn new(policy: DeleteGuardPolicy) -> Self {
        Self { policy }
    }

    /// Return the policy used by this guard.
    #[must_use]
    pub fn policy(self) -> DeleteGuardPolicy {
        self.policy
    }

    /// Evaluate delete candidates without mutating storage or calling providers.
    #[must_use]
    pub fn evaluate(&self, input: &DeleteGuardInput) -> DeleteGuardDecision {
        if input.proposed_delete_count > self.policy.max_deletes_per_run
            && !input.manual_unlock.covers_too_many_deletes(&input.scope)
        {
            if self.policy.require_manual_unlock_for_mass_delete {
                return DeleteGuardDecision::BlockedRequiresManualUnlock {
                    proposed_delete_count: input.proposed_delete_count,
                    total_files_before_run: input.total_files_before_run,
                    reason: DeleteGuardBlockReason::TooManyDeletes,
                };
            }

            return DeleteGuardDecision::BlockedTooManyDeletes {
                proposed_delete_count: input.proposed_delete_count,
                max_deletes_per_run: self.policy.max_deletes_per_run,
            };
        }

        if self
            .policy
            .max_delete_ratio_per_run
            .is_exceeded_by(input.proposed_delete_count, input.total_files_before_run)
            && !input.manual_unlock.covers_delete_ratio(&input.scope)
        {
            if self.policy.require_manual_unlock_for_mass_delete {
                return DeleteGuardDecision::BlockedRequiresManualUnlock {
                    proposed_delete_count: input.proposed_delete_count,
                    total_files_before_run: input.total_files_before_run,
                    reason: DeleteGuardBlockReason::DeleteRatio,
                };
            }

            return DeleteGuardDecision::BlockedDeleteRatio {
                proposed_delete_count: input.proposed_delete_count,
                total_files_before_run: input.total_files_before_run,
                max_delete_ratio_per_run: self.policy.max_delete_ratio_per_run,
            };
        }

        DeleteGuardDecision::Allowed
    }
}

impl Default for DeleteGuard {
    fn default() -> Self {
        Self::new(DeleteGuardPolicy::default())
    }
}

/// Safe guard configuration/input errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeleteGuardError {
    InvalidDeleteRatioLimit,
    InvalidRunId,
}

impl DeleteGuardError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidDeleteRatioLimit => "invalid_delete_ratio_limit",
            Self::InvalidRunId => "invalid_delete_run_id",
        }
    }
}

impl fmt::Display for DeleteGuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidDeleteRatioLimit => "delete ratio limit is invalid",
            Self::InvalidRunId => "delete run id is invalid",
        })
    }
}

impl Error for DeleteGuardError {}

fn validate_run_id(run_id: &str) -> Result<(), DeleteGuardError> {
    if run_id.is_empty()
        || run_id.len() > MAX_RUN_ID_LEN
        || run_id.as_bytes().contains(&0)
        || !run_id
            .chars()
            .all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
            })
    {
        return Err(DeleteGuardError::InvalidRunId);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter_id() -> AdapterId {
        AdapterId::parse("gdrive-adapter").unwrap()
    }

    fn scope(run_id: &str) -> DeleteRunScope {
        DeleteRunScope::new(adapter_id(), run_id).unwrap()
    }

    #[test]
    fn default_policy_is_conservative() {
        let policy = DeleteGuardPolicy::default();
        assert_eq!(policy.max_deletes_per_run, 20);
        assert_eq!(policy.max_delete_ratio_per_run, DeleteRatioLimit::default());
        assert!(policy.require_manual_unlock_for_mass_delete);
    }

    #[test]
    fn delete_guard_blocks_too_many_deletes() {
        let guard = DeleteGuard::new(DeleteGuardPolicy::new(
            2,
            DeleteRatioLimit::percent(100).unwrap(),
            false,
        ));
        let input = DeleteGuardInput::without_manual_unlock(scope("scan-1"), 3, 100);

        assert_eq!(
            guard.evaluate(&input),
            DeleteGuardDecision::BlockedTooManyDeletes {
                proposed_delete_count: 3,
                max_deletes_per_run: 2,
            }
        );
    }

    #[test]
    fn delete_guard_blocks_unsafe_ratio() {
        let guard = DeleteGuard::new(DeleteGuardPolicy::new(
            20,
            DeleteRatioLimit::percent(5).unwrap(),
            false,
        ));
        let input = DeleteGuardInput::without_manual_unlock(scope("scan-2"), 6, 100);

        assert_eq!(
            guard.evaluate(&input),
            DeleteGuardDecision::BlockedDeleteRatio {
                proposed_delete_count: 6,
                total_files_before_run: 100,
                max_delete_ratio_per_run: DeleteRatioLimit::percent(5).unwrap(),
            }
        );
    }

    #[test]
    fn manual_unlock_allows_only_explicit_scope() {
        let guard = DeleteGuard::default();
        let actual_scope = scope("run-safe");
        let wrong_scope = scope("run-other");
        let unsafe_input = DeleteGuardInput::without_manual_unlock(actual_scope.clone(), 25, 100)
            .with_manual_unlock(ManualDeleteUnlock::scoped_for_all(wrong_scope));

        assert_eq!(
            guard.evaluate(&unsafe_input),
            DeleteGuardDecision::BlockedRequiresManualUnlock {
                proposed_delete_count: 25,
                total_files_before_run: 100,
                reason: DeleteGuardBlockReason::TooManyDeletes,
            }
        );

        let allowed_input = DeleteGuardInput::without_manual_unlock(actual_scope.clone(), 25, 100)
            .with_manual_unlock(ManualDeleteUnlock::scoped_for_all(actual_scope));
        assert_eq!(guard.evaluate(&allowed_input), DeleteGuardDecision::Allowed);
    }

    #[test]
    fn invalid_run_id_is_rejected() {
        assert_eq!(
            DeleteRunScope::new(adapter_id(), "bad/run").unwrap_err(),
            DeleteGuardError::InvalidRunId
        );
    }
}
