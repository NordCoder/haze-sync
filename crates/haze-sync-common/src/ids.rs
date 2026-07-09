//! Identifier newtypes shared across Haze Sync components.
//!
//! This module validates and carries identifiers. It deliberately does not mint
//! IDs, allocate sequences, or decide storage/runtime ownership.

use crate::ValidationError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

const MAX_IDENTIFIER_LEN: usize = 128;

/// Stable adapter identifier, for example `iphone-anna` or `worktree-adapter`.
///
/// Adapter IDs are configured names and therefore do not require a typed system
/// prefix. They still reject empty, overly long, null-byte, and unsupported
/// character input.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdapterId(String);

/// Immutable file revision identifier, for example `rev_01J...`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RevisionId(String);

/// Operation identifier, for example `op_01J...`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(String);

/// Conflict record identifier, for example `conf_01J...`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConflictId(String);

impl AdapterId {
    /// Validate and construct an adapter identifier.
    pub fn parse(input: &str) -> Result<Self, ValidationError> {
        validate_identifier(input, None)?;
        Ok(Self(input.to_owned()))
    }

    /// Borrow the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the identifier and return its string value.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl RevisionId {
    /// Validate and construct a revision identifier.
    ///
    /// Revision IDs must use the `rev_` prefix and include a non-empty suffix.
    pub fn parse(input: &str) -> Result<Self, ValidationError> {
        validate_identifier(input, Some("rev_"))?;
        Ok(Self(input.to_owned()))
    }

    /// Borrow the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the identifier and return its string value.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl OperationId {
    /// Validate and construct an operation identifier.
    ///
    /// Operation IDs must use the `op_` prefix and include a non-empty suffix.
    pub fn parse(input: &str) -> Result<Self, ValidationError> {
        validate_identifier(input, Some("op_"))?;
        Ok(Self(input.to_owned()))
    }

    /// Borrow the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the identifier and return its string value.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl ConflictId {
    /// Validate and construct a conflict identifier.
    ///
    /// Conflict IDs must use the `conf_` prefix and include a non-empty suffix.
    pub fn parse(input: &str) -> Result<Self, ValidationError> {
        validate_identifier(input, Some("conf_"))?;
        Ok(Self(input.to_owned()))
    }

    /// Borrow the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the identifier and return its string value.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

macro_rules! impl_identifier_traits {
    ($type_name:ident) => {
        impl AsRef<str> for $type_name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $type_name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $type_name {
            type Err = ValidationError;

            fn from_str(input: &str) -> Result<Self, Self::Err> {
                Self::parse(input)
            }
        }

        impl TryFrom<&str> for $type_name {
            type Error = ValidationError;

            fn try_from(input: &str) -> Result<Self, Self::Error> {
                Self::parse(input)
            }
        }

        impl Serialize for $type_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $type_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(&value).map_err(serde::de::Error::custom)
            }
        }
    };
}

impl_identifier_traits!(AdapterId);
impl_identifier_traits!(RevisionId);
impl_identifier_traits!(OperationId);
impl_identifier_traits!(ConflictId);

