//! Temporary object-root helper for tests that need filesystem storage space.

use super::{unique_test_id, TestSupportError};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Temporary object-store root for storage/Core tests.
///
/// The directory is removed on drop by default. Call [`Self::cleanup`] when a
/// test must verify cleanup success, or [`Self::keep`] only while debugging a
/// local test run.
pub struct TestObjectRoot {
    path: PathBuf,
    keep: bool,
}

impl TestObjectRoot {
    /// Creates a temporary object root with `sha256` and `tmp` subdirectories.
    pub fn create() -> Result<Self, TestSupportError> {
        let path = std::env::temp_dir().join(unique_test_id("haze-sync-object-root"));
        Self::create_at(path)
    }

    fn create_at(path: PathBuf) -> Result<Self, TestSupportError> {
        let layout_result = fs::create_dir_all(path.join("sha256"))
            .and_then(|()| fs::create_dir_all(path.join("tmp")));

        if layout_result.is_err() {
            let _ = fs::remove_dir_all(&path);
            return Err(TestSupportError::FilesystemOperationFailed);
        }

        Ok(Self { path, keep: false })
    }

    /// Returns the temporary object root path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the directory intended for SHA-256-addressed objects.
    #[must_use]
    pub fn sha256_dir(&self) -> PathBuf {
        self.path.join("sha256")
    }

    /// Returns the directory intended for temporary object writes.
    #[must_use]
    pub fn tmp_dir(&self) -> PathBuf {
        self.path.join("tmp")
    }

    /// Removes the directory and reports cleanup failures without exposing paths.
    ///
    /// A failed explicit cleanup leaves Drop cleanup enabled for one final
    /// best-effort retry.
    pub fn cleanup(mut self) -> Result<(), TestSupportError> {
        fs::remove_dir_all(&self.path)
            .map(|()| self.keep = true)
            .map_err(|_| TestSupportError::FilesystemOperationFailed)
    }

    /// Keeps the directory on disk and returns its path.
    #[must_use]
    pub fn keep(mut self) -> PathBuf {
        self.keep = true;
        self.path.clone()
    }
}

impl Drop for TestObjectRoot {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

impl fmt::Debug for TestObjectRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TestObjectRoot")
            .field("path", &"<temporary object root>")
            .field("keep", &self.keep)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_object_root_directories() {
        let root = TestObjectRoot::create().expect("object root should be created");

        assert!(root.path().is_dir());
        assert!(root.sha256_dir().is_dir());
        assert!(root.tmp_dir().is_dir());
        assert!(!format!("{root:?}").contains(root.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn partial_layout_failure_removes_created_root() {
        let path = std::env::temp_dir().join(unique_test_id("haze-sync-partial-root"));
        fs::create_dir_all(&path).expect("fixture root should be created");
        fs::write(path.join("tmp"), b"blocks tmp directory creation")
            .expect("blocking fixture should be created");

        assert!(matches!(
            TestObjectRoot::create_at(path.clone()),
            Err(TestSupportError::FilesystemOperationFailed)
        ));
        assert!(!path.exists());
    }

    #[test]
    fn explicit_cleanup_removes_root() {
        let root = TestObjectRoot::create().expect("object root should be created");
        let path = root.path().to_owned();

        root.cleanup().expect("object root cleanup should succeed");

        assert!(!path.exists());
    }

    #[test]
    fn explicit_cleanup_failure_is_redacted() {
        let root = TestObjectRoot::create().expect("object root should be created");
        fs::remove_dir_all(root.path()).expect("fixture should remove root before cleanup");

        let error = root.cleanup().unwrap_err();

        assert_eq!(error, TestSupportError::FilesystemOperationFailed);
        assert!(!error.to_string().contains(std::path::MAIN_SEPARATOR));
    }
}
