//! Small deterministic-enough identifier helpers for isolated tests.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_SANITIZED_LABEL_LEN: usize = 32;
const MAX_SHARED_IDENTIFIER_LEN: usize = 128;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Generates a process-local unique identifier with a sanitized bounded prefix.
#[must_use]
pub fn unique_test_id(prefix: &str) -> String {
    let prefix = sanitize_label(prefix);
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let timestamp_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    format!(
        "{prefix}-{:x}-{timestamp_nanos:x}-{sequence:x}",
        std::process::id()
    )
}

/// Namespace for grouping rows, adapter ids, and paths owned by one test.
///
/// Namespace creation is unique per process, while child identifiers are stable
/// for the same label inside one namespace. Bounded labels keep generated shared
/// identifiers within the current common identifier limit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestNamespace {
    id: String,
}

impl TestNamespace {
    /// Creates a fresh namespace with a sanitized bounded prefix.
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
        let child = format!("{}-{}", self.id, sanitize_label(label));
        debug_assert!(child.len() <= MAX_SHARED_IDENTIFIER_LEN);
        child
    }

    /// Produces an adapter id suitable for fixture rows.
    #[must_use]
    pub fn adapter_id(&self, label: &str) -> String {
        self.child_id(&format!("adapter-{label}"))
    }

    /// Produces an operation id suitable for fixture rows.
    #[must_use]
    pub fn operation_id(&self, label: &str) -> String {
        let operation_id = format!("op_{}", self.child_id(&format!("op-{label}")));
        debug_assert!(operation_id.len() <= MAX_SHARED_IDENTIFIER_LEN);
        operation_id
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
    let label = if trimmed.is_empty() { "test" } else { trimmed };

    label.chars().take(MAX_SANITIZED_LABEL_LEN).collect()
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
    let bounded: String = segment.chars().take(MAX_SANITIZED_LABEL_LEN).collect();

    if bounded.contains('.') {
        bounded
    } else {
        format!("{bounded}.md")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_common::{AdapterId, OperationId, VaultPath};

    #[test]
    fn unique_ids_include_sanitized_prefix() {
        let first = unique_test_id("Storage Harness");
        let second = unique_test_id("Storage Harness");

        assert!(first.starts_with("storage-harness-"));
        assert!(second.starts_with("storage-harness-"));
        assert_ne!(first, second);
    }

    #[test]
    fn namespace_builds_stable_contract_valid_fixture_values() {
        let namespace = TestNamespace::new("Pg Fixture");
        let adapter_id = namespace.adapter_id("iphone");
        let operation_id = namespace.operation_id("put file");
        let vault_path = namespace.vault_path("Folder/Note.md");

        assert!(adapter_id.contains(namespace.id()));
        assert_eq!(operation_id, namespace.operation_id("put file"));
        assert!(operation_id.contains("op-put-file"));
        assert!(namespace.request_id("upload").contains("req-upload"));
        assert!(namespace.vault_path("note").ends_with("/note.md"));
        assert!(vault_path.ends_with("/folder-note.md"));

        AdapterId::parse(&adapter_id).expect("fixture adapter id should satisfy Common");
        OperationId::parse(&operation_id).expect("fixture operation id should satisfy Common");
        VaultPath::parse(&vault_path).expect("fixture vault path should satisfy Common");
    }

    #[test]
    fn generated_shared_identifiers_remain_bounded() {
        let namespace = TestNamespace::new(&"prefix".repeat(50));
        let child = namespace.child_id(&"label".repeat(100));
        let operation_id = namespace.operation_id(&"operation".repeat(100));

        assert!(child.len() <= MAX_SHARED_IDENTIFIER_LEN);
        assert!(operation_id.len() <= MAX_SHARED_IDENTIFIER_LEN);
        assert!(namespace.id().len() < child.len());
    }
}
