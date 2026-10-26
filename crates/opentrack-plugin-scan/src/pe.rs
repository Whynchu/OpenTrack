// SPDX-License-Identifier: Apache-2.0

//! Minimal, allocation-bounded PE machine reading.
//!
//! The scanner opens untrusted `.vst3` images. Before any LoadLibrary call, it
//! reads the DOS/COFF headers and rejects images that do not match the x64 host.
//! This module intentionally parses only the fields required for that decision;
//! it is not a general PE library and performs no code loading.
//!
//! Reference: `docs/plugin-hosting-plan.md` section 4.12 ("Read the PE header
//! before loading. Reject x86/ARM64...") and the probe findings that x86 modules
//! must fail before `LoadLibrary`.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use opentrack_plugins::PluginArchitecture;

/// Minimum DOS header length needed to locate the PE header.
const DOS_HEADER_LEN: usize = 0x40;

/// Maximum number of header bytes read from a module.
///
/// Normal images place the PE header well inside this bound. A larger DOS stub
/// is reported as an invalid/truncated header instead of causing an unbounded
/// read.
const MAX_PE_HEADER_BYTES: u64 = 64 * 1024;

const IMAGE_FILE_MACHINE_I386: u16 = 0x014c;
const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
const IMAGE_FILE_MACHINE_ARM64: u16 = 0xaa64;

/// COFF machine type read from a PE header.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PeMachine(u16);

impl PeMachine {
    /// 32-bit x86 (`IMAGE_FILE_MACHINE_I386`, `0x014c`).
    pub const X86: Self = Self(IMAGE_FILE_MACHINE_I386);
    /// 64-bit x86 (`IMAGE_FILE_MACHINE_AMD64`, `0x8664`).
    pub const X64: Self = Self(IMAGE_FILE_MACHINE_AMD64);
    /// 64-bit ARM (`IMAGE_FILE_MACHINE_ARM64`, `0xaa64`).
    pub const ARM64: Self = Self(IMAGE_FILE_MACHINE_ARM64);

    /// Wraps a raw machine value.
    pub const fn from_raw(value: u16) -> Self {
        Self(value)
    }

    /// Raw COFF machine value.
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Maps the machine value to the catalog architecture vocabulary.
    pub const fn architecture(self) -> PluginArchitecture {
        classify_pe_machine(self.0)
    }
}

impl fmt::Display for PeMachine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:04x}", self.0)
    }
}

/// Classifies a raw COFF machine value.
///
/// Unknown values map to [`PluginArchitecture::Other`]; they are still reported
/// with their raw value and never loaded by the x64 host.
pub const fn classify_pe_machine(machine: u16) -> PluginArchitecture {
    match machine {
        IMAGE_FILE_MACHINE_AMD64 => PluginArchitecture::X64,
        IMAGE_FILE_MACHINE_I386 => PluginArchitecture::X86,
        IMAGE_FILE_MACHINE_ARM64 => PluginArchitecture::Arm64,
        _ => PluginArchitecture::Other,
    }
}

/// Parses the machine type from an in-memory PE header.
pub fn parse_pe_machine(bytes: &[u8]) -> Result<PeMachine, PeHeaderError> {
    if bytes.len() < DOS_HEADER_LEN {
        return Err(PeHeaderError::TooShort {
            needed: DOS_HEADER_LEN,
            found: bytes.len(),
        });
    }
    if &bytes[..2] != b"MZ" {
        return Err(PeHeaderError::MissingDosMagic);
    }

    let pe_offset = u32::from_le_bytes(
        bytes[0x3c..0x40]
            .try_into()
            .expect("four-byte PE offset slice"),
    ) as usize;
    let signature_end = pe_offset
        .checked_add(6)
        .ok_or(PeHeaderError::InvalidPeOffset { offset: pe_offset })?;

    if bytes.len() < signature_end {
        return Err(PeHeaderError::TooShort {
            needed: signature_end,
            found: bytes.len(),
        });
    }
    if &bytes[pe_offset..pe_offset + 4] != b"PE\0\0" {
        return Err(PeHeaderError::MissingPeSignature { offset: pe_offset });
    }

    let machine = u16::from_le_bytes(
        bytes[pe_offset + 4..pe_offset + 6]
            .try_into()
            .expect("two-byte machine slice"),
    );
    Ok(PeMachine::from_raw(machine))
}

/// Reads the PE machine type from a file, capped at 64 KiB of header bytes.
pub fn read_pe_machine_from_file(path: &Path) -> Result<PeMachine, PeFileError> {
    let file = File::open(path).map_err(PeFileError::Io)?;
    let mut header = Vec::new();
    file.take(MAX_PE_HEADER_BYTES)
        .read_to_end(&mut header)
        .map_err(PeFileError::Io)?;
    parse_pe_machine(&header).map_err(PeFileError::Header)
}

