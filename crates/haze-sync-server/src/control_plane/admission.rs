use std::sync::{
    atomic::{AtomicU8, AtomicUsize, Ordering},
    Arc,
};

use tokio::sync::Notify;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DurableMaintenanceState {
    Normal,
    Quiescing,
    Quiesced,
    Maintenance,
    Resuming,
    FailClosed,
}

impl DurableMaintenanceState {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "normal" => Some(Self::Normal),
            "quiescing" => Some(Self::Quiescing),
            "quiesced" => Some(Self::Quiesced),
            "maintenance" => Some(Self::Maintenance),
            "resuming" => Some(Self::Resuming),
            _ => None,
        }
    }

    const fn as_u8(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Quiescing => 1,
            Self::Quiesced => 2,
            Self::Maintenance => 3,
            Self::Resuming => 4,
            Self::FailClosed => 5,
        }
    }

    const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Normal,
            1 => Self::Quiescing,
            2 => Self::Quiesced,
            3 => Self::Maintenance,
            4 => Self::Resuming,
            _ => Self::FailClosed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AdmissionClass {
    AuthoritativeMutation,
    PublicRead,
    ChangeFeedRead,
    CredentialCreateOrRotate,
    CredentialRevoke,
    OperationalPlan,
    ControlRead,
}

#[derive(Clone)]
pub(crate) struct AdmissionController {
    inner: Arc<AdmissionInner>,
}

struct AdmissionInner {
    state: AtomicU8,
    active_authoritative_mutations: AtomicUsize,
    drained: Notify,
}

impl AdmissionController {
    pub(crate) fn fail_closed() -> Self {
        Self {
            inner: Arc::new(AdmissionInner {
                state: AtomicU8::new(DurableMaintenanceState::FailClosed.as_u8()),
                active_authoritative_mutations: AtomicUsize::new(0),
                drained: Notify::new(),
            }),
        }
    }

    pub(crate) fn from_state(state: DurableMaintenanceState) -> Self {
        let controller = Self::fail_closed();
        controller.set_state(state);
        controller
    }

    pub(crate) fn state(&self) -> DurableMaintenanceState {
        DurableMaintenanceState::from_u8(self.inner.state.load(Ordering::Acquire))
    }

    pub(crate) fn set_state(&self, state: DurableMaintenanceState) {
        self.inner.state.store(state.as_u8(), Ordering::Release);
    }

    pub(crate) fn active_authoritative_mutations(&self) -> usize {
        self.inner
            .active_authoritative_mutations
            .load(Ordering::Acquire)
    }

    pub(crate) fn admit(
        &self,
        class: AdmissionClass,
    ) -> Result<Option<AuthoritativeMutationPermit>, AdmissionDenied> {
        let state = self.state();
        match class {
            AdmissionClass::AuthoritativeMutation => {
                if state != DurableMaintenanceState::Normal {
                    return Err(AdmissionDenied::Maintenance(state));
                }
                self.inner
                    .active_authoritative_mutations
                    .fetch_add(1, Ordering::AcqRel);
                if self.state() != DurableMaintenanceState::Normal {
                    self.release_mutation();
                    return Err(AdmissionDenied::Maintenance(self.state()));
                }
                Ok(Some(AuthoritativeMutationPermit {
                    controller: self.clone(),
                }))
            }
            AdmissionClass::PublicRead | AdmissionClass::ChangeFeedRead => {
                if matches!(
                    state,
                    DurableMaintenanceState::Maintenance
                        | DurableMaintenanceState::Resuming
                        | DurableMaintenanceState::FailClosed
                ) {
                    Err(AdmissionDenied::Maintenance(state))
                } else {
                    Ok(None)
                }
            }
            AdmissionClass::CredentialCreateOrRotate => {
                if matches!(
                    state,
                    DurableMaintenanceState::Normal
                        | DurableMaintenanceState::Quiesced
                        | DurableMaintenanceState::Maintenance
                ) {
                    Ok(None)
                } else {
                    Err(AdmissionDenied::Maintenance(state))
                }
            }
            AdmissionClass::CredentialRevoke | AdmissionClass::ControlRead => Ok(None),
            AdmissionClass::OperationalPlan => {
                if matches!(
                    state,
                    DurableMaintenanceState::Normal
                        | DurableMaintenanceState::Quiesced
                        | DurableMaintenanceState::Maintenance
                ) {
                    Ok(None)
                } else {
                    Err(AdmissionDenied::Maintenance(state))
                }
            }
        }
    }

    pub(crate) async fn wait_for_authoritative_drain(
        &self,
        timeout: std::time::Duration,
    ) -> Result<(), AdmissionDenied> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if self.active_authoritative_mutations() == 0 {
                return Ok(());
            }
            let notified = self.inner.drained.notified();
            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                return Err(AdmissionDenied::DrainTimeout);
            }
        }
    }

    fn release_mutation(&self) {
        let previous = self
            .inner
            .active_authoritative_mutations
            .fetch_sub(1, Ordering::AcqRel);
        if previous <= 1 {
            self.inner.drained.notify_waiters();
        }
    }
}

impl std::fmt::Debug for AdmissionController {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmissionController")
            .field("state", &self.state())
            .field(
                "active_authoritative_mutations",
                &self.active_authoritative_mutations(),
            )
            .finish()
    }
}

pub(crate) struct AuthoritativeMutationPermit {
    controller: AdmissionController,
}

impl Drop for AuthoritativeMutationPermit {
    fn drop(&mut self) {
        self.controller.release_mutation();
    }
}

impl std::fmt::Debug for AuthoritativeMutationPermit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AuthoritativeMutationPermit(active)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AdmissionDenied {
    Maintenance(DurableMaintenanceState),
    DrainTimeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fence_rejects_route_and_non_route_mutations_and_drains_existing_permits() {
        let admission = AdmissionController::from_state(DurableMaintenanceState::Normal);
        let permit = admission
            .admit(AdmissionClass::AuthoritativeMutation)
            .unwrap()
            .unwrap();
        admission.set_state(DurableMaintenanceState::Quiescing);
        assert!(matches!(
            admission.admit(AdmissionClass::AuthoritativeMutation),
            Err(AdmissionDenied::Maintenance(DurableMaintenanceState::Quiescing))
        ));
        let waiter = {
            let admission = admission.clone();
            tokio::spawn(async move {
                admission
                    .wait_for_authoritative_drain(std::time::Duration::from_secs(1))
                    .await
            })
        };
        drop(permit);
        assert_eq!(waiter.await.unwrap(), Ok(()));
    }

    #[test]
    fn maintenance_admission_matrix_is_fail_closed() {
        let admission = AdmissionController::from_state(DurableMaintenanceState::Maintenance);
        assert!(admission.admit(AdmissionClass::ControlRead).is_ok());
        assert!(admission.admit(AdmissionClass::CredentialRevoke).is_ok());
        assert!(admission
            .admit(AdmissionClass::CredentialCreateOrRotate)
            .is_ok());
        assert!(admission.admit(AdmissionClass::PublicRead).is_err());
        admission.set_state(DurableMaintenanceState::FailClosed);
        assert!(admission
            .admit(AdmissionClass::AuthoritativeMutation)
            .is_err());
    }
}
