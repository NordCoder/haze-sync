use super::model::{required_page_token, ChangeFeedModelError, DriveChangePoll};
use crate::drive::ProviderError;
use std::collections::BTreeMap;
use std::fmt;

pub trait DriveChangeFeedProvider {
    fn get_start_page_token(&self) -> Result<String, ProviderError>;

    fn list_changes(&self, page_token: &str) -> Result<DriveChangePoll, ProviderError>;
}

#[derive(Clone)]
pub struct FakeDriveChangeFeedProvider {
    start_page_token: String,
    polls_by_page_token: BTreeMap<String, DriveChangePoll>,
    errors_by_operation: BTreeMap<&'static str, ProviderError>,
}

impl FakeDriveChangeFeedProvider {
    pub fn new(start_page_token: impl Into<String>) -> Result<Self, ChangeFeedModelError> {
        Ok(Self {
            start_page_token: required_page_token(start_page_token)?,
            polls_by_page_token: BTreeMap::new(),
            errors_by_operation: BTreeMap::new(),
        })
    }

    pub fn with_poll(mut self, page_token: impl Into<String>, poll: DriveChangePoll) -> Self {
        self.polls_by_page_token.insert(page_token.into(), poll);
        self
    }

    pub fn with_error(mut self, operation: &'static str, error: ProviderError) -> Self {
        self.errors_by_operation.insert(operation, error);
        self
    }

    fn configured_error(&self, operation: &'static str) -> Option<ProviderError> {
        self.errors_by_operation.get(operation).cloned()
    }
}

impl fmt::Debug for FakeDriveChangeFeedProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FakeDriveChangeFeedProvider")
            .field("configured_pages", &self.polls_by_page_token.len())
            .field("configured_errors", &self.errors_by_operation.len())
            .finish()
    }
}

impl DriveChangeFeedProvider for FakeDriveChangeFeedProvider {
    fn get_start_page_token(&self) -> Result<String, ProviderError> {
        if let Some(error) = self.configured_error("get_start_page_token") {
            return Err(error);
        }
        Ok(self.start_page_token.clone())
    }

    fn list_changes(&self, page_token: &str) -> Result<DriveChangePoll, ProviderError> {
        if let Some(error) = self.configured_error("list_changes") {
            return Err(error);
        }
        self.polls_by_page_token
            .get(page_token)
            .cloned()
            .ok_or_else(|| ProviderError::not_found("list_changes"))
    }
}