/// Error while locating or validating the PE header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PeHeaderError {
    /// File ended before the required header bytes were available.
    TooShort {
        /// Required byte count.
        needed: usize,
        /// Available byte count.
        found: usize,
    },
    /// The DOS header does not begin with `MZ`.
    MissingDosMagic,
    /// The 32-bit PE offset could not be advanced by a header signature.
    InvalidPeOffset {
        /// Invalid offset value.
        offset: usize,
    },
    /// The offset does not point at `PE\0\0`.
    MissingPeSignature {
        /// Offset containing the invalid signature.
        offset: usize,
    },
}

impl fmt::Display for PeHeaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { needed, found } => write!(
                formatter,
                "truncated PE header: need {needed} bytes, found {found}"
            ),
            Self::MissingDosMagic => formatter.write_str("missing DOS MZ signature"),
            Self::InvalidPeOffset { offset } => {
                write!(formatter, "invalid PE header offset 0x{offset:x}")
            }
            Self::MissingPeSignature { offset } => {
                write!(formatter, "missing PE signature at 0x{offset:x}")
            }
        }
    }
}

impl std::error::Error for PeHeaderError {}

/// Error while reading or validating a module image.
#[derive(Debug)]
pub enum PeFileError {
    /// The file could not be opened or read.
    Io(std::io::Error),
    /// The bytes are not a supported PE header.
    Header(PeHeaderError),
}

impl fmt::Display for PeFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "cannot read module: {error}"),
            Self::Header(error) => write!(formatter, "invalid module: {error}"),
        }
    }
}

impl std::error::Error for PeFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Header(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    fn pe_header(machine: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x80];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        bytes[0x40..0x44].copy_from_slice(b"PE\0\0");
        bytes[0x44..0x46].copy_from_slice(&machine.to_le_bytes());
        bytes
    }

    #[test]
    fn classifies_known_machine_types() {
        assert_eq!(classify_pe_machine(0x8664), PluginArchitecture::X64);
        assert_eq!(classify_pe_machine(0x014c), PluginArchitecture::X86);
        assert_eq!(classify_pe_machine(0xaa64), PluginArchitecture::Arm64);
        assert_eq!(classify_pe_machine(0x01c4), PluginArchitecture::Other);
        assert_eq!(classify_pe_machine(0), PluginArchitecture::Other);
    }

    #[test]
    fn parses_synthetic_pe_headers() {
        for (raw, expected) in [
            (0x8664, PeMachine::X64),
            (0x014c, PeMachine::X86),
            (0xaa64, PeMachine::ARM64),
        ] {
            let parsed = parse_pe_machine(&pe_header(raw)).expect("synthetic PE parses");
            assert_eq!(parsed, expected);
            assert_eq!(parsed.raw(), raw);
        }
    }

    #[test]
    fn rejects_malformed_headers() {
        assert_eq!(
            parse_pe_machine(b"MZ"),
            Err(PeHeaderError::TooShort {
                needed: DOS_HEADER_LEN,
                found: 2,
            })
        );

        let mut bad_magic = pe_header(0x8664);
        bad_magic[0] = b'X';
        assert_eq!(
            parse_pe_machine(&bad_magic),
            Err(PeHeaderError::MissingDosMagic)
        );

        let mut bad_signature = pe_header(0x8664);
        bad_signature[0x40] = b'X';
        assert_eq!(
            parse_pe_machine(&bad_signature),
            Err(PeHeaderError::MissingPeSignature { offset: 0x40 })
        );

        let mut short_machine = pe_header(0x8664);
        short_machine.truncate(0x44);
        assert_eq!(
            parse_pe_machine(&short_machine),
            Err(PeHeaderError::TooShort {
                needed: 0x46,
                found: 0x44,
            })
        );
    }

    #[test]
    fn reads_machine_from_file() {
        let temp = TempDir::new("pe-file");
        let module = temp.path().join("image.vst3");
        std::fs::write(&module, pe_header(0x8664)).expect("write fixture");

        assert_eq!(
            read_pe_machine_from_file(&module)
                .expect("fixture header parses")
                .architecture(),
            PluginArchitecture::X64
        );

        let text = temp.path().join("broken.vst3");
        std::fs::write(&text, b"not a PE image").expect("write malformed fixture");
        assert!(matches!(
            read_pe_machine_from_file(&text),
            Err(PeFileError::Header(_))
        ));
    }
}
