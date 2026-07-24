use std::{collections::VecDeque, future::Future, pin::Pin, sync::{Arc, Mutex}};

use haze_sync_storage::control_plane::{
    QuiescenceAdapterSnapshotInput, QuiescenceRuntimeSnapshotInput,
};
use serde_json::Value;

use super::ControlPlaneError;

pub(crate) type ControlFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ControlPlaneError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AdapterControlTarget {
    pub(crate) adapter_id: String,
    pub(crate) adapter_kind: String,
    pub(crate) control_authority: String,
    pub(crate) desired_enabled: bool,
    pub(crate) desired_mode: String,
    pub(crate) adapter_control_generation: i64,
    pub(crate) maintenance_generation: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct QuiescenceCollection {
    pub(crate) adapter_snapshots: Vec<QuiescenceAdapterSnapshotInput>,
    pub(crate) runtime_snapshots: Vec<QuiescenceRuntimeSnapshotInput>,
    pub(crate) worktree_external_writer_evidence: Value,
}

pub(crate) trait AdapterControlEvidence: Send + Sync {
    fn collect_quiescence<'a>(
        &'a self,
        targets: &'a [AdapterControlTarget],
    ) -> ControlFuture<'a, QuiescenceCollection>;

    fn await_resume<'a>(
        &'a self,
        targets: &'a [AdapterControlTarget],
    ) -> ControlFuture<'a, ()>;
}

#[derive(Clone, Debug, Default)]
pub(crate) struct EmptyInventoryAdapterControl;

impl AdapterControlEvidence for EmptyInventoryAdapterControl {
    fn collect_quiescence<'a>(
        &'a self,
        targets: &'a [AdapterControlTarget],
    ) -> ControlFuture<'a, QuiescenceCollection> {
        Box::pin(async move {
            if !targets.is_empty() {
                return Err(ControlPlaneError::AdapterControlUnavailable);
            }
            Ok(QuiescenceCollection {
                adapter_snapshots: Vec::new(),
                runtime_snapshots: Vec::new(),
                worktree_external_writer_evidence: serde_json::json!({
                    "boundary": "stage11_fake_adapter_control",
                    "status": "no_configured_adapter_instances"
                }),
            })
        })
    }

    fn await_resume<'a>(
        &'a self,
        targets: &'a [AdapterControlTarget],
    ) -> ControlFuture<'a, ()> {
        Box::pin(async move {
            if targets.is_empty() {
                Ok(())
            } else {
                Err(ControlPlaneError::AdapterControlUnavailable)
            }
        })
    }
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) enum ScriptedAdapterOutcome {
    Quiesced(QuiescenceCollection),
    Timeout,
    Failed,
    Resumed,
}

#[cfg(test)]
#[derive(Clone, Debug, Default)]
pub(crate) struct DeterministicFakeAdapterControl {
    outcomes: Arc<Mutex<VecDeque<ScriptedAdapterOutcome>>>,
}

#[cfg(test)]
impl DeterministicFakeAdapterControl {
    #[cfg(test)]
    pub(crate) fn scripted(outcomes: impl IntoIterator<Item = ScriptedAdapterOutcome>) -> Self {
        Self {
            outcomes: Arc::new(Mutex::new(outcomes.into_iter().collect())),
        }
    }

    fn next(&self) -> Result<ScriptedAdapterOutcome, ControlPlaneError> {
        self.outcomes
            .lock()
            .map_err(|_| ControlPlaneError::Internal)?
            .pop_front()
            .ok_or(ControlPlaneError::AdapterControlUnavailable)
    }
}

