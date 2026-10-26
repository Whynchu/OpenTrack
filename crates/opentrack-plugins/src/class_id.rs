// SPDX-License-Identifier: Apache-2.0

//! Stable plug-in class identity.
//!
//! A plug-in class ID is 16 opaque bytes. VST3 transport is one producer of
//! such IDs, but this type is deliberately format-neutral and carries no ABI
//! meaning. The canonical text form is 32 lowercase hexadecimal digits. Bytes
//! are rendered in storage order; no GUID endianness reinterpretation is done.
//! The format layer must preserve its native byte order when filling the ID.
//!
//! Deserialization accepts the canonical form, uppercase hex, hyphenated
//! GUID-style text, and a single surrounding pair of braces. Serialization
//! always emits the canonical bare lowercase form so cache files are stable.

use std::fmt;
use std::fmt::Write as _;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use serde::de::Error as _;

/// Number of bytes in a plug-in class ID.
pub const CLASS_ID_BYTES: usize = 16;

/// Number of hexadecimal characters in the canonical class ID text form.
pub const CLASS_ID_HEX_LEN: usize = CLASS_ID_BYTES * 2;

/// Opaque 16-byte identity for a plug-in class.
///
/// Identity is the tuple `(PluginFormat, PluginClassId)`. A module path is a
/// hint that can change after an update, move, or duplicate install.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PluginClassId([u8; CLASS_ID_BYTES]);

impl PluginClassId {
    /// All-zero class ID.
    pub const ZERO: Self = Self([0; CLASS_ID_BYTES]);

    /// Creates an ID from its raw bytes.
    pub const fn from_bytes(bytes: [u8; CLASS_ID_BYTES]) -> Self {
        Self(bytes)
    }

    /// Returns the raw bytes.
    pub const fn as_bytes(&self) -> &[u8; CLASS_ID_BYTES] {
        &self.0
    }

    /// Consumes the ID and returns its raw bytes.
    pub const fn into_bytes(self) -> [u8; CLASS_ID_BYTES] {
        self.0
    }

    /// Returns true when every byte is zero.
    pub fn is_zero(self) -> bool {
        self.0 == [0; CLASS_ID_BYTES]
    }

    /// Returns the canonical 32-character lowercase hexadecimal form.
    pub fn to_hex(self) -> String {
        let mut output = String::with_capacity(CLASS_ID_HEX_LEN);
        for byte in self.0 {
            // Writing into a String cannot fail.
            let _ = write!(output, "{byte:02x}");
        }
        output
    }

    /// Parses a class ID from text.
    ///
    /// Accepted forms are 32 hex digits, optional hyphens between groups, and
    /// one optional pair of surrounding braces. Whitespace around the value is
    /// ignored; whitespace inside it is rejected.
    pub fn from_hex(text: &str) -> Result<Self, ClassIdParseError> {
        let trimmed = text.trim();
        let body = match trimmed.strip_prefix('{') {
            Some(rest) => rest
                .strip_suffix('}')
                .ok_or(ClassIdParseError::UnbalancedBraces)?,
            None if trimmed.ends_with('}') => {
                return Err(ClassIdParseError::UnbalancedBraces);
            }
            None => trimmed,
        };

        let mut bytes = [0u8; CLASS_ID_BYTES];
        let mut byte_index = 0usize;
        let mut high_nibble = None;
        let mut digits = 0usize;

        for character in body.chars() {
            if character == '-' {
                continue;
            }
            let digit = character
                .to_digit(16)
                .ok_or(ClassIdParseError::InvalidCharacter { character })?;
            digits += 1;
            match high_nibble.take() {
                Some(high) => {
                    if byte_index >= CLASS_ID_BYTES {
                        return Err(ClassIdParseError::InvalidLength { digits });
                    }
                    bytes[byte_index] = ((high as u8) << 4) | digit as u8;
                    byte_index += 1;
                }
                None => high_nibble = Some(digit as u8),
            }
        }

        if high_nibble.is_some() {
            return Err(ClassIdParseError::InvalidLength { digits });
        }
        if byte_index != CLASS_ID_BYTES {
            return Err(ClassIdParseError::InvalidLength { digits });
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for PluginClassId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl fmt::Debug for PluginClassId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "PluginClassId({})", self.to_hex())
    }
}

