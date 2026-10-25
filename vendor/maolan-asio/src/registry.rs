//! ASIO driver discovery via the Windows registry.
//!
//! Drivers are registered under `HKLM\SOFTWARE\ASIO` (with a
//! `WOW6432Node` mirror for 32-bit drivers on 64-bit Windows): each
//! subkey is a driver, holding a `CLSID` string value and an optional
//! `Description` string value.

use std::io;

use crate::abi::AbiGuid;

/// An installed ASIO driver, as described by the registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DriverInfo {
    /// Driver name: the registry subkey name.
    pub name: String,
    /// COM class identifier used to instantiate the driver.
    pub clsid: AbiGuid,
    /// Optional human-readable description from the `Description` value.
    pub description: Option<String>,
}

impl std::fmt::Display for DriverInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.description {
            Some(desc) => write!(f, "{} ({desc})", self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

/// Enumerate installed ASIO drivers.
///
/// Returns an empty list when no drivers are registered or when the host
/// bitness has no matching drivers. Never fails merely because the
/// registry key is absent.
pub fn drivers() -> io::Result<Vec<DriverInfo>> {
    imp::enumerate()
}

/// Parse a CLSID string (`{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`, braces
/// optional, case-insensitive) into an [`AbiGuid`].
#[cfg(any(windows, test))]
fn parse_clsid(raw: &str) -> Option<AbiGuid> {
    let hex = raw
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(raw);
    if hex.len() != 36 {
        return None;
    }
    let bytes = hex.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        let expected_dash = matches!(i, 8 | 13 | 18 | 23);
        if (b == b'-') != expected_dash {
            return None;
        }
    }
    let u32_at = |a: usize, b: usize| u32::from_str_radix(&hex[a..b], 16).ok();
    let u16_at = |a: usize, b: usize| u16::from_str_radix(&hex[a..b], 16).ok();
    let u8_at = |a: usize| u8::from_str_radix(&hex[a..a + 2], 16).ok();
    let mut data4 = [0u8; 8];
    for (i, a) in [19, 21, 24, 26, 28, 30, 32, 34].into_iter().enumerate() {
        data4[i] = u8_at(a)?;
    }
    Some(AbiGuid {
        data1: u32_at(0, 8)?,
        data2: u16_at(9, 13)?,
        data3: u16_at(14, 18)?,
        data4,
    })
}

/// Build a [`DriverInfo`] from raw registry values, rejecting entries
/// without a parseable `CLSID`.
#[cfg(any(windows, test))]
fn parse_driver(
    name: &str,
    clsid: Option<&str>,
    description: Option<String>,
) -> Option<DriverInfo> {
    let clsid = parse_clsid(clsid?)?;
    Some(DriverInfo {
        name: name.to_owned(),
        clsid,
        description,
    })
}

/// Remove duplicate CLSIDs, keeping the first occurrence (the caller
/// enumerates the 64-bit view before the 32-bit view, so 64-bit drivers
/// win when both views register the same CLSID under different names).
#[cfg(any(windows, test))]
fn dedup(list: Vec<DriverInfo>) -> Vec<DriverInfo> {
    let mut out: Vec<DriverInfo> = Vec::with_capacity(list.len());
    for d in list {
        if !out.iter().any(|k| k.clsid == d.clsid) {
            out.push(d);
        }
    }
    out
}

#[cfg(windows)]
pub(crate) mod imp {
    use std::ffi::c_void;
    use std::io;

    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS};
    use windows::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_SAM_FLAGS,
        RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
    };
    use windows::core::PCWSTR;

    use super::{DriverInfo, dedup, parse_driver};
    use crate::abi::AbiGuid;

    const ASIO_KEY_PATH: &str = "SOFTWARE\\ASIO";
    const NAME_MAX_CHARS: usize = 256;
    const VALUE_MAX_CHARS: usize = 1024;

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn io_err(e: impl Into<windows::core::Error>) -> io::Error {
        io::Error::other(e.into())
    }

    /// Handle wrapper that closes the key on drop.
    struct Key(HKEY);

    impl Key {
        /// Open `HKLM\SOFTWARE\ASIO` with an explicit registry view.
        /// Returns `Ok(None)` when the key does not exist in this view.
        fn open_asio(view: REG_SAM_FLAGS) -> io::Result<Option<Key>> {
            let path = to_wide(ASIO_KEY_PATH);
            let mut hkey = HKEY::default();
            let err = unsafe {
                RegOpenKeyExW(
                    HKEY_LOCAL_MACHINE,
                    PCWSTR(path.as_ptr()),
                    None,
                    KEY_READ | view,
                    &mut hkey,
                )
            };
            if err.is_ok() {
                Ok(Some(Key(hkey)))
            } else if err == ERROR_FILE_NOT_FOUND {
                Ok(None)
            } else {
                Err(io_err(err))
            }
        }

        fn open_subkey(&self, name: &str, view: REG_SAM_FLAGS) -> io::Result<Option<Key>> {
            let name_wide = to_wide(name);
            let mut hkey = HKEY::default();
            let err = unsafe {
                RegOpenKeyExW(
                    self.0,
                    PCWSTR(name_wide.as_ptr()),
                    None,
                    KEY_READ | view,
                    &mut hkey,
                )
            };
            if err.is_ok() {
                Ok(Some(Key(hkey)))
            } else if err == ERROR_FILE_NOT_FOUND {
                Ok(None)
            } else {
                Err(io_err(err))
            }
        }

        /// Names of all direct subkeys.
        fn subkeys(&self) -> io::Result<Vec<String>> {
            let mut out = Vec::new();
            let mut index = 0u32;
            loop {
                let mut buf = [0u16; NAME_MAX_CHARS];
                let mut len = buf.len() as u32;
                let err = unsafe {
                    RegEnumKeyExW(
                        self.0,
                        index,
                        Some(windows::core::PWSTR(buf.as_mut_ptr())),
                        &mut len,
                        None,
                        None,
                        None,
                        None,
                    )
                };
                if err.is_ok() {
                    out.push(String::from_utf16_lossy(&buf[..len as usize]));
                    index += 1;
                } else if err == ERROR_NO_MORE_ITEMS {
                    break;
                } else {
                    return Err(io_err(err));
                }
            }
            Ok(out)
        }

        /// Read a string value; `Ok(None)` when the value is absent.
        fn string_value(&self, name: &str) -> io::Result<Option<String>> {
            let name_wide = to_wide(name);
            let mut buf = [0u16; VALUE_MAX_CHARS];
            let mut bytes = (buf.len() * size_of::<u16>()) as u32;
            let err = unsafe {
                RegGetValueW(
                    self.0,
                    PCWSTR::null(),
                    PCWSTR(name_wide.as_ptr()),
                    RRF_RT_REG_SZ,
                    None,
                    Some(buf.as_mut_ptr().cast::<c_void>()),
                    Some(&mut bytes),
                )
            };
            if err.is_ok() {
                // Reported size includes the trailing NUL.
                let chars = (bytes as usize / size_of::<u16>()).saturating_sub(1);
                Ok(Some(String::from_utf16_lossy(&buf[..chars])))
            } else if err == ERROR_FILE_NOT_FOUND {
                Ok(None)
            } else {
                Err(io_err(err))
            }
        }
    }

    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }

    pub fn enumerate() -> io::Result<Vec<DriverInfo>> {
        // Query both registry views so 64-bit hosts also see 32-bit-only
        // driver registrations; dedup prefers the 64-bit view.
        let mut all = Vec::new();
        for view in [if cfg!(target_pointer_width = "64") {
            KEY_WOW64_64KEY
        } else {
            KEY_WOW64_32KEY
        }] {
            let Some(root) = Key::open_asio(view)? else {
                continue;
            };
            for name in root.subkeys()? {
                let Some(key) = root.open_subkey(&name, view)? else {
                    continue;
                };
                let clsid = key.string_value("CLSID")?;
                let description = key.string_value("Description")?;
                if let Some(info) = parse_driver(&name, clsid.as_deref(), description) {
                    all.push(info);
                }
            }
        }
        Ok(dedup(all))
    }

    /// Convert to a `windows` crate GUID for COM calls.
    pub(crate) fn to_windows_guid(g: &AbiGuid) -> windows::core::GUID {
        windows::core::GUID::from_values(g.data1, g.data2, g.data3, g.data4)
    }
}