#[cfg(test)]
impl AdapterControlEvidence for DeterministicFakeAdapterControl {
    fn collect_quiescence<'a>(
        &'a self,
        _targets: &'a [AdapterControlTarget],
    ) -> ControlFuture<'a, QuiescenceCollection> {
        Box::pin(async move {
            match self.next()? {
                ScriptedAdapterOutcome::Quiesced(evidence) => Ok(evidence),
                ScriptedAdapterOutcome::Timeout => Err(ControlPlaneError::QuiescenceTimeout),
                ScriptedAdapterOutcome::Failed | ScriptedAdapterOutcome::Resumed => {
                    Err(ControlPlaneError::AdapterControlUnavailable)
                }
            }
        })
    }

    fn await_resume<'a>(
        &'a self,
        _targets: &'a [AdapterControlTarget],
    ) -> ControlFuture<'a, ()> {
        Box::pin(async move {
            match self.next()? {
                ScriptedAdapterOutcome::Resumed => Ok(()),
                ScriptedAdapterOutcome::Timeout => Err(ControlPlaneError::QuiescenceTimeout),
                ScriptedAdapterOutcome::Failed | ScriptedAdapterOutcome::Quiesced(_) => {
                    Err(ControlPlaneError::AdapterControlUnavailable)
                }
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ExecutorOutcome {
    Succeeded {
        safe_summary: Value,
        checkpoint: Option<Value>,
    },
    Failed {
        safe_error_category: String,
        checkpoint: Option<Value>,
    },
    Cancelled {
        checkpoint: Option<Value>,
    },
    Uncertain {
        safe_error_category: String,
        checkpoint: Option<Value>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ExecutorCommand {
    pub(crate) operation_id: String,
    pub(crate) kind: String,
    pub(crate) dry_run: bool,
    pub(crate) executor_fence: i64,
    pub(crate) safe_summary: Value,
}

pub(crate) trait OperationalExecutor: Send + Sync {
    fn execute<'a>(&'a self, command: ExecutorCommand) -> ControlFuture<'a, ExecutorOutcome>;
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DeterministicFakeExecutor {
    scripted: Arc<Mutex<VecDeque<ExecutorOutcome>>>,
}

impl DeterministicFakeExecutor {
    #[cfg(test)]
    pub(crate) fn scripted(outcomes: impl IntoIterator<Item = ExecutorOutcome>) -> Self {
        Self {
            scripted: Arc::new(Mutex::new(outcomes.into_iter().collect())),
        }
    }
}

impl OperationalExecutor for DeterministicFakeExecutor {
    fn execute<'a>(&'a self, command: ExecutorCommand) -> ControlFuture<'a, ExecutorOutcome> {
        Box::pin(async move {
            if let Some(outcome) = self
                .scripted
                .lock()
                .map_err(|_| ControlPlaneError::Internal)?
                .pop_front()
            {
                return Ok(outcome);
            }
            if command.kind != "adapter_dry_run" || !command.dry_run {
                return Err(ControlPlaneError::UnsupportedOperationKind);
            }
            Ok(ExecutorOutcome::Succeeded {
                safe_summary: serde_json::json!({
                    "status": "passed",
                    "writes": "none",
                    "executor": "stage11_deterministic_fake",
                    "operation_id": command.operation_id,
                    "executor_fence": command.executor_fence,
                    "plan": command.safe_summary
                }),
                checkpoint: Some(serde_json::json!({
                    "phase": "complete",
                    "external_effects": false
                })),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_executor_is_allowlisted_and_never_accepts_arbitrary_actions() {
        let executor = DeterministicFakeExecutor::default();
        let unsupported = executor
            .execute(ExecutorCommand {
                operation_id: "operation-a".into(),
                kind: "arbitrary_shell".into(),
                dry_run: false,
                executor_fence: 1,
                safe_summary: serde_json::json!({}),
            })
            .await;
        assert_eq!(unsupported, Err(ControlPlaneError::UnsupportedOperationKind));
        let dry_run = executor
            .execute(ExecutorCommand {
                operation_id: "operation-b".into(),
                kind: "adapter_dry_run".into(),
                dry_run: true,
                executor_fence: 1,
                safe_summary: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(matches!(dry_run, ExecutorOutcome::Succeeded { .. }));
    }

    #[tokio::test]
    async fn fake_adapter_control_reports_timeout_and_does_not_fabricate_evidence() {
        let control = DeterministicFakeAdapterControl::scripted([
            ScriptedAdapterOutcome::Timeout,
            ScriptedAdapterOutcome::Failed,
        ]);
        assert_eq!(
            control.collect_quiescence(&[]).await,
            Err(ControlPlaneError::QuiescenceTimeout)
        );
        assert_eq!(
            control.await_resume(&[]).await,
            Err(ControlPlaneError::AdapterControlUnavailable)
        );
    }
}
