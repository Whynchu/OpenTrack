//! Read-only inspection of observed Cubase project container framing.
//!
//! This is not a project parser. It validates the outer header and reports
//! length-prefixed outer blocks observed in the installed templates.

use std::fmt;

/// Outer header found in the supplied `.cpr` templates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CprHeader {
    /// Four-byte container identifier found at offsets 8-11 (`NUND`).
    pub container_id: [u8; 4],
    /// Four-byte root identifier found at offsets 12-15 (`ROOT`).
    pub root_id: [u8; 4],
    /// Big-endian length field found at offsets 4-7.
    pub declared_length: u32,
    /// Whether `declared_length + 12` equals the observed file length.
    pub length_matches_file: bool,
}

/// Errors returned when the observed `.cpr` container header cannot be read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CprHeaderError {
    TooShort {
        actual: usize,
    },
    MissingRiffSignature,
    UnsupportedRoot {
        container_id: [u8; 4],
        root_id: [u8; 4],
    },
}

impl fmt::Display for CprHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { actual } => {
                write!(f, "CPR header needs 16 bytes; received {actual}")
            }
            Self::MissingRiffSignature => write!(f, "missing RIFF signature"),
            Self::UnsupportedRoot {
                container_id,
                root_id,
            } => write!(
                f,
                "unsupported CPR root {:?}/{:?}",
                String::from_utf8_lossy(container_id),
                String::from_utf8_lossy(root_id)
            ),
        }
    }
}

impl std::error::Error for CprHeaderError {}

/// An outer length-prefixed block in a CPR container.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CprBlock {
    /// The four raw tag bytes at the start of the block.
    pub tag: [u8; 4],
    /// Offset of the block tag in the source file.
    pub header_offset: usize,
    /// Offset of the payload immediately following the eight-byte block header.
    pub payload_offset: usize,
    /// Payload length declared by the block header.
    pub payload_length: usize,
    /// Offset immediately following the payload.
    pub end_offset: usize,
}

/// The observed CPR header and its bounded outer block sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CprStructure {
    pub header: CprHeader,
    pub blocks: Vec<CprBlock>,
}

/// Errors returned when the observed outer CPR block sequence is malformed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CprStructureError {
    Header(CprHeaderError),
    TruncatedBlockHeader {
        offset: usize,
        remaining: usize,
    },
    BlockLengthOverflow {
        offset: usize,
        declared_length: u32,
    },
    BlockOutOfBounds {
        offset: usize,
        declared_length: u32,
        file_length: usize,
    },
}

impl fmt::Display for CprStructureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header(error) => write!(f, "invalid CPR header: {error}"),
            Self::TruncatedBlockHeader { offset, remaining } => write!(
                f,
                "outer block at 0x{offset:X} needs 8 header bytes; only {remaining} remain"
            ),
            Self::BlockLengthOverflow {
                offset,
                declared_length,
            } => write!(
                f,
                "outer block at 0x{offset:X} declares an unrepresentable length {declared_length}"
            ),
            Self::BlockOutOfBounds {
                offset,
                declared_length,
                file_length,
            } => write!(
                f,
                "outer block at 0x{offset:X} declares {declared_length} payload bytes beyond the {file_length}-byte file"
            ),
        }
    }
}

impl std::error::Error for CprStructureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Header(error) => Some(error),
            _ => None,
        }
    }
}

impl CprHeader {
    /// Parses the 16-byte header shape observed in the installed templates.
    pub fn parse(bytes: &[u8]) -> Result<Self, CprHeaderError> {
        if bytes.len() < 16 {
            return Err(CprHeaderError::TooShort {
                actual: bytes.len(),
            });
        }
        if &bytes[0..4] != b"RIFF" {
            return Err(CprHeaderError::MissingRiffSignature);
        }

        let declared_length = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let container_id = [bytes[8], bytes[9], bytes[10], bytes[11]];
        let root_id = [bytes[12], bytes[13], bytes[14], bytes[15]];
        if container_id != *b"NUND" || root_id != *b"ROOT" {
            return Err(CprHeaderError::UnsupportedRoot {
                container_id,
                root_id,
            });
        }

        Ok(Self {
            container_id,
            root_id,
            declared_length,
            length_matches_file: u64::from(declared_length) + 12 == bytes.len() as u64,
        })
    }
}

