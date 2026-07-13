//! Production filesystem watcher with path-free bounded hints.

use crate::{WorktreeWatcher, WorktreeWatcherFailure, WorktreeWatcherHint, WorktreeWatcherPoll};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProductionWorktreeWatcherState {
    Created,
    Running,
    Closed,
    Failed,
    Stopped,
}

pub struct ProductionWorktreeWatcher {
    root: PathBuf,
    capacity: usize,
    receiver: Option<Receiver<()>>,
    watcher: Option<RecommendedWatcher>,
    overflowed: Arc<AtomicBool>,
    backend_failed: Arc<AtomicBool>,
    sequence: AtomicU64,
    state: ProductionWorktreeWatcherState,
}

impl ProductionWorktreeWatcher {
    pub fn new(root: PathBuf, capacity: usize) -> Result<Self, WorktreeWatcherFailure> {
        if !root.is_absolute() || capacity == 0 {
            return Err(WorktreeWatcherFailure::Start);
        }
        Ok(Self {
            root,
            capacity,
            receiver: None,
            watcher: None,
            overflowed: Arc::new(AtomicBool::new(false)),
            backend_failed: Arc::new(AtomicBool::new(false)),
            sequence: AtomicU64::new(0),
            state: ProductionWorktreeWatcherState::Created,
        })
    }

    #[must_use]
    pub const fn state(&self) -> ProductionWorktreeWatcherState {
        self.state
    }

    fn next_hint(&self) -> WorktreeWatcherPoll {
        WorktreeWatcherPoll::Hint(WorktreeWatcherHint {
            sequence: self
                .sequence
                .fetch_add(1, Ordering::AcqRel)
                .saturating_add(1),
        })
    }
}

impl fmt::Debug for ProductionWorktreeWatcher {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProductionWorktreeWatcher")
            .field("root", &"<redacted>")
            .field("capacity", &self.capacity)
            .field("state", &self.state)
            .finish()
    }
}

impl WorktreeWatcher for ProductionWorktreeWatcher {
    fn start(&mut self) -> Result<(), WorktreeWatcherFailure> {
        if self.state != ProductionWorktreeWatcherState::Created {
            return Err(WorktreeWatcherFailure::Start);
        }

        let (sender, receiver) = sync_channel(self.capacity);
        let overflowed = self.overflowed.clone();
        let backend_failed = self.backend_failed.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| match result {
                Ok(_) => send_hint(&sender, &overflowed),
                Err(_) => backend_failed.store(true, Ordering::Release),
            },
            Config::default(),
        )
        .map_err(|_| WorktreeWatcherFailure::Start)?;
        watcher
            .watch(&self.root, RecursiveMode::Recursive)
            .map_err(|_| WorktreeWatcherFailure::Start)?;

        self.receiver = Some(receiver);
        self.watcher = Some(watcher);
        self.state = ProductionWorktreeWatcherState::Running;
        Ok(())
    }

    fn poll_hint(&mut self) -> Result<WorktreeWatcherPoll, WorktreeWatcherFailure> {
        if self.backend_failed.swap(false, Ordering::AcqRel) {
            self.state = ProductionWorktreeWatcherState::Failed;
            return Err(WorktreeWatcherFailure::Poll);
        }
        if self.overflowed.swap(false, Ordering::AcqRel) {
            return Ok(self.next_hint());
        }

        let Some(receiver) = self.receiver.as_ref() else {
            return match self.state {
                ProductionWorktreeWatcherState::Closed
                | ProductionWorktreeWatcherState::Stopped => Ok(WorktreeWatcherPoll::Closed),
                _ => Err(WorktreeWatcherFailure::Poll),
            };
        };

        match receiver.try_recv() {
            Ok(()) => Ok(self.next_hint()),
            Err(TryRecvError::Empty) => Ok(WorktreeWatcherPoll::Idle),
            Err(TryRecvError::Disconnected) => {
                self.state = ProductionWorktreeWatcherState::Closed;
                Ok(WorktreeWatcherPoll::Closed)
            }
        }
    }

    fn shutdown(&mut self) -> Result<(), WorktreeWatcherFailure> {
        match self.state {
            ProductionWorktreeWatcherState::Created => {
                return Err(WorktreeWatcherFailure::Shutdown)
            }
            ProductionWorktreeWatcherState::Stopped => return Ok(()),
            _ => {}
        }
        self.watcher.take();
        self.receiver.take();
        self.state = ProductionWorktreeWatcherState::Stopped;
        Ok(())
    }
}

impl Drop for ProductionWorktreeWatcher {
    fn drop(&mut self) {
        self.watcher.take();
        self.receiver.take();
        self.state = ProductionWorktreeWatcherState::Stopped;
    }
}

fn send_hint(sender: &SyncSender<()>, overflowed: &AtomicBool) {
    match sender.try_send(()) {
        Ok(()) => {}
        Err(TrySendError::Full(())) => overflowed.store(true, Ordering::Release),
        Err(TrySendError::Disconnected(())) => {}
    }
}
