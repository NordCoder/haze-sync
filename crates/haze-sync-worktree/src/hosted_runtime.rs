use crate::{WorktreeRuntimeClock, WorktreeRuntimeCycle, WorktreeRuntimeLifecycle, WorktreeRuntimeLifecycleError, WorktreeRuntimeManualOutcome, WorktreeRuntimeManualRequest, WorktreeRuntimePoll, WorktreeRuntimeService, WorktreeRuntimeShutdownSummary, WorktreeRuntimeStartSummary, WorktreeRuntimeStatus, WorktreeWatcher};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::Arc;

const CREATED: u8 = 0;
const RUNNING: u8 = 1;
const CANCELLING: u8 = 2;
const SHUTDOWN: u8 = 3;

struct Envelope {
    request: WorktreeRuntimeManualRequest,
    response: SyncSender<WorktreeRuntimeManualOutcome>,
}

struct Gate {
    lifecycle: AtomicU8,
    busy: AtomicBool,
}

#[derive(Clone)]
pub struct WorktreeRuntimeManualHandle {
    sender: SyncSender<Envelope>,
    gate: Arc<Gate>,
}

impl fmt::Debug for WorktreeRuntimeManualHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorktreeRuntimeManualHandle")
            .field("lifecycle", &self.lifecycle())
            .field("busy", &self.gate.busy.load(Ordering::Acquire))
            .finish()
    }
}

impl WorktreeRuntimeManualHandle {
    pub fn submit(&self, request: WorktreeRuntimeManualRequest) -> WorktreeRuntimeManualSubmission {
        match self.lifecycle() {
            WorktreeRuntimeLifecycle::Created => return WorktreeRuntimeManualSubmission::NotStarted,
            WorktreeRuntimeLifecycle::Cancelling => return WorktreeRuntimeManualSubmission::Cancelling,
            WorktreeRuntimeLifecycle::Shutdown => return WorktreeRuntimeManualSubmission::Shutdown,
            WorktreeRuntimeLifecycle::Running => {}
        }
        if self.gate.busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
            return WorktreeRuntimeManualSubmission::Busy;
        }
        let (response, receiver) = sync_channel(1);
        match self.sender.try_send(Envelope { request, response }) {
            Ok(()) => WorktreeRuntimeManualSubmission::Accepted(WorktreeRuntimeManualTicket { receiver }),
            Err(TrySendError::Full(_)) => {
                self.gate.busy.store(false, Ordering::Release);
                WorktreeRuntimeManualSubmission::Busy
            }
            Err(TrySendError::Disconnected(_)) => {
                self.gate.busy.store(false, Ordering::Release);
                WorktreeRuntimeManualSubmission::Shutdown
            }
        }
    }

    fn lifecycle(&self) -> WorktreeRuntimeLifecycle {
        match self.gate.lifecycle.load(Ordering::Acquire) {
            CREATED => WorktreeRuntimeLifecycle::Created,
            RUNNING => WorktreeRuntimeLifecycle::Running,
            CANCELLING => WorktreeRuntimeLifecycle::Cancelling,
            _ => WorktreeRuntimeLifecycle::Shutdown,
        }
    }
}

pub enum WorktreeRuntimeManualSubmission {
    Accepted(WorktreeRuntimeManualTicket),
    Busy,
    NotStarted,
    Cancelling,
    Shutdown,
}

impl fmt::Debug for WorktreeRuntimeManualSubmission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Accepted(_) => "Accepted",
            Self::Busy => "Busy",
            Self::NotStarted => "NotStarted",
            Self::Cancelling => "Cancelling",
            Self::Shutdown => "Shutdown",
        })
    }
}

pub struct WorktreeRuntimeManualTicket {
    receiver: Receiver<WorktreeRuntimeManualOutcome>,
}

impl WorktreeRuntimeManualTicket {
    pub fn try_result(&self) -> WorktreeRuntimeManualTicketPoll {
        match self.receiver.try_recv() {
            Ok(value) => WorktreeRuntimeManualTicketPoll::Completed(value),
            Err(TryRecvError::Empty) => WorktreeRuntimeManualTicketPoll::Pending,
            Err(TryRecvError::Disconnected) => WorktreeRuntimeManualTicketPoll::Closed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeManualTicketPoll {
    Pending,
    Completed(WorktreeRuntimeManualOutcome),
    Closed,
}

pub struct WorktreeHostedRuntime<C, W, X> {
    service: WorktreeRuntimeService<C, W, X>,
    receiver: Receiver<Envelope>,
    gate: Arc<Gate>,
}

impl<C, W, X> WorktreeHostedRuntime<C, W, X>
where
    C: WorktreeRuntimeClock,
    W: WorktreeWatcher,
    X: WorktreeRuntimeCycle,
{
    pub fn new(service: WorktreeRuntimeService<C, W, X>, capacity: usize) -> Result<(Self, WorktreeRuntimeManualHandle), WorktreeHostedRuntimeError> {
        if capacity == 0 {
            return Err(WorktreeHostedRuntimeError::ZeroManualCapacity);
        }
        let (sender, receiver) = sync_channel(capacity);
        let gate = Arc::new(Gate { lifecycle: AtomicU8::new(CREATED), busy: AtomicBool::new(false) });
        let handle = WorktreeRuntimeManualHandle { sender, gate: gate.clone() };
        Ok((Self { service, receiver, gate }, handle))
    }

    pub fn start(&mut self) -> Result<WorktreeRuntimeStartSummary, WorktreeRuntimeLifecycleError> {
        let value = self.service.start()?;
        self.gate.lifecycle.store(RUNNING, Ordering::Release);
        Ok(value)
    }

    pub fn request_cancel(&mut self) -> Result<(), WorktreeRuntimeLifecycleError> {
        self.service.request_cancel()?;
        self.gate.lifecycle.store(CANCELLING, Ordering::Release);
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<WorktreeRuntimeShutdownSummary, WorktreeRuntimeLifecycleError> {
        let value = self.service.shutdown()?;
        self.gate.lifecycle.store(SHUTDOWN, Ordering::Release);
        self.gate.busy.store(false, Ordering::Release);
        Ok(value)
    }

    pub async fn poll(&mut self) -> Result<WorktreeHostedRuntimePoll, WorktreeRuntimeLifecycleError> {
        match self.receiver.try_recv() {
            Ok(envelope) => {
                let outcome = self.service.run_manual_cycle(envelope.request).await;
                let _ = envelope.response.try_send(outcome);
                self.gate.busy.store(false, Ordering::Release);
                Ok(WorktreeHostedRuntimePoll::Manual(outcome))
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {
                if self.gate.busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
                    return Ok(WorktreeHostedRuntimePoll::Idle);
                }
                let result = self.service.poll().await;
                self.gate.busy.store(false, Ordering::Release);
                result.map(WorktreeHostedRuntimePoll::Automatic)
            }
        }
    }

    #[must_use]
    pub const fn status(&self) -> WorktreeRuntimeStatus {
        self.service.status()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeHostedRuntimePoll {
    Idle,
    Manual(WorktreeRuntimeManualOutcome),
    Automatic(WorktreeRuntimePoll),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeHostedRuntimeError {
    ZeroManualCapacity,
}

impl fmt::Display for WorktreeHostedRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("worktree manual request capacity must be non-zero")
    }
}

impl std::error::Error for WorktreeHostedRuntimeError {}
