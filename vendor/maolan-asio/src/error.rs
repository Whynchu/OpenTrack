//! Error type for ASIO driver calls.

use core::fmt;

use crate::abi::{
    ASE_HW_MALFUNCTION, ASE_INVALID_MODE, ASE_INVALID_PARAMETER, ASE_NO_CLOCK, ASE_NO_MEMORY,
    ASE_NOT_PRESENT, ASE_OK, ASE_SP_NOT_ADVANCING, ASE_SUCCESS,
};

/// Failure of an ASIO driver call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsioError {
    /// `ASE_NotPresent`: hardware input or output is not present.
    NotPresent,
    /// `ASE_HWMalfunction`: hardware is malfunctioning.
    HwMalfunction,
    /// `ASE_InvalidParameter`: an input parameter was invalid.
    InvalidParameter,
    /// `ASE_InvalidMode`: the driver rejected the mode or state transition.
    InvalidMode,
    /// `ASE_SPNotAdvancing`: the sample position is not advancing.
    SpNotAdvancing,
    /// `ASE_NoClock`: the sample clock could not be determined.
    NoClock,
    /// `ASE_NoMemory`: not enough memory for the operation.
    NoMemory,
    /// `init` returned `ASIOFalse`: initialization failed (see the
    /// driver's error message for the reason).
    InitFailed,
    /// A status code not defined by the ASIO ABI.
    Unexpected(i32),
}

impl AsioError {
    /// Map a raw `ASIOError` status code.
    pub(crate) fn from_code(code: i32) -> Self {
        match code {
            ASE_NOT_PRESENT => Self::NotPresent,
            ASE_HW_MALFUNCTION => Self::HwMalfunction,
            ASE_INVALID_PARAMETER => Self::InvalidParameter,
            ASE_INVALID_MODE => Self::InvalidMode,
            ASE_SP_NOT_ADVANCING => Self::SpNotAdvancing,
            ASE_NO_CLOCK => Self::NoClock,
            ASE_NO_MEMORY => Self::NoMemory,
            other => Self::Unexpected(other),
        }
    }
}

impl fmt::Display for AsioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPresent => write!(f, "hardware input or output is not present"),
            Self::HwMalfunction => write!(f, "hardware malfunction"),
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::InvalidMode => write!(f, "invalid mode"),
            Self::SpNotAdvancing => write!(f, "sample position is not advancing"),
            Self::NoClock => write!(f, "sample clock could not be determined"),
            Self::NoMemory => write!(f, "out of memory"),
            Self::InitFailed => write!(f, "driver initialization failed"),
            Self::Unexpected(code) => write!(f, "unexpected ASIO status code {code}"),
        }
    }
}

impl std::error::Error for AsioError {}

/// Failure to create or run a stream.
#[derive(Debug)]
pub enum StreamError {
    /// An ASIO driver call failed.
    Asio(AsioError),
    /// A channel uses a sample format the converter does not support yet;
    /// the raw tag is included.
    UnsupportedSampleType(i32),
    /// Another stream is already active in this process. The ABI
    /// provides no user pointer in `ASIOCallbacks`, so state recovery in
    /// the trampolines is process-global; only one stream at a time is
    /// supported (the same limitation as other ASIO hosts).
    Busy,
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Asio(e) => write!(f, "{e}"),
            Self::UnsupportedSampleType(raw) => write!(f, "unsupported sample type tag {raw}"),
            Self::Busy => write!(f, "another ASIO stream is already active"),
        }
    }
}

impl std::error::Error for StreamError {}

impl From<AsioError> for StreamError {
    fn from(e: AsioError) -> Self {
        Self::Asio(e)
    }
}

/// Convert a raw status code into a `Result`, treating both success codes
/// as `Ok`.
pub(crate) fn check(code: i32) -> Result<(), AsioError> {
    match code {
        ASE_OK | ASE_SUCCESS => Ok(()),
        other => Err(AsioError::from_code(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_codes_pass() {
        assert!(check(ASE_OK).is_ok());
        assert!(check(ASE_SUCCESS).is_ok());
    }

    #[test]
    fn error_codes_map() {
        assert_eq!(check(ASE_NOT_PRESENT), Err(AsioError::NotPresent));
        assert_eq!(
            check(ASE_INVALID_PARAMETER),
            Err(AsioError::InvalidParameter)
        );
        assert_eq!(check(ASE_NO_MEMORY), Err(AsioError::NoMemory));
        assert_eq!(check(123), Err(AsioError::Unexpected(123)));
    }

    #[test]
    fn display_is_human_readable() {
        assert!(!AsioError::NoClock.to_string().is_empty());
        assert!(AsioError::Unexpected(42).to_string().contains('4'));
    }
}
