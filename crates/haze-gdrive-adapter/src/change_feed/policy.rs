use crate::drive::ProviderErrorCategory;
use std::collections::BTreeSet;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryDisposition {
    RetryAfter(Duration),
    Reauthenticate,
    DoNotRetry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderBackoffPolicy {
    delays: [Duration; 4],
}

impl ProviderBackoffPolicy {
    pub const fn new(delays: [Duration; 4]) -> Self {
        Self { delays }
    }

    pub fn classify(&self, category: ProviderErrorCategory, attempt: usize) -> RetryDisposition {
        match category {
            ProviderErrorCategory::RateLimit
            | ProviderErrorCategory::ProviderUnavailable
            | ProviderErrorCategory::Internal => {
                let index = attempt.min(self.delays.len() - 1);
                RetryDisposition::RetryAfter(self.delays[index])
            }
            ProviderErrorCategory::Auth => RetryDisposition::Reauthenticate,
            ProviderErrorCategory::NotFound
            | ProviderErrorCategory::Unsupported
            | ProviderErrorCategory::InvalidRequest => RetryDisposition::DoNotRetry,
        }
    }
}

impl Default for ProviderBackoffPolicy {
    fn default() -> Self {
        Self::new([
            Duration::from_secs(5),
            Duration::from_secs(15),
            Duration::from_secs(60),
            Duration::from_secs(300),
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangePollTrigger {
    PollTick,
    ProviderSignal,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebouncedPoll {
    pub trigger_count: u32,
    pub trigger_kinds: BTreeSet<ChangePollTrigger>,
    pub first_trigger_millis: u64,
    pub last_trigger_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePollDebouncer {
    debounce_window: Duration,
    pending: Option<DebouncedPoll>,
}

impl ChangePollDebouncer {
    pub const fn new(debounce_window: Duration) -> Self {
        Self {
            debounce_window,
            pending: None,
        }
    }

    pub fn record_trigger(&mut self, trigger: ChangePollTrigger, at_millis: u64) {
        match &mut self.pending {
            Some(pending) => {
                pending.trigger_count = pending.trigger_count.saturating_add(1);
                pending.trigger_kinds.insert(trigger);
                pending.last_trigger_millis = pending.last_trigger_millis.max(at_millis);
                pending.first_trigger_millis = pending.first_trigger_millis.min(at_millis);
            }
            None => {
                self.pending = Some(DebouncedPoll {
                    trigger_count: 1,
                    trigger_kinds: BTreeSet::from([trigger]),
                    first_trigger_millis: at_millis,
                    last_trigger_millis: at_millis,
                });
            }
        }
    }

    pub fn take_due(&mut self, now_millis: u64) -> Option<DebouncedPoll> {
        let pending = self.pending.as_ref()?;
        let window_millis = u64::try_from(self.debounce_window.as_millis()).unwrap_or(u64::MAX);
        if now_millis < pending.last_trigger_millis.saturating_add(window_millis) {
            return None;
        }
        self.pending.take()
    }

    pub const fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
}