fn validate_identifier(input: &str, required_prefix: Option<&str>) -> Result<(), ValidationError> {
    if input.is_empty()
        || input.len() > MAX_IDENTIFIER_LEN
        || input.as_bytes().contains(&0)
        || !input.chars().all(is_identifier_char)
    {
        return Err(ValidationError::InvalidIdentifier);
    }

    if let Some(prefix) = required_prefix {
        if input.len() == prefix.len() || !input.starts_with(prefix) {
            return Err(ValidationError::InvalidIdentifierPrefix);
        }
    }

    Ok(())
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_id_parses_and_formats() {
        let adapter_id = AdapterId::parse("iphone-anna").unwrap();
        assert_eq!(adapter_id.as_str(), "iphone-anna");
        assert_eq!(adapter_id.as_ref(), "iphone-anna");
        assert_eq!(adapter_id.to_string(), "iphone-anna");
        assert_eq!(adapter_id.clone().into_string(), "iphone-anna");
        assert_eq!(AdapterId::from_str("iphone-anna").unwrap(), adapter_id);
        assert_eq!(AdapterId::try_from("iphone-anna").unwrap(), adapter_id);
    }

    #[test]
    fn revision_id_requires_prefix() {
        let revision_id = RevisionId::parse("rev_01JTEST").unwrap();
        assert_eq!(revision_id.to_string(), "rev_01JTEST");
        assert_eq!(revision_id.clone().into_string(), "rev_01JTEST");
        assert_eq!(
            RevisionId::parse("01JTEST").unwrap_err(),
            ValidationError::InvalidIdentifierPrefix
        );
        assert_eq!(
            RevisionId::parse("rev_").unwrap_err(),
            ValidationError::InvalidIdentifierPrefix
        );
    }

    #[test]
    fn operation_id_requires_prefix() {
        let operation_id = OperationId::parse("op_01JTEST").unwrap();
        assert_eq!(operation_id.to_string(), "op_01JTEST");
        assert_eq!(operation_id.clone().into_string(), "op_01JTEST");
        assert_eq!(
            OperationId::parse("op_").unwrap_err(),
            ValidationError::InvalidIdentifierPrefix
        );
    }

    #[test]
    fn conflict_id_requires_prefix() {
        let conflict_id = ConflictId::parse("conf_01JTEST").unwrap();
        assert_eq!(conflict_id.to_string(), "conf_01JTEST");
        assert_eq!(conflict_id.clone().into_string(), "conf_01JTEST");
        assert_eq!(
            ConflictId::parse("conf_").unwrap_err(),
            ValidationError::InvalidIdentifierPrefix
        );
    }

    #[test]
    fn typed_ids_reject_missing_wrong_or_case_mismatched_prefixes() {
        for input in ["01JTEST", "op_01JTEST", "conf_01JTEST", "REV_01JTEST"] {
            assert_eq!(
                RevisionId::parse(input).unwrap_err(),
                ValidationError::InvalidIdentifierPrefix,
                "input={input:?}"
            );
        }

        for input in ["01JTEST", "rev_01JTEST", "conf_01JTEST", "OP_01JTEST"] {
            assert_eq!(
                OperationId::parse(input).unwrap_err(),
                ValidationError::InvalidIdentifierPrefix,
                "input={input:?}"
            );
        }

        for input in ["01JTEST", "rev_01JTEST", "op_01JTEST", "CONF_01JTEST"] {
            assert_eq!(
                ConflictId::parse(input).unwrap_err(),
                ValidationError::InvalidIdentifierPrefix,
                "input={input:?}"
            );
        }
    }

    #[test]
    fn identifiers_accept_documented_safe_characters() {
        let adapter_id = AdapterId::parse("worktree-adapter_01.alpha").unwrap();
        assert_eq!(adapter_id.as_str(), "worktree-adapter_01.alpha");

        let revision_id = RevisionId::parse("rev_ABC-xyz.012_345").unwrap();
        assert_eq!(revision_id.as_str(), "rev_ABC-xyz.012_345");
        let operation_id = OperationId::parse("op_ABC-xyz.012_345").unwrap();
        assert_eq!(operation_id.as_str(), "op_ABC-xyz.012_345");
        let conflict_id = ConflictId::parse("conf_ABC-xyz.012_345").unwrap();
        assert_eq!(conflict_id.as_str(), "conf_ABC-xyz.012_345");
    }

    #[test]
    fn adapter_id_can_carry_configured_names_that_resemble_typed_ids() {
        for input in ["rev_01JTEST", "op_01JTEST", "conf_01JTEST"] {
            let adapter_id = AdapterId::parse(input).unwrap();
            assert_eq!(adapter_id.as_str(), input);
        }
    }

    #[test]
    fn identifiers_reject_unsafe_values() {
        let invalid_cases = [
            "",
            "bad/path",
            "bad id",
            "bad:id",
            "bad\0id",
            "ümlaut",
            "bad@id",
            "bad#id",
            "bad?id",
            "bad%id",
        ];

        for input in invalid_cases {
            assert_eq!(
                AdapterId::parse(input).unwrap_err(),
                ValidationError::InvalidIdentifier,
                "input={input:?}"
            );
        }
    }

    #[test]
    fn typed_ids_reject_unsafe_values_before_prefix_classification() {
        for input in ["rev_bad/id", "op_bad id", "conf_bad\0id"] {
            let error = if input.starts_with("rev_") {
                RevisionId::parse(input).unwrap_err()
            } else if input.starts_with("op_") {
                OperationId::parse(input).unwrap_err()
            } else {
                ConflictId::parse(input).unwrap_err()
            };
            assert_eq!(error, ValidationError::InvalidIdentifier, "input={input:?}");
        }
    }

    #[test]
    fn identifiers_enforce_total_maximum_length() {
        let adapter_max = "a".repeat(MAX_IDENTIFIER_LEN);
        let adapter_too_long = "a".repeat(MAX_IDENTIFIER_LEN + 1);
        assert!(AdapterId::parse(&adapter_max).is_ok());
        assert_eq!(
            AdapterId::parse(&adapter_too_long).unwrap_err(),
            ValidationError::InvalidIdentifier
        );

        let revision_max = format!("rev_{}", "a".repeat(MAX_IDENTIFIER_LEN - "rev_".len()));
        let revision_too_long = format!(
            "rev_{}",
            "a".repeat(MAX_IDENTIFIER_LEN + 1 - "rev_".len())
        );
        assert_eq!(revision_max.len(), MAX_IDENTIFIER_LEN);
        assert!(RevisionId::parse(&revision_max).is_ok());
        assert_eq!(
            RevisionId::parse(&revision_too_long).unwrap_err(),
            ValidationError::InvalidIdentifier
        );

        let operation_max = format!("op_{}", "a".repeat(MAX_IDENTIFIER_LEN - "op_".len()));
        let operation_too_long = format!(
            "op_{}",
            "a".repeat(MAX_IDENTIFIER_LEN + 1 - "op_".len())
        );
        assert_eq!(operation_max.len(), MAX_IDENTIFIER_LEN);
        assert!(OperationId::parse(&operation_max).is_ok());
        assert_eq!(
            OperationId::parse(&operation_too_long).unwrap_err(),
            ValidationError::InvalidIdentifier
        );

        let conflict_max = format!("conf_{}", "a".repeat(MAX_IDENTIFIER_LEN - "conf_".len()));
        let conflict_too_long = format!(
            "conf_{}",
            "a".repeat(MAX_IDENTIFIER_LEN + 1 - "conf_".len())
        );
        assert_eq!(conflict_max.len(), MAX_IDENTIFIER_LEN);
        assert!(ConflictId::parse(&conflict_max).is_ok());
        assert_eq!(
            ConflictId::parse(&conflict_too_long).unwrap_err(),
            ValidationError::InvalidIdentifier
        );
    }

    #[test]
    fn serde_roundtrips_all_identifier_types() {
        let adapter_id = AdapterId::parse("worktree-adapter").unwrap();
        let adapter_json = serde_json::to_string(&adapter_id).unwrap();
        assert_eq!(adapter_json, "\"worktree-adapter\"");
        assert_eq!(
            serde_json::from_str::<AdapterId>(&adapter_json).unwrap(),
            adapter_id
        );

        let revision_id = RevisionId::parse("rev_01JTEST").unwrap();
        let revision_json = serde_json::to_string(&revision_id).unwrap();
        assert_eq!(revision_json, "\"rev_01JTEST\"");
        assert_eq!(
            serde_json::from_str::<RevisionId>(&revision_json).unwrap(),
            revision_id
        );

        let operation_id = OperationId::parse("op_01JTEST").unwrap();
        let operation_json = serde_json::to_string(&operation_id).unwrap();
        assert_eq!(operation_json, "\"op_01JTEST\"");
        assert_eq!(
            serde_json::from_str::<OperationId>(&operation_json).unwrap(),
            operation_id
        );

        let conflict_id = ConflictId::parse("conf_01JTEST").unwrap();
        let conflict_json = serde_json::to_string(&conflict_id).unwrap();
        assert_eq!(conflict_json, "\"conf_01JTEST\"");
        assert_eq!(
            serde_json::from_str::<ConflictId>(&conflict_json).unwrap(),
            conflict_id
        );
    }

    #[test]
    fn serde_rejects_invalid_identifier_values() {
        assert!(serde_json::from_str::<AdapterId>("\"bad/path\"").is_err());
        assert!(serde_json::from_str::<RevisionId>("\"01JTEST\"").is_err());
        assert!(serde_json::from_str::<OperationId>("\"rev_01JTEST\"").is_err());
        assert!(serde_json::from_str::<ConflictId>("\"op_01JTEST\"").is_err());
    }
}
