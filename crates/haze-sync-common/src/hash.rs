//! Content-addressed hash primitives.

use crate::ValidationError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

const SHA256_BYTES: usize = 32;
const SHA256_HEX_LEN: usize = SHA256_BYTES * 2;
const SHA256_PREFIX: &str = "sha256:";

/// A SHA-256 content hash.
///
/// Parsing accepts canonical 64-character hexadecimal values and the
/// `sha256:<hex>` form required by the Core API headers. Serialization and
/// display use the deterministic API form `sha256:<lowercase-hex>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sha256([u8; SHA256_BYTES]);

/// Alias used when call sites want a domain-oriented content hash name.
pub type ContentHash = Sha256;

impl Sha256 {
    /// Construct a hash from raw SHA-256 bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; SHA256_BYTES]) -> Self {
        Self(bytes)
    }

    /// Parse a SHA-256 hash from hex or `sha256:<hex>` input.
    pub fn parse(input: &str) -> Result<Self, ValidationError> {
        let hex = input.strip_prefix(SHA256_PREFIX).unwrap_or(input);
        if hex.len() != SHA256_HEX_LEN {
            return Err(ValidationError::InvalidHashLength);
        }

        let mut bytes = [0_u8; SHA256_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let hi = hex_nibble(hex.as_bytes()[index * 2])?;
            let lo = hex_nibble(hex.as_bytes()[index * 2 + 1])?;
            *byte = (hi << 4) | lo;
        }

        Ok(Self(bytes))
    }

    /// Borrow the raw SHA-256 bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; SHA256_BYTES] {
        &self.0
    }

    /// Return the raw SHA-256 bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; SHA256_BYTES] {
        self.0
    }

    /// Return canonical lowercase hexadecimal without the `sha256:` prefix.
    #[must_use]
    pub fn as_hex(&self) -> String {
        let mut output = String::with_capacity(SHA256_HEX_LEN);
        for byte in self.0 {
            output.push(hex_char(byte >> 4));
            output.push(hex_char(byte & 0x0f));
        }
        output
    }

    /// Return the Core API header/JSON form, `sha256:<lowercase-hex>`.
    #[must_use]
    pub fn to_prefixed_string(&self) -> String {
        let mut output = String::with_capacity(SHA256_PREFIX.len() + SHA256_HEX_LEN);
        output.push_str(SHA256_PREFIX);
        output.push_str(&self.as_hex());
        output
    }
}

impl fmt::Display for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_prefixed_string())
    }
}

impl FromStr for Sha256 {
    type Err = ValidationError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

impl TryFrom<&str> for Sha256 {
    type Error = ValidationError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::parse(input)
    }
}

impl Serialize for Sha256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_prefixed_string())
    }
}

impl<'de> Deserialize<'de> for Sha256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

fn hex_nibble(byte: u8) -> Result<u8, ValidationError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ValidationError::InvalidHashCharacter),
    }
}

fn hex_char(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + (nibble - 10)),
        _ => unreachable!("nibble is masked to four bits"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZERO_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    const UPPER_HEX: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const UPPER_CANONICAL: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn parses_plain_hex_and_formats_prefixed_canonical() {
        let hash = Sha256::parse(ZERO_HEX).unwrap();
        assert_eq!(hash.as_hex(), ZERO_HEX);
        assert_eq!(hash.to_string(), format!("sha256:{ZERO_HEX}"));
    }

    #[test]
    fn parses_prefixed_hash_and_normalizes_case() {
        let hash = Sha256::parse(&format!("sha256:{UPPER_HEX}")).unwrap();
        assert_eq!(hash.as_hex(), UPPER_CANONICAL);
        assert_eq!(hash.to_string(), format!("sha256:{UPPER_CANONICAL}"));
    }

    #[test]
    fn rejects_invalid_hash_length() {
        assert_eq!(
            Sha256::parse("abc").unwrap_err(),
            ValidationError::InvalidHashLength
        );
    }

    #[test]
    fn rejects_invalid_hash_characters() {
        let invalid = format!("{}z", &ZERO_HEX[..63]);
        assert_eq!(
            Sha256::parse(&invalid).unwrap_err(),
            ValidationError::InvalidHashCharacter
        );
    }

    #[test]
    fn serde_roundtrip_uses_prefixed_form() {
        let hash = Sha256::parse(ZERO_HEX).unwrap();
        let json = serde_json::to_string(&hash).unwrap();
        assert_eq!(json, format!("\"sha256:{ZERO_HEX}\""));
        let decoded: Sha256 = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, hash);
    }
}
