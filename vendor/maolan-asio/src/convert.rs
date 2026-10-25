//! Conversion between ASIO's raw per-channel buffers and `f32` samples.
//!
//! Only little-endian formats are supported (Windows hosts); other
//! formats are rejected at stream creation so the audio path never
//! fails. Integer formats are normalized to `[-1.0, 1.0)` with full-scale
//! mapping to 1.0.

use crate::driver::SampleType;

/// Whether the audio path can convert `ty` to and from `f32`.
pub(crate) fn supported(ty: SampleType) -> bool {
    matches!(
        ty,
        SampleType::Int16Lsb | SampleType::Int24Lsb | SampleType::Int32Lsb | SampleType::Float32Lsb
    )
}

const INT16_SCALE: f32 = 32768.0; // 2^15
const INT24_SCALE: f32 = 8388608.0; // 2^23
const INT32_SCALE: f64 = 2147483648.0; // 2^31

/// Convert `frames` samples of raw driver data into `dst`.
///
/// # Safety
///
/// `src` must be valid for reads of `frames * ty.size_bytes()` bytes.
pub(crate) unsafe fn to_f32(ty: SampleType, src: *const u8, frames: usize, dst: &mut [f32]) {
    debug_assert!(dst.len() >= frames);
    for (i, out) in dst.iter_mut().enumerate().take(frames) {
        // SAFETY: the caller guarantees `frames * size` readable bytes;
        // each access stays within that region.
        let p = unsafe { src.add(i * ty.size_bytes().unwrap_or(0)) };
        *out = match ty {
            SampleType::Int16Lsb => {
                let b = unsafe { (p as *const [u8; 2]).read_unaligned() };
                f32::from(i16::from_le_bytes(b)) / INT16_SCALE
            }
            SampleType::Int24Lsb => {
                let b = unsafe { (p as *const [u8; 3]).read_unaligned() };
                let v =
                    (i32::from(b[0]) | (i32::from(b[1]) << 8) | (i32::from(b[2]) << 16)) << 8 >> 8;
                (v as f32) / INT24_SCALE
            }
            SampleType::Int32Lsb => {
                let b = unsafe { (p as *const [u8; 4]).read_unaligned() };
                (i32::from_le_bytes(b) as f64 / INT32_SCALE) as f32
            }
            SampleType::Float32Lsb => {
                let b = unsafe { (p as *const [u8; 4]).read_unaligned() };
                f32::from_le_bytes(b)
            }
            other => unreachable!("unsupported sample type in audio path: {other:?}"),
        };
    }
}

/// Convert `src` `f32` samples into raw driver data.
///
/// # Safety
///
/// `dst` must be valid for writes of `src.len() * ty.size_bytes()` bytes.
pub(crate) unsafe fn from_f32(ty: SampleType, src: &[f32], dst: *mut u8) {
    let scale = match ty {
        SampleType::Int16Lsb => INT16_SCALE,
        SampleType::Int24Lsb => INT24_SCALE,
        SampleType::Int32Lsb => INT32_SCALE as f32,
        SampleType::Float32Lsb => {
            for (i, &x) in src.iter().enumerate() {
                // SAFETY: caller guarantees `len * 4` writable bytes.
                let p = unsafe { dst.add(i * 4) };
                unsafe { (p as *mut [u8; 4]).write_unaligned(x.to_le_bytes()) };
            }
            return;
        }
        other => unreachable!("unsupported sample type in audio path: {other:?}"),
    };
    let max = scale - 1.0;
    for (i, &x) in src.iter().enumerate() {
        let v = (x.clamp(-1.0, 1.0) * scale).round().clamp(-scale, max) as i32;
        // SAFETY: caller guarantees `len * size` writable bytes.
        let p = unsafe { dst.add(i * ty.size_bytes().unwrap_or(0)) };
        match ty {
            SampleType::Int16Lsb => unsafe {
                (p as *mut [u8; 2]).write_unaligned((v as i16).to_le_bytes())
            },
            SampleType::Int24Lsb => {
                let b = v.to_le_bytes();
                unsafe { (p as *mut [u8; 3]).write_unaligned([b[0], b[1], b[2]]) };
            }
            SampleType::Int32Lsb => unsafe { (p as *mut [u8; 4]).write_unaligned(v.to_le_bytes()) },
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASES: &[f32] = &[-1.0, -0.5, -0.001, 0.0, 0.001, 0.5, 0.999_969_5, 1.0];

    fn round_trip(ty: SampleType) {
        let size = ty.size_bytes().unwrap();
        let mut raw = vec![0u8; CASES.len() * size];
        // SAFETY: `raw` is exactly `len * size` bytes.
        unsafe { from_f32(ty, CASES, raw.as_mut_ptr()) };
        let mut out = vec![0.0f32; CASES.len()];
        // SAFETY: `raw` is exactly `len * size` bytes.
        unsafe { to_f32(ty, raw.as_ptr(), CASES.len(), &mut out) };
        let tol = 1.0
            / match ty {
                SampleType::Int16Lsb => INT16_SCALE,
                SampleType::Int24Lsb => INT24_SCALE,
                SampleType::Int32Lsb => INT32_SCALE as f32,
                SampleType::Float32Lsb => 0.0,
                _ => unreachable!(),
            };
        for (want, got) in CASES.iter().zip(&out) {
            assert!((want - got).abs() <= tol, "{ty:?}: {want} became {got}");
        }
    }

    #[test]
    fn round_trips_all_supported_types() {
        round_trip(SampleType::Int16Lsb);
        round_trip(SampleType::Int24Lsb);
        round_trip(SampleType::Int32Lsb);
        round_trip(SampleType::Float32Lsb);
    }

    #[test]
    fn known_int24_encoding() {
        let mut raw = [0u8; 3];
        // SAFETY: 3 bytes, exactly one Int24 sample.
        unsafe { from_f32(SampleType::Int24Lsb, &[1.0], raw.as_mut_ptr()) };
        // +1.0 clamps to the largest positive 24-bit value.
        assert_eq!(raw, [0xff, 0xff, 0x7f]);
        let mut out = [0.0f32; 1];
        // SAFETY: 3 readable bytes.
        unsafe { to_f32(SampleType::Int24Lsb, raw.as_ptr(), 1, &mut out) };
        assert!((out[0] - 8388607.0 / 8388608.0).abs() < 1e-7);
        // Negative full scale.
        // SAFETY: 3 bytes, exactly one Int24 sample.
        unsafe { from_f32(SampleType::Int24Lsb, &[-1.0], raw.as_mut_ptr()) };
        assert_eq!(raw, [0x00, 0x00, 0x80]);
    }

    #[test]
    fn negative_int24_sign_extends() {
        let raw = [0x00u8, 0x00, 0x80]; // -8388608
        let mut out = [0.0f32; 1];
        // SAFETY: 3 readable bytes.
        unsafe { to_f32(SampleType::Int24Lsb, raw.as_ptr(), 1, &mut out) };
        assert_eq!(out[0], -1.0);
    }

    #[test]
    fn support_matrix() {
        assert!(supported(SampleType::Int16Lsb));
        assert!(supported(SampleType::Int24Lsb));
        assert!(supported(SampleType::Int32Lsb));
        assert!(supported(SampleType::Float32Lsb));
        assert!(!supported(SampleType::Int16Msb));
        assert!(!supported(SampleType::Float64Lsb));
        assert!(!supported(SampleType::Unknown(7)));
    }
}
