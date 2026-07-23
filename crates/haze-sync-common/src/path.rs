//! Vault-relative path primitives.
//!
//! `VaultPath` owns the final synchronized vault path representation. Provider
//! path reconstruction, local filesystem traversal, and HTTP extraction happen
//! in their owning components before constructing this validated value.

use crate::ValidationError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

/// A normalized relative path inside the synchronized vault view.
///
/// `VaultPath` percent-decodes input before validation, normalizes duplicate
/// separators and `.` segments, and rejects empty paths, absolute paths, path
/// traversal, Windows drive prefixes, backslash separators, null bytes, and
/// runtime/state paths that are reserved by the V1 contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultPath(String);

impl VaultPath {
    /// Validate and normalize a vault path.
    pub fn parse(input: &str) -> Result<Self, ValidationError> {
        let decoded = decode_percent_sequences(input)?;
        let input = decoded.as_ref();

        if input.is_empty() {
            return Err(ValidationError::EmptyPath);
        }

        if input.as_bytes().contains(&0) {
            return Err(ValidationError::NullByte);
        }

        if has_windows_drive_prefix(input) {
            return Err(ValidationError::WindowsDrivePrefix);
        }

        if input.contains('\\') {
            return Err(ValidationError::WindowsSeparator);
        }

        if input.starts_with('/') || input == "~" || input.starts_with("~/") {
            return Err(ValidationError::AbsolutePath);
        }

        let mut normalized_segments = Vec::new();
        for segment in input.split('/') {
            match segment {
                "" | "." => {}
                ".." => return Err(ValidationError::PathTraversal),
                safe_segment => normalized_segments.push(safe_segment),
            }
        }

        if normalized_segments.is_empty() {
            return Err(ValidationError::EmptyPath);
        }

        let normalized = normalized_segments.join("/");
        if is_reserved_runtime_path(&normalized) {
            return Err(ValidationError::RuntimePath);
        }

        Ok(Self(normalized))
    }

    /// Borrow the normalized vault path as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the value and return the normalized vault path string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Iterate over normalized path segments.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }

    /// Returns true when a normalized path is reserved runtime state.
    #[must_use]
    pub fn is_reserved_runtime_path(&self) -> bool {
        is_reserved_runtime_path(&self.0)
    }
}

impl AsRef<str> for VaultPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for VaultPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VaultPath {
    type Err = ValidationError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

impl TryFrom<&str> for VaultPath {
    type Error = ValidationError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::parse(input)
    }
}

impl Serialize for VaultPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for VaultPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

fn decode_percent_sequences(input: &str) -> Result<Cow<'_, str>, ValidationError> {
    if !input.as_bytes().contains(&b'%') {
        return Ok(Cow::Borrowed(input));
    }

    let input_bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(input_bytes.len());
    let mut index = 0;

    while index < input_bytes.len() {
        if input_bytes[index] == b'%' {
            if index + 2 >= input_bytes.len() {
                return Err(ValidationError::InvalidPercentEncoding);
            }
            let hi = percent_nibble(input_bytes[index + 1])?;
            let lo = percent_nibble(input_bytes[index + 2])?;
            decoded.push((hi << 4) | lo);
            index += 3;
        } else {
            decoded.push(input_bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded)
        .map(Cow::Owned)
        .map_err(|_| ValidationError::InvalidPercentEncoding)
}

fn percent_nibble(byte: u8) -> Result<u8, ValidationError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ValidationError::InvalidPercentEncoding),
    }
}

