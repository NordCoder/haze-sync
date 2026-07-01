//! Contract-local string wrappers for public DTO values.
//!
//! These wrappers avoid depending on sibling branch domain types. The Wave 1
//! fan-in phase can reconcile them with common value types when those are on the
//! integration branch.

use core::fmt;

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
}