impl FromStr for PluginClassId {
    type Err = ClassIdParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::from_hex(text)
    }
}

impl From<[u8; CLASS_ID_BYTES]> for PluginClassId {
    fn from(bytes: [u8; CLASS_ID_BYTES]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl Serialize for PluginClassId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for PluginClassId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Self::from_hex(&text).map_err(D::Error::custom)
    }
}

/// Error returned when class ID text cannot be parsed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassIdParseError {
    /// Wrong number of hexadecimal digits.
    InvalidLength {
        /// Number of hex digits found, excluding hyphens.
        digits: usize,
    },
    /// A character other than a hex digit or separator was found.
    InvalidCharacter {
        /// The rejected character.
        character: char,
    },
    /// One brace was present without its matching partner.
    UnbalancedBraces,
}

impl fmt::Display for ClassIdParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { digits } => write!(
                formatter,
                "class ID must contain exactly {CLASS_ID_HEX_LEN} hex digits, found {digits}"
            ),
            Self::InvalidCharacter { character } => {
                write!(formatter, "invalid class ID character {character:?}")
            }
            Self::UnbalancedBraces => formatter.write_str("class ID has an unmatched brace"),
        }
    }
}

impl std::error::Error for ClassIdParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> [u8; CLASS_ID_BYTES] {
        [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ]
    }

    #[test]
    fn hex_round_trip_is_canonical() {
        let id = PluginClassId::from_bytes(sample());
        assert_eq!(id.to_hex(), "0123456789abcdeffedcba9876543210");
        assert_eq!(PluginClassId::from_hex(&id.to_hex()), Ok(id));
    }

    #[test]
    fn parser_accepts_guid_forms() {
        let expected = PluginClassId::from_bytes(sample());
        for text in [
            "0123456789ABCDEFFEDCBA9876543210",
            "01234567-89ab-cdef-fedc-ba9876543210",
            "{01234567-89ab-cdef-fedc-ba9876543210}",
            "{0123456789abcdeffedcba9876543210}",
            "  0123456789abcdeffedcba9876543210  ",
        ] {
            assert_eq!(PluginClassId::from_hex(text), Ok(expected), "{text}");
        }
    }

    #[test]
    fn parser_rejects_bad_text() {
        assert_eq!(
            PluginClassId::from_hex("0123"),
            Err(ClassIdParseError::InvalidLength { digits: 4 })
        );
        assert_eq!(
            PluginClassId::from_hex("0123456789abcdeffedcba987654321"),
            Err(ClassIdParseError::InvalidLength { digits: 31 })
        );
        assert_eq!(
            PluginClassId::from_hex("0123456789abcdeffedcba987654321g"),
            Err(ClassIdParseError::InvalidCharacter { character: 'g' })
        );
        assert_eq!(
            PluginClassId::from_hex("{0123456789abcdeffedcba9876543210"),
            Err(ClassIdParseError::UnbalancedBraces)
        );
        assert_eq!(
            PluginClassId::from_hex("0123456789abcdeffedcba9876543210}"),
            Err(ClassIdParseError::UnbalancedBraces)
        );
    }

    #[test]
    fn serialized_identity_is_hex_text() {
        let id = PluginClassId::from_bytes(sample());
        let json = serde_json::to_string(&id).expect("class ID serializes");
        assert_eq!(json, "\"0123456789abcdeffedcba9876543210\"");
        let parsed: PluginClassId = serde_json::from_str(&json).expect("class ID deserializes");
        assert_eq!(parsed, id);
    }
}
