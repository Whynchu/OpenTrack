// SPDX-License-Identifier: Apache-2.0

//! Small, bounded architecture probes for native VST3 module images.
//!
//! This reads only the executable headers needed to decide whether a module
//! can run in this scanner. It does not map or execute plug-in code.

use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use opentrack_plugins::PluginArchitecture;

use crate::pe::{PeFileError, read_pe_machine_from_file};

const MAX_HEADER_BYTES: usize = 4 * 1024;
const MAX_FAT_ARCHITECTURES: usize = 64;

/// Error while reading or identifying a native module image.
#[derive(Debug)]
pub enum ModuleImageError {
    /// The module could not be read.
    Io(io::Error),
    /// The module has a malformed or unrecognized executable header.
    Invalid(String),
}

impl fmt::Display for ModuleImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "cannot read module: {error}"),
            Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ModuleImageError {}

/// Executable container format found at the module path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleImageFormat {
    /// Windows Portable Executable.
    Pe,
    /// Linux Executable and Linkable Format.
    Elf,
    /// macOS Mach-O image or universal binary.
    MachO,
}

impl ModuleImageFormat {
    /// True when this executable format belongs to the current operating system.
    pub const fn is_host_compatible(self) -> bool {
        match self {
            Self::Pe => cfg!(windows),
            Self::Elf => cfg!(target_os = "linux"),
            Self::MachO => cfg!(target_os = "macos"),
        }
    }
}

impl fmt::Display for ModuleImageFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Pe => "PE",
            Self::Elf => "ELF",
            Self::MachO => "Mach-O",
        })
    }
}

/// Native image format and CPU architecture read from a module header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleImageInfo {
    /// Executable container format.
    pub format: ModuleImageFormat,
    /// CPU architecture or selected universal-binary slice.
    pub architecture: PluginArchitecture,
    /// Raw COFF machine ID, present only for PE images.
    pub pe_machine: Option<u16>,
}

/// Reads the native image format, architecture, and optional raw COFF machine ID.
/// Universal Mach-O binaries report the slice matching the current process
/// when present; otherwise they report their first recognized slice.
pub fn read_module_image(path: &Path) -> Result<ModuleImageInfo, ModuleImageError> {
    let mut file = File::open(path).map_err(ModuleImageError::Io)?;
    let mut bytes = Vec::with_capacity(MAX_HEADER_BYTES);
    file.by_ref()
        .take(MAX_HEADER_BYTES as u64)
        .read_to_end(&mut bytes)
        .map_err(ModuleImageError::Io)?;

    if bytes.starts_with(b"MZ") {
        return match read_pe_machine_from_file(path) {
            Ok(machine) => Ok(ModuleImageInfo {
                format: ModuleImageFormat::Pe,
                architecture: machine.architecture(),
                pe_machine: Some(machine.raw()),
            }),
            Err(PeFileError::Io(error)) => Err(ModuleImageError::Io(error)),
            Err(PeFileError::Header(error)) => Err(ModuleImageError::Invalid(error.to_string())),
        };
    }
    if bytes.starts_with(b"\x7fELF") {
        return parse_elf_architecture(&bytes).map(|architecture| ModuleImageInfo {
            format: ModuleImageFormat::Elf,
            architecture,
            pe_machine: None,
        });
    }
    if is_macho(&bytes) {
        return parse_macho_architecture(&bytes).map(|architecture| ModuleImageInfo {
            format: ModuleImageFormat::MachO,
            architecture,
            pe_machine: None,
        });
    }

    Err(ModuleImageError::Invalid(
        "unrecognized executable format (expected PE, ELF, or Mach-O)".to_owned(),
    ))
}

