//! Contract-local string wrappers for public DTO values.
//!
//! The wrappers preserve the W1 API JSON wire shape while providing explicit
//! conversions to and from shared common domain/value types where those types are
//! already available.

use core::fmt;

use haze_sync_common::{
    AdapterId, ConflictId, ContentHash, OperationId, RevisionId, ValidationError, VaultPath,
};
use serde::{Deserialize, Serialize};

macro_rules! string_dto {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Creates the DTO wrapper from a string-like value.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns the wrapped string value.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

macro_rules! common_string_conversions {
    ($dto:ident, $common:ty, $parse:path) => {
        impl From<$common> for $dto {
            fn from(value: $common) -> Self {
                Self::new(value.into_string())
            }
        }

        impl TryFrom<$dto> for $common {
            type Error = ValidationError;

            fn try_from(value: $dto) -> Result<Self, Self::Error> {
                $parse(value.as_str())
            }
        }

        impl TryFrom<&$dto> for $common {
            type Error = ValidationError;

            fn try_from(value: &$dto) -> Result<Self, Self::Error> {
                $parse(value.as_str())
            }
        }
    };
}

string_dto! {
    /// Normalized vault-relative path as represented in public API JSON.
    VaultPathDto
}

string_dto! {
    /// Stable adapter identifier as represented in public API JSON.
    AdapterIdDto
}

string_dto! {
    /// File revision identifier as represented in public API JSON.
    RevisionIdDto
}

string_dto! {
    /// Conflict identifier as represented in public API JSON.
    ConflictIdDto
}

string_dto! {
    /// Tombstone identifier as represented in public API JSON.
    TombstoneIdDto
}

string_dto! {
    /// Operation identifier as represented in public API JSON.
    OperationIdDto
}

string_dto! {
    /// Content hash string, normally sha256:<hex>, as represented in public API JSON.
    ContentSha256Dto
}

string_dto! {
    /// UTC timestamp string as represented in public API JSON.
    TimestampDto
}

common_string_conversions!(VaultPathDto, VaultPath, VaultPath::parse);
common_string_conversions!(AdapterIdDto, AdapterId, AdapterId::parse);
common_string_conversions!(RevisionIdDto, RevisionId, RevisionId::parse);
common_string_conversions!(ConflictIdDto, ConflictId, ConflictId::parse);
common_string_conversions!(OperationIdDto, OperationId, OperationId::parse);

impl From<ContentHash> for ContentSha256Dto {
    fn from(value: ContentHash) -> Self {
        Self::new(value.to_string())
    }
}

impl TryFrom<ContentSha256Dto> for ContentHash {
    type Error = ValidationError;

    fn try_from(value: ContentSha256Dto) -> Result<Self, Self::Error> {
        ContentHash::parse(value.as_str())
    }
}

impl TryFrom<&ContentSha256Dto> for ContentHash {
    type Error = ValidationError;

    fn try_from(value: &ContentSha256Dto) -> Result<Self, Self::Error> {
        ContentHash::parse(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_wrappers_roundtrip_as_json_strings() {
        let path = VaultPathDto::from("Projects/Haze/plan.md");
        let json = serde_json::to_string(&path).unwrap();

        assert_eq!(json, "\"Projects/Haze/plan.md\"");
        assert_eq!(serde_json::from_str::<VaultPathDto>(&json).unwrap(), path);
    }

    #[test]
    fn common_domain_values_convert_without_changing_wire_json() {
        let path = VaultPath::parse("./Notes//a.md").unwrap();
        let adapter_id = AdapterId::parse("iphone-anna").unwrap();
        let revision_id = RevisionId::parse("rev_01JTEST").unwrap();
        let conflict_id = ConflictId::parse("conf_01JTEST").unwrap();
        let operation_id = OperationId::parse("op_01JTEST").unwrap();
        let hash = ContentHash::parse(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();

        let path_dto = VaultPathDto::from(path.clone());
        let adapter_dto = AdapterIdDto::from(adapter_id.clone());
        let revision_dto = RevisionIdDto::from(revision_id.clone());
        let conflict_dto = ConflictIdDto::from(conflict_id.clone());
        let operation_dto = OperationIdDto::from(operation_id.clone());
        let hash_dto = ContentSha256Dto::from(hash);

        assert_eq!(path_dto.as_str(), "Notes/a.md");
        assert_eq!(VaultPath::try_from(&path_dto).unwrap(), path);
        assert_eq!(AdapterId::try_from(&adapter_dto).unwrap(), adapter_id);
        assert_eq!(RevisionId::try_from(&revision_dto).unwrap(), revision_id);
        assert_eq!(ConflictId::try_from(&conflict_dto).unwrap(), conflict_id);
        assert_eq!(OperationId::try_from(&operation_dto).unwrap(), operation_id);
        assert_eq!(ContentHash::try_from(&hash_dto).unwrap(), hash);

        assert_eq!(
            serde_json::to_string(&hash_dto).unwrap(),
            "\"sha256:0000000000000000000000000000000000000000000000000000000000000000\""
        );
    }
}
