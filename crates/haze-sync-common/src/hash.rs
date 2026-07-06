//! SHA-256 content hash representation primitives.
//!
//! This module validates, parses, and formats SHA-256 values. It deliberately
//! does not hash file bytes, read content, or own object-store behavior.

use crate::ValidationError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

const SHA256_BYTES: usize = 32;
const SHA256_HEX_LEN: usize = SHA256_BYTES * 2;
const SHA256_PREFIX: &str = concat!("sha", "256:");

/// Validated SHA-256 digest bytes.
///
/// `Sha256` accepts either plain 64-character hexadecimal input or canonical
/// `sha256:<hex>` input. Formatting and serialization always emit the canonical
/// lowercase prefixed form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sha256([u8; SHA256_BYTES]);

/// Shared content hash representation used by components that exchange blob identity.
pub type ContentHash = Sha256;

impl Sha256 {
    /// Construct a digest from already validated raw SHA-256 bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; SHA256_BYTES]) -> Self {
        Self(bytes)
    }

    /// Parse plain 64-character hex or canonical `sha256:<hex>` input.
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

    /// Borrow the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; SHA256_BYTES] {
        &self.0
    }

    /// Consume the digest and return the raw bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; SHA256_BYTES] {
        self.0
    }

    /// Return the lowercase 64-character hex digest without the `sha256:` prefix.
    #[must_use]
    pub fn as_hex(&self) -> String {
        let mut output = String::with_capacity(SHA256_HEX_LEN);
        for byte in self.0 {
            output.push(hex_char(byte >> 4));
            output.push(hex_char(byte & 0x0f));
        }
        output
    }

    /// Return the canonical `sha256:<lowercase-hex>` wire representation.
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

    fn repeated(ch: &str) -> String {
        ch.repeat(SHA256_HEX_LEN)
    }

    #[test]
    fn parses_plain_hex_and_formats_prefixed_canonical() {
        let zero_hex = repeated("0");
        let hash = Sha256::parse(&zero_hex).unwrap();
        assert_eq!(hash.as_hex(), zero_hex);
        assert_eq!(hash.to_string(), [SHA256_PREFIX, &zero_hex].concat());
    }

    #[test]
    fn parses_prefixed_hash_and_normalizes_case() {
        let upper_hex = repeated("A");
        let upper_canonical = repeated("a");
        let input = [SHA256_PREFIX, &upper_hex].concat();
        let hash = Sha256::parse(&input).unwrap();
        assert_eq!(hash.as_hex(), upper_canonical);
        assert_eq!(hash.to_string(), [SHA256_PREFIX, &upper_canonical].concat());
    }

    #[test]
    fn from_str_and_try_from_share_parse_contract() {
        let hex = repeated("1");
        let from_str = Sha256::from_str(&hex).unwrap();
        let try_from = Sha256::try_from(hex.as_str()).unwrap();
        assert_eq!(from_str, try_from);
        assert_eq!(from_str.to_string(), format!("{SHA256_PREFIX}{hex}"));
    }

    #[test]
    fn byte_access_preserves_digest_bytes() {
        let bytes = [0xab; SHA256_BYTES];
        let hash = Sha256::from_bytes(bytes);

        assert_eq!(hash.as_bytes(), &bytes);
        assert_eq!(hash.into_bytes(), bytes);
        assert_eq!(hash.as_hex(), "ab".repeat(SHA256_BYTES));
    }

    #[test]
    fn content_hash_alias_uses_same_wire_representation() {
        let hash: ContentHash = Sha256::from_bytes([0; SHA256_BYTES]);

        assert_eq!(
            serde_json::to_string(&hash).unwrap(),
            format!("\"{}{}\"", SHA256_PREFIX, repeated("0"))
        );
    }

    #[test]
    fn rejects_invalid_hash_length() {
        assert_eq!(
            Sha256::parse("abc").unwrap_err(),
            ValidationError::InvalidHashLength
        );
        assert_eq!(
            Sha256::parse(&format!("{SHA256_PREFIX}{}", repeated("0") + "0")).unwrap_err(),
            ValidationError::InvalidHashLength
        );
    }

    #[test]
    fn rejects_invalid_hash_characters() {
        let invalid = format!("{}z", &repeated("0")[..63]);
        assert_eq!(
            Sha256::parse(&invalid).unwrap_err(),
            ValidationError::InvalidHashCharacter
        );
    }

    #[test]
    fn serde_roundtrip_uses_prefixed_form() {
        let zero_hex = repeated("0");
        let hash = Sha256::parse(&zero_hex).unwrap();
        let json = serde_json::to_string(&hash).unwrap();
        assert_eq!(json, format!("\"{}{}\"", SHA256_PREFIX, zero_hex));
        let decoded: Sha256 = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, hash);
    }

    #[test]
    fn serde_rejects_non_canonical_values_safely() {
        assert!(serde_json::from_str::<Sha256>("\"not-a-hash\"").is_err());
    }
}