fn parse_elf_architecture(bytes: &[u8]) -> Result<PluginArchitecture, ModuleImageError> {
    if bytes.len() < 20 || !matches!(bytes[4], 1 | 2) || !matches!(bytes[5], 1 | 2) {
        return Err(ModuleImageError::Invalid(
            "truncated or invalid ELF header".to_owned(),
        ));
    }
    let machine = match bytes[5] {
        1 => u16::from_le_bytes([bytes[18], bytes[19]]),
        2 => u16::from_be_bytes([bytes[18], bytes[19]]),
        _ => {
            return Err(ModuleImageError::Invalid(
                "ELF header has an unsupported byte order".to_owned(),
            ));
        }
    };
    Ok(match machine {
        3 => PluginArchitecture::X86,
        62 => PluginArchitecture::X64,
        183 => PluginArchitecture::Arm64,
        _ => PluginArchitecture::Other,
    })
}

fn is_macho(bytes: &[u8]) -> bool {
    let Some(magic) = bytes.get(..4) else {
        return false;
    };
    magic == b"\xfe\xed\xfa\xce"
        || magic == b"\xce\xfa\xed\xfe"
        || magic == b"\xfe\xed\xfa\xcf"
        || magic == b"\xcf\xfa\xed\xfe"
        || magic == b"\xca\xfe\xba\xbe"
        || magic == b"\xbe\xba\xfe\xca"
        || magic == b"\xca\xfe\xba\xbf"
        || magic == b"\xbf\xba\xfe\xca"
}

fn parse_macho_architecture(bytes: &[u8]) -> Result<PluginArchitecture, ModuleImageError> {
    let magic = bytes
        .get(..4)
        .ok_or_else(|| ModuleImageError::Invalid("truncated Mach-O header".to_owned()))?;
    let magic_be = u32::from_be_bytes(magic.try_into().expect("four-byte magic"));
    let architectures = match magic_be {
        0xcafebabe | 0xcafebabf | 0xbebafeca | 0xbfbafeca => {
            let little_endian = matches!(magic_be, 0xbebafeca | 0xbfbafeca);
            let wide_entries = matches!(magic_be, 0xcafebabf | 0xbfbafeca);
            let count_bytes = bytes.get(4..8).ok_or_else(|| {
                ModuleImageError::Invalid("truncated universal Mach-O header".to_owned())
            })?;
            let count = read_u32(count_bytes, little_endian) as usize;
            if count == 0 || count > MAX_FAT_ARCHITECTURES {
                return Err(ModuleImageError::Invalid(format!(
                    "universal Mach-O has an invalid architecture count: {count}"
                )));
            }
            let entry_size = if wide_entries { 32 } else { 20 };
            let required = 8usize.saturating_add(count.saturating_mul(entry_size));
            if bytes.len() < required {
                return Err(ModuleImageError::Invalid(
                    "truncated universal Mach-O architecture table".to_owned(),
                ));
            }
            (0..count)
                .map(|index| {
                    let start = 8 + index * entry_size;
                    cpu_architecture(read_u32(&bytes[start..start + 4], little_endian))
                })
                .collect::<Vec<_>>()
        }
        0xfeedface | 0xfeedfacf | 0xcefaedfe | 0xcffaedfe => {
            if bytes.len() < 8 {
                return Err(ModuleImageError::Invalid(
                    "truncated Mach-O header".to_owned(),
                ));
            }
            let little_endian = matches!(magic_be, 0xcefaedfe | 0xcffaedfe);
            vec![cpu_architecture(read_u32(&bytes[4..8], little_endian))]
        }
        _ => {
            return Err(ModuleImageError::Invalid("invalid Mach-O magic".to_owned()));
        }
    };

    let host = PluginArchitecture::current_host();
    Ok(architectures
        .iter()
        .copied()
        .find(|architecture| *architecture == host)
        .or_else(|| architectures.first().copied())
        .unwrap_or(PluginArchitecture::Other))
}

fn read_u32(bytes: &[u8], little_endian: bool) -> u32 {
    let value: [u8; 4] = bytes.try_into().expect("four-byte integer");
    if little_endian {
        u32::from_le_bytes(value)
    } else {
        u32::from_be_bytes(value)
    }
}

fn cpu_architecture(cpu_type: u32) -> PluginArchitecture {
    match cpu_type {
        7 => PluginArchitecture::X86,
        0x0100_0007 => PluginArchitecture::X64,
        0x0100_000c => PluginArchitecture::Arm64,
        _ => PluginArchitecture::Other,
    }
}
