//! Identifier newtypes shared across Haze Sync components.

use crate::ValidationError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

const MAX_IDENTIFIER_LEN: usize = 128;

/// Stable adapter identifier, for example `iphone-anna` or `worktree-adapter`.
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
        assert_eq!(adapter_id.to_string(), "iphone-anna");
    }

    #[test]
    fn revision_id_requires_prefix() {
        let revision_id = RevisionId::parse("rev_01JTEST").unwrap();
        assert_eq!(revision_id.to_string(), "rev_01JTEST");
        assert_eq!(
            RevisionId::parse("01JTEST").unwrap_err(),
            ValidationError::InvalidIdentifierPrefix
        );
    }

    #[test]
    fn operation_id_requires_prefix() {
        let operation_id = OperationId::parse("op_01JTEST").unwrap();
        assert_eq!(operation_id.to_string(), "op_01JTEST");
        assert_eq!(
            OperationId::parse("op_").unwrap_err(),
            ValidationError::InvalidIdentifierPrefix
        );
    }

    #[test]
    fn conflict_id_requires_prefix() {
        let conflict_id = ConflictId::parse("conf_01JTEST").unwrap();
        assert_eq!(conflict_id.to_string(), "conf_01JTEST");
    }

    #[test]
    fn identifiers_reject_unsafe_values() {
        assert_eq!(
            AdapterId::parse("bad/path").unwrap_err(),
            ValidationError::InvalidIdentifier
        );
        assert_eq!(
            AdapterId::parse("bad\0id").unwrap_err(),
            ValidationError::InvalidIdentifier
        );
    }

    #[test]
    fn serde_roundtrips() {
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
    }
}