impl CprStructure {
    /// Parses the observed outer block framing after the 12-byte RIFF/NUND header.
    ///
    /// Each observed block has a four-byte tag and a big-endian `u32` payload
    /// length. The payload begins immediately after the eight-byte block
    /// header; no alignment padding is applied. Tags and payload semantics are
    /// preserved as raw offsets and are not interpreted here.
    pub fn parse(bytes: &[u8]) -> Result<Self, CprStructureError> {
        let header = CprHeader::parse(bytes).map_err(CprStructureError::Header)?;
        let mut blocks = Vec::new();
        let mut offset = 12_usize;

        while offset < bytes.len() {
            let remaining = bytes.len() - offset;
            if remaining < 8 {
                return Err(CprStructureError::TruncatedBlockHeader { offset, remaining });
            }

            let tag = [
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ];
            let declared_length = u32::from_be_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            let payload_length = usize::try_from(declared_length).map_err(|_| {
                CprStructureError::BlockLengthOverflow {
                    offset,
                    declared_length,
                }
            })?;
            let payload_offset =
                offset
                    .checked_add(8)
                    .ok_or(CprStructureError::BlockLengthOverflow {
                        offset,
                        declared_length,
                    })?;
            let end_offset = payload_offset.checked_add(payload_length).ok_or(
                CprStructureError::BlockLengthOverflow {
                    offset,
                    declared_length,
                },
            )?;
            if end_offset > bytes.len() {
                return Err(CprStructureError::BlockOutOfBounds {
                    offset,
                    declared_length,
                    file_length: bytes.len(),
                });
            }

            blocks.push(CprBlock {
                tag,
                header_offset: offset,
                payload_offset,
                payload_length,
                end_offset,
            });
            offset = end_offset;
        }

        Ok(Self { header, blocks })
    }
}

/// Decodes a payload that is entirely a sequence of length-prefixed printable
/// ASCII strings. This shape is observed in the small ROOT label blocks of the
/// installed templates; the string names do not, by themselves, establish the
/// meaning of the paired ARCH data.
pub fn decode_root_label_strings(payload: &[u8]) -> Option<Vec<String>> {
    let mut labels = Vec::new();
    let mut offset = 0_usize;
    while offset < payload.len() {
        let length_bytes = payload.get(offset..offset.checked_add(4)?)?;
        let length = usize::try_from(u32::from_be_bytes([
            length_bytes[0],
            length_bytes[1],
            length_bytes[2],
            length_bytes[3],
        ]))
        .ok()?;
        offset = offset.checked_add(4)?;
        if length == 0 {
            return None;
        }
        let end = offset.checked_add(length)?;
        let value = payload.get(offset..end)?;
        if value.iter().any(|byte| !(0x20..=0x7e).contains(byte)) {
            return None;
        }
        labels.push(String::from_utf8(value.to_vec()).ok()?);
        offset = end;
    }
    (!labels.is_empty()).then_some(labels)
}

/// One contiguous printable ASCII run and its offset in the source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrintableAsciiRun {
    pub offset: usize,
    pub text: String,
}

/// Extracts printable ASCII runs and byte offsets in their original order.
///
/// These runs are investigation leads, not proof that a run is a serialized
/// field or a record boundary.
pub fn printable_ascii_runs_with_offsets(
    bytes: &[u8],
    minimum_length: usize,
) -> Vec<PrintableAsciiRun> {
    let mut runs = Vec::new();
    let mut run_start = None;

    for (offset, &byte) in bytes.iter().enumerate() {
        if (0x20..=0x7e).contains(&byte) {
            run_start.get_or_insert(offset);
        } else if let Some(start) = run_start.take() {
            if offset - start >= minimum_length {
                runs.push(PrintableAsciiRun {
                    offset: start,
                    text: String::from_utf8_lossy(&bytes[start..offset]).into_owned(),
                });
            }
        }
    }

    if let Some(start) = run_start
        && bytes.len() - start >= minimum_length
    {
        runs.push(PrintableAsciiRun {
            offset: start,
            text: String::from_utf8_lossy(&bytes[start..]).into_owned(),
        });
    }

    runs
}

/// Extracts printable ASCII runs from binary data in their original order.
pub fn printable_ascii_runs(bytes: &[u8], minimum_length: usize) -> Vec<String> {
    printable_ascii_runs_with_offsets(bytes, minimum_length)
        .into_iter()
        .map(|run| run.text)
        .collect()
}