#[cfg(not(windows))]
mod imp {
    use std::io;

    use super::DriverInfo;

    pub fn enumerate() -> io::Result<Vec<DriverInfo>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clsid_braced() {
        let g = parse_clsid("{232685C6-6548-49D8-846D-4141A3EF7560}").unwrap();
        assert_eq!(g.to_string(), "232685C6-6548-49D8-846D-4141A3EF7560");
    }

    #[test]
    fn parse_clsid_bare_and_lowercase() {
        let a = parse_clsid("232685c6-6548-49d8-846d-4141a3ef7560").unwrap();
        let b = parse_clsid("{232685C6-6548-49D8-846D-4141A3EF7560}").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn parse_clsid_rejects_garbage() {
        assert!(parse_clsid("").is_none());
        assert!(parse_clsid("232685C6654849D8846D4141A3EF7560").is_none());
        assert!(parse_clsid("{232685C6-6548-49D8-846D-4141A3EF75ZZ}").is_none());
        assert!(parse_clsid("{232685C6-6548-49D8-846D-4141A3EF756}").is_none());
        assert!(parse_clsid("{232685C6-6548-49D8-846D4141-A3EF7560}").is_none());
        assert!(parse_clsid("{232685C6-6548-49D8-846D-4141A3EF75600}").is_none());
    }

    #[test]
    fn parse_driver_requires_clsid() {
        assert!(parse_driver("ASIO4ALL v2", None, None).is_none());
        assert!(parse_driver("ASIO4ALL v2", Some("not-a-clsid"), None).is_none());
        let d = parse_driver(
            "ASIO4ALL v2",
            Some("{232685C6-6548-49D8-846D-4141A3EF7560}"),
            Some("ASIO4ALL v2".to_owned()),
        )
        .unwrap();
        assert_eq!(d.name, "ASIO4ALL v2");
        assert_eq!(d.description.as_deref(), Some("ASIO4ALL v2"));
    }

    #[test]
    fn dedup_prefers_first() {
        let a = parse_driver(
            "ASIO4ALL v2",
            Some("{232685C6-6548-49D8-846D-4141A3EF7560}"),
            None,
        )
        .unwrap();
        let b = parse_driver(
            "ASIO4ALL v2 (32 bit)",
            Some("{232685C6-6548-49D8-846D-4141A3EF7560}"),
            None,
        )
        .unwrap();
        let c = parse_driver(
            "Other",
            Some("{11111111-2222-3333-4444-555555555555}"),
            None,
        )
        .unwrap();
        let out = dedup(vec![a.clone(), b, c.clone()]);
        assert_eq!(out, vec![a, c]);
    }

    #[test]
    fn drivers_runs_without_registry_entries() {
        // On non-Windows this is the stub; on Windows CI runners typically
        // have no ASIO drivers — either way it must succeed.
        assert!(drivers().is_ok());
    }
}
