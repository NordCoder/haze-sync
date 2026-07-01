//! Small deterministic-enough identifier helpers for isolated tests.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Generates a process-local unique identifier with a sanitized prefix.
#[must_use]
pub fn unique_test_id(prefix: &str) -> String {
    let prefix = sanitize_label(prefix);
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let timestamp_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    format!(
        "{prefix}-{}-{timestamp_nanos}-{sequence}",
        std::process::id()
    )
}

/// Namespace for grouping rows, adapter ids, and paths owned by one test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestNamespace {
    id: String,
}

impl TestNamespace {
    /// Creates a fresh namespace with a sanitized prefix.
    #[must_use]
    pub fn new(prefix: &str) -> Self {
        Self {
            id: unique_test_id(prefix),
        }
    }

    /// Returns the namespace id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Produces a stable child identifier inside this namespace.
    #[must_use]
    pub fn child_id(&self, label: &str) -> String {
        format!("{}-{}", self.id, sanitize_label(label))
    }

    /// Produces an adapter id suitable for fixture rows.
    #[must_use]
    pub fn adapter_id(&self, label: &str) -> String {
        self.child_id(&format!("adapter-{label}"))
    }

    /// Produces an operation id suitable for fixture rows.
    #[must_use]
    pub fn operation_id(&self, label: &str) -> String {
        self.child_id(&format!("op-{label}"))
    }

    /// Produces a request id suitable for harness diagnostics.
    #[must_use]
    pub fn request_id(&self, label: &str) -> String {
        self.child_id(&format!("req-{label}"))
    }

    /// Produces a vault-relative path under `_haze_tests/` for future E2E tests.
    #[must_use]
    pub fn vault_path(&self, file_name: &str) -> String {
        format!(
            "_haze_tests/{}/{}",
            self.id,
            sanitize_path_segment(file_name)
        )
    }
}

fn sanitize_label(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');

    if trimmed.is_empty() {
        "test".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn sanitize_path_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches(|ch| ch == '-' || ch == '.');
    let segment = if trimmed.is_empty() { "note" } else { trimmed };

    if segment.contains('.') {
        segment.to_owned()
    } else {
        format!("{segment}.md")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_ids_include_sanitized_prefix() {
        let first = unique_test_id("Storage Harness");
        let second = unique_test_id("Storage Harness");

        assert!(first.starts_with("storage-harness-"));
        assert!(second.starts_with("storage-harness-"));
        assert_ne!(first, second);
    }

    #[test]
    fn namespace_builds_related_fixture_identifiers() {
        let namespace = TestNamespace::new("Pg Fixture");

        assert!(namespace.adapter_id("iphone").contains(namespace.id()));
        assert!(namespace.operation_id("put file").contains("op-put-file"));
        assert!(namespace.request_id("upload").contains("req-upload"));
        assert!(namespace.vault_path("note").ends_with("/note.md"));
        assert!(namespace
            .vault_path("Folder/Note.md")
            .ends_with("/folder-note.md"));
    }
}