fn has_windows_drive_prefix(input: &str) -> bool {
    let bytes = input.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn is_reserved_runtime_path(normalized: &str) -> bool {
    let first_segment = normalized.split('/').next().unwrap_or_default();
    matches!(
        first_segment,
        "_haze_runtime" | "_haze_tmp" | "state" | "logs" | "trash"
    ) || normalized.ends_with(".tmp")
        || normalized.ends_with(".part")
        || normalized.ends_with(".swp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_safe_subdirectory_path() {
        let path = VaultPath::parse("Notes/a.md").unwrap();
        assert_eq!(path.as_str(), "Notes/a.md");
        assert_eq!(path.as_ref(), "Notes/a.md");
        assert_eq!(path.to_string(), "Notes/a.md");
        assert_eq!(path.clone().into_string(), "Notes/a.md");
        assert_eq!(VaultPath::from_str("Notes/a.md").unwrap(), path);
        assert_eq!(VaultPath::try_from("Notes/a.md").unwrap(), path);
    }

    #[test]
    fn normalizes_dot_and_duplicate_separators() {
        for input in ["./Notes//./a.md", "Notes/%2e/%2E/a.md"] {
            let path = VaultPath::parse(input).unwrap();
            assert_eq!(path.as_str(), "Notes/a.md", "input={input:?}");
            assert_eq!(path.segments().collect::<Vec<_>>(), vec!["Notes", "a.md"]);
        }
    }

    #[test]
    fn normalizes_safely_decoded_url_path() {
        for input in ["Notes%2Fa.md", "Notes%2fa.md"] {
            let path = VaultPath::parse(input).unwrap();
            assert_eq!(path.as_str(), "Notes/a.md", "input={input:?}");
        }
    }

    #[test]
    fn accepts_home_like_names_that_are_not_home_paths() {
        let path = VaultPath::parse("~drafts/today.md").unwrap();
        assert_eq!(path.as_str(), "~drafts/today.md");
    }

    #[test]
    fn rejects_empty_paths_after_normalization() {
        for input in ["", ".", "./", "//", "/"] {
            let expected = if input.starts_with('/') {
                ValidationError::AbsolutePath
            } else {
                ValidationError::EmptyPath
            };
            assert_eq!(
                VaultPath::parse(input).unwrap_err(),
                expected,
                "input={input:?}"
            );
        }
    }

    #[test]
    fn rejects_path_traversal() {
        for input in [
            "../a.md",
            "Notes/../../bad.md",
            "%2e%2e/a.md",
            "%2E%2e/a.md",
            "Notes/%2e%2E/bad.md",
            "Notes/%2E%2E%2Fbad.md",
            "Notes%2F..%2Fbad.md",
        ] {
            assert_eq!(
                VaultPath::parse(input).unwrap_err(),
                ValidationError::PathTraversal,
                "input={input:?}"
            );
        }
    }

    #[test]
    fn rejects_absolute_path() {
        for input in [
            "/etc/passwd",
            "//server/share",
            "~/vault.md",
            "~",
            "%2Fetc%2Fpasswd",
            "%2fetc%2fpasswd",
            "%2F%2Fserver%2Fshare",
            "~%2Fvault.md",
        ] {
            assert_eq!(
                VaultPath::parse(input).unwrap_err(),
                ValidationError::AbsolutePath,
                "input={input:?}"
            );
        }
    }

    #[test]
    fn rejects_windows_drive_prefix() {
        for input in [
            "C:\\secret",
            "C:/secret",
            "c:/secret",
            "Z%3A/secret",
            "C%3A%5Csecret",
        ] {
            assert_eq!(
                VaultPath::parse(input).unwrap_err(),
                ValidationError::WindowsDrivePrefix,
                "input={input:?}"
            );
        }
    }

    #[test]
    fn rejects_windows_separator() {
        for input in [
            "Notes\\secret.md",
            "Notes%5Csecret.md",
            "Notes%5csecret.md",
            "Notes/subdir%5Csecret.md",
        ] {
            assert_eq!(
                VaultPath::parse(input).unwrap_err(),
                ValidationError::WindowsSeparator,
                "input={input:?}"
            );
        }
    }

    #[test]
    fn rejects_null_byte() {
        for input in ["Notes/a\0.md", "Notes/a%00.md", "%00.md", "Notes/%00"] {
            assert_eq!(
                VaultPath::parse(input).unwrap_err(),
                ValidationError::NullByte,
                "input={input:?}"
            );
        }
    }

    #[test]
    fn rejects_invalid_percent_encoding() {
        for input in [
            "Notes/%GG.md",
            "Notes/%",
            "Notes/%F0%28%8C%28",
            "Notes/%0",
            "Notes/%0X.md",
        ] {
            assert_eq!(
                VaultPath::parse(input).unwrap_err(),
                ValidationError::InvalidPercentEncoding,
                "input={input:?}"
            );
        }
    }

    #[test]
    fn rejects_runtime_state_paths() {
        for input in [
            "_haze_runtime",
            "_haze_runtime/",
            "_haze_runtime/cache.json",
            "./_haze_runtime/cache.json",
            "_haze_runtime%2Fcache.json",
            "_haze_tmp",
            "_haze_tmp/upload",
            "state",
            "state/db.json",
            "logs",
            "logs/server.log",
            "logs%2Fserver.log",
            "trash",
            "trash/deleted.md",
            "Notes/upload.tmp",
            "Notes%2Fupload.tmp",
            "Notes/upload.part",
            "Notes/.draft.swp",
        ] {
            let error = VaultPath::parse(input).unwrap_err();
            assert_eq!(error, ValidationError::RuntimePath, "input={input:?}");
        }
    }

    #[test]
    fn accepts_paths_that_only_resemble_runtime_state() {
        for input in [
            "_haze_runtime_notes/cache.json",
            "_haze_tmp_notes/upload.md",
            "stateful/db.json",
            "logs-md/server.log",
            "trashcan/deleted.md",
            "Notes/upload.tmp.md",
            "Notes/.draft.swp.md",
        ] {
            let path = VaultPath::parse(input).unwrap();
            assert_eq!(path.as_str(), input, "input={input:?}");
            assert!(!path.is_reserved_runtime_path(), "input={input:?}");
        }
    }

    #[test]
    fn conflict_and_agent_paths_are_syncable_vault_paths() {
        for (input, normalized) in [
            ("_haze_conflicts", "_haze_conflicts"),
            (
                "_haze_conflicts/open/Projects/Haze/plan.md",
                "_haze_conflicts/open/Projects/Haze/plan.md",
            ),
            (
                "_haze_conflicts%2Fopen%2FProjects%2FHaze%2Fplan.md",
                "_haze_conflicts/open/Projects/Haze/plan.md",
            ),
            ("_haze_agent_outbox", "_haze_agent_outbox"),
            ("_haze_agent_outbox/draft.md", "_haze_agent_outbox/draft.md"),
        ] {
            let path = VaultPath::parse(input).unwrap();
            assert_eq!(path.as_str(), normalized, "input={input:?}");
            assert!(!path.is_reserved_runtime_path(), "input={input:?}");
        }
    }

    #[test]
    fn serde_roundtrip() {
        let path = VaultPath::parse("Notes/a.md").unwrap();
        let json = serde_json::to_string(&path).unwrap();
        assert_eq!(json, "\"Notes/a.md\"");
        let decoded: VaultPath = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, path);
    }

    #[test]
    fn serde_rejects_invalid_paths() {
        for json in [
            "\"../a.md\"",
            "\"/etc/passwd\"",
            "\"Notes%2F..%2Fbad.md\"",
            "\"Notes/a%00.md\"",
            "\"_haze_runtime/cache.json\"",
        ] {
            assert!(
                serde_json::from_str::<VaultPath>(json).is_err(),
                "json={json:?}"
            );
        }
    }
}
