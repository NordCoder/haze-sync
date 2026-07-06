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
        let path = VaultPath::parse("./Notes//./a.md").unwrap();
        assert_eq!(path.as_str(), "Notes/a.md");
        assert_eq!(path.segments().collect::<Vec<_>>(), vec!["Notes", "a.md"]);
    }

    #[test]
    fn normalizes_safely_decoded_url_path() {
        let path = VaultPath::parse("Notes%2Fa.md").unwrap();
        assert_eq!(path.as_str(), "Notes/a.md");
    }

    #[test]
    fn rejects_empty_paths_after_normalization() {
        for input in ["", ".", "./", "//", "/"] {
            let expected = if input == "/" {
                ValidationError::AbsolutePath
            } else {
                ValidationError::EmptyPath
            };
            assert_eq!(VaultPath::parse(input).unwrap_err(), expected, "input={input:?}");
        }
    }

    #[test]
    fn rejects_path_traversal() {
        assert_eq!(
            VaultPath::parse("../a.md").unwrap_err(),
            ValidationError::PathTraversal
        );
        assert_eq!(
            VaultPath::parse("Notes/../../bad.md").unwrap_err(),
            ValidationError::PathTraversal
        );
        assert_eq!(
            VaultPath::parse("%2e%2e/a.md").unwrap_err(),
            ValidationError::PathTraversal
        );
    }

    #[test]
    fn rejects_absolute_path() {
        assert_eq!(
            VaultPath::parse("/etc/passwd").unwrap_err(),
            ValidationError::AbsolutePath
        );
        assert_eq!(
            VaultPath::parse("~/vault.md").unwrap_err(),
            ValidationError::AbsolutePath
        );
        assert_eq!(
            VaultPath::parse("~").unwrap_err(),
            ValidationError::AbsolutePath
        );
        assert_eq!(
            VaultPath::parse("%2Fetc%2Fpasswd").unwrap_err(),
            ValidationError::AbsolutePath
        );
    }

    #[test]
    fn rejects_windows_drive_prefix() {
        assert_eq!(
            VaultPath::parse("C:\\secret").unwrap_err(),
            ValidationError::WindowsDrivePrefix
        );
        assert_eq!(
            VaultPath::parse("C:/secret").unwrap_err(),
            ValidationError::WindowsDrivePrefix
        );
    }

    #[test]
    fn rejects_windows_separator() {
        assert_eq!(
            VaultPath::parse("Notes\\secret.md").unwrap_err(),
            ValidationError::WindowsSeparator
        );
        assert_eq!(
            VaultPath::parse("Notes%5Csecret.md").unwrap_err(),
            ValidationError::WindowsSeparator
        );
    }

    #[test]
    fn rejects_null_byte() {
        assert_eq!(
            VaultPath::parse("Notes/a\0.md").unwrap_err(),
            ValidationError::NullByte
        );
        assert_eq!(
            VaultPath::parse("Notes/a%00.md").unwrap_err(),
            ValidationError::NullByte
        );
    }

    #[test]
    fn rejects_invalid_percent_encoding() {
        for input in ["Notes/%GG.md", "Notes/%", "Notes/%F0%28%8C%28"] {
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
            "_haze_runtime/cache.json",
            "_haze_tmp/upload",
            "state/db.json",
            "logs/server.log",
            "trash/deleted.md",
            "Notes/upload.tmp",
            "Notes/upload.part",
            "Notes/.draft.swp",
        ] {
            let error = VaultPath::parse(input).unwrap_err();
            assert_eq!(error, ValidationError::RuntimePath, "input={input:?}");
        }
    }

    #[test]
    fn preserves_contract_safe_haze_paths() {
        let path = VaultPath::parse("_haze_conflicts/open/Projects/Haze/plan.md").unwrap();
        assert_eq!(path.as_str(), "_haze_conflicts/open/Projects/Haze/plan.md");
        assert!(!path.is_reserved_runtime_path());

        let outbox = VaultPath::parse("_haze_agent_outbox/draft.md").unwrap();
        assert_eq!(outbox.as_str(), "_haze_agent_outbox/draft.md");
        assert!(!outbox.is_reserved_runtime_path());
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
        assert!(serde_json::from_str::<VaultPath>("\"../a.md\"").is_err());
        assert!(serde_json::from_str::<VaultPath>("\"/etc/passwd\"").is_err());
    }
}
