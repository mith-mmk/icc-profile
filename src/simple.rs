//! Profile-free, explicitly encoded color conversions.
//!
//! These functions never inspect an ICC profile or fall back to another engine.
//! Scalar pixels are three-element arrays; buffers are tightly interleaved triples
//! with no padding or alpha. Buffers consume every sample and reject incomplete
//! pixels. Empty buffers return empty output, but supplied matrices/whitepoints
//! are still validated. Output allocation is fallible; no partial output is returned.
//!
//! `rgb8`/`ycbcr8_full` use full-range bytes (0–255, neutral chroma 128),
//! with the existing rounded/clipped byte quantization. The coefficient choice
//! does not apply a transfer function. Floating RGB is already linear, XYZ is
//! physical/relative tristimulus data (normally reference-white Y = 1, not ICC
//! encoded XYZ), and Lab uses L* (normally 0–100) and signed a*, b* units.
//! Floating values are finite, unclipped, and may be outside nominal ranges.
//! Callers choose matching matrix directions, XYZ scales, and whitepoints.
//! There is no automatic matrix inversion, chromatic adaptation, transfer curve,
//! gamut mapping, alpha processing, or CMYK interpretation.
//! Reexported `ColorMatrix3D` and `WhitePoint` retain legacy utilities, including
//! unchecked methods; the guarantees above apply to this module's conversion
//! functions, not to every method on those reexported types.

use crate::cms::transration::{
    rgb_to_yuv_with_mode, try_lab_to_xyz_wp, try_xyz_to_lab_wp, yuv_to_rgb_with_mode,
    RGBToYUVCoefficient, YUVToRGBCoefficient,
};
pub use crate::cms::transration::WhitePoint;
pub use crate::cms::ColorMatrix3D;
use std::{collections::TryReserveError, error::Error, fmt};

/// Luma/chroma coefficients for full-range digital YCbCr, not studio-range video.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum YcbcrCoefficients {
    /// BT.601 luma/chroma coefficients.
    Bt601,
    /// BT.709 luma/chroma coefficients.
    Bt709,
}

/// Invalid input, nonfinite arithmetic, or failure to reserve output storage.
#[derive(Debug)]
#[non_exhaustive]
pub enum SimpleError {
    /// A tightly interleaved buffer must contain complete triples.
    InvalidBufferLength { samples: usize },
    /// A floating input sample was NaN or infinity.
    NonFiniteInput,
    /// A matrix coefficient was NaN or infinity.
    InvalidMatrix,
    /// Whitepoint components must be finite and strictly positive.
    InvalidWhitePoint,
    /// Finite inputs produced a nonfinite result (for example, overflow).
    NonFiniteResult,
    /// Output capacity overflow or allocation failure.
    Allocation(TryReserveError),
}

impl fmt::Display for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBufferLength { samples } => write!(f, "buffer has {samples} samples; complete three-sample pixels are required"),
            Self::NonFiniteInput => f.write_str("input contains NaN or infinity"),
            Self::InvalidMatrix => f.write_str("matrix contains NaN or infinity"),
            Self::InvalidWhitePoint => f.write_str("whitepoint components must be finite and strictly positive"),
            Self::NonFiniteResult => f.write_str("conversion produced a nonfinite result"),
            Self::Allocation(error) => write!(f, "cannot reserve output storage: {error}"),
        }
    }
}
impl Error for SimpleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self { Self::Allocation(error) => Some(error), _ => None }
    }
}

fn reserve<T>(samples: usize) -> Result<Vec<T>, SimpleError> {
    let mut result = Vec::new();
    result.try_reserve_exact(samples).map_err(SimpleError::Allocation)?;
    Ok(result)
}
fn buffer<T: Copy>(input: &[T], mut convert: impl FnMut([T; 3]) -> Result<[T; 3], SimpleError>) -> Result<Vec<T>, SimpleError> {
    if input.len() % 3 != 0 {
        return Err(SimpleError::InvalidBufferLength { samples: input.len() });
    }
    let mut result = reserve(input.len())?;
    for pixel in input.chunks_exact(3) {
        result.extend_from_slice(&convert([pixel[0], pixel[1], pixel[2]])?);
    }
    Ok(result)
}
fn finite(input: [f64; 3]) -> Result<(), SimpleError> {
    if input.iter().all(|v| v.is_finite()) { Ok(()) } else { Err(SimpleError::NonFiniteInput) }
}
fn matrix_valid(matrix: &ColorMatrix3D) -> Result<(), SimpleError> {
    if matrix.e.iter().all(|v| v.is_finite()) { Ok(()) } else { Err(SimpleError::InvalidMatrix) }
}
fn white_valid(white: &WhitePoint) -> Result<(), SimpleError> {
    white.validate().map_err(|_| SimpleError::InvalidWhitePoint)
}
fn array<T>((a, b, c): (T, T, T)) -> [T; 3] { [a, b, c] }

/// Convert `[R,G,B]` bytes to full-range `[Y,Cb,Cr]` bytes.
/// Uses existing coefficients and round-to-nearest/clipped byte quantization.
pub fn rgb8_to_ycbcr8_full(rgb: [u8; 3], coefficients: YcbcrCoefficients) -> [u8; 3] {
    let mode = match coefficients { YcbcrCoefficients::Bt601 => RGBToYUVCoefficient::Bt601, YcbcrCoefficients::Bt709 => RGBToYUVCoefficient::Bt709 };
    array(rgb_to_yuv_with_mode(rgb[0], rgb[1], rgb[2], &mode))
}
/// Convert full-range `[Y,Cb,Cr]` bytes to `[R,G,B]` bytes; clips to 0–255.
pub fn ycbcr8_full_to_rgb8(ycbcr: [u8; 3], coefficients: YcbcrCoefficients) -> [u8; 3] {
    let mode = match coefficients { YcbcrCoefficients::Bt601 => YUVToRGBCoefficient::Bt601, YcbcrCoefficients::Bt709 => YUVToRGBCoefficient::Bt709 };
    array(yuv_to_rgb_with_mode(ycbcr[0], ycbcr[1], ycbcr[2], &mode))
}
/// Convert all interleaved RGB byte triples to full-range YCbCr triples.
/// Rejects incomplete pixels; an empty buffer succeeds.
pub fn rgb8_to_ycbcr8_full_buffer(input: &[u8], coefficients: YcbcrCoefficients) -> Result<Vec<u8>, SimpleError> {
    buffer(input, |pixel| Ok(rgb8_to_ycbcr8_full(pixel, coefficients)))
}
/// Convert all full-range YCbCr byte triples to RGB triples.
/// Rejects incomplete pixels; an empty buffer succeeds.
pub fn ycbcr8_full_to_rgb8_buffer(input: &[u8], coefficients: YcbcrCoefficients) -> Result<Vec<u8>, SimpleError> {
    buffer(input, |pixel| Ok(ycbcr8_full_to_rgb8(pixel, coefficients)))
}

fn matrix_pixel(input: [f64; 3], matrix: &ColorMatrix3D) -> Result<[f64; 3], SimpleError> {
    matrix_valid(matrix)?;
    finite(input)?;
    matrix.try_convert_3d(input[0], input[1], input[2]).map(array).map_err(|_| SimpleError::NonFiniteResult)
}
/// Multiply linear `[R,G,B]` by the supplied RGB-to-XYZ matrix without clipping.
/// RGB is normally normalized to 0–1; the matrix determines physical XYZ scale.
/// Rejects nonfinite input, coefficients, or results. Does not decode sRGB.
pub fn linear_rgb_f64_to_xyz_f64(rgb: [f64; 3], rgb_to_xyz: &ColorMatrix3D) -> Result<[f64; 3], SimpleError> {
    matrix_pixel(rgb, rgb_to_xyz)
}
/// Multiply physical `[X,Y,Z]` by an explicit XYZ-to-linear-RGB matrix.
/// No inverse is inferred, no transfer encoding or clipping is performed.
pub fn xyz_f64_to_linear_rgb_f64(xyz: [f64; 3], xyz_to_rgb: &ColorMatrix3D) -> Result<[f64; 3], SimpleError> {
    matrix_pixel(xyz, xyz_to_rgb)
}
/// Convert complete linear RGB triples to physical XYZ triples, without clipping.
/// Validates the matrix even for an empty buffer.
pub fn linear_rgb_f64_to_xyz_f64_buffer(input: &[f64], rgb_to_xyz: &ColorMatrix3D) -> Result<Vec<f64>, SimpleError> {
    matrix_valid(rgb_to_xyz)?;
    buffer(input, |pixel| linear_rgb_f64_to_xyz_f64(pixel, rgb_to_xyz))
}
/// Convert complete physical XYZ triples to linear RGB triples, without clipping.
/// Validates the explicitly supplied inverse-direction matrix even for empty input.
pub fn xyz_f64_to_linear_rgb_f64_buffer(input: &[f64], xyz_to_rgb: &ColorMatrix3D) -> Result<Vec<f64>, SimpleError> {
    matrix_valid(xyz_to_rgb)?;
    buffer(input, |pixel| xyz_f64_to_linear_rgb_f64(pixel, xyz_to_rgb))
}
/// Convert physical XYZ to CIE L*, a*, b* using an explicit whitepoint.
/// XYZ and whitepoint must use the same scale; white components must be positive.
/// Rejects nonfinite inputs/results and never clips or chromatically adapts.
pub fn xyz_f64_to_lab_f64(xyz: [f64; 3], white: &WhitePoint) -> Result<[f64; 3], SimpleError> {
    white_valid(white)?;
    finite(xyz)?;
    try_xyz_to_lab_wp(xyz[0], xyz[1], xyz[2], white).map(array).map_err(|_| SimpleError::NonFiniteResult)
}
/// Convert CIE L*, a*, b* to physical XYZ in the supplied whitepoint's scale.
/// Whitepoint must be finite/positive; finite out-of-range Lab is not clipped.
pub fn lab_f64_to_xyz_f64(lab: [f64; 3], white: &WhitePoint) -> Result<[f64; 3], SimpleError> {
    white_valid(white)?;
    finite(lab)?;
    try_lab_to_xyz_wp(lab[0], lab[1], lab[2], white).map(array).map_err(|_| SimpleError::NonFiniteResult)
}
/// Convert complete physical XYZ triples to CIE L*, a*, b* triples.
/// Validates the whitepoint even for empty input; never returns partial output.
pub fn xyz_f64_to_lab_f64_buffer(input: &[f64], white: &WhitePoint) -> Result<Vec<f64>, SimpleError> {
    white_valid(white)?;
    buffer(input, |pixel| xyz_f64_to_lab_f64(pixel, white))
}
/// Convert complete CIE L*, a*, b* triples to physical XYZ triples.
/// Validates the whitepoint even for empty input; never returns partial output.
pub fn lab_f64_to_xyz_f64_buffer(input: &[f64], white: &WhitePoint) -> Result<Vec<f64>, SimpleError> {
    white_valid(white)?;
    buffer(input, |pixel| lab_f64_to_xyz_f64(pixel, white))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_allocator_failure_returns_error() {
        use crate::allocation_probe::{fail_once, failure_hits};
        let result = fail_once(3, || rgb8_to_ycbcr8_full_buffer(&[1, 2, 3], YcbcrCoefficients::Bt601));
        assert_eq!(failure_hits(), 1);
        assert!(matches!(result, Err(SimpleError::Allocation(_))));
        let white = WhitePoint::new(1., 1., 1.);
        let result = fail_once(3 * std::mem::size_of::<f64>(), || xyz_f64_to_lab_f64_buffer(&[0.; 3], &white));
        assert_eq!(failure_hits(), 1);
        assert!(matches!(result, Err(SimpleError::Allocation(_))));
    }
    #[test]
    fn capacity_overflow_is_reported_without_allocating() {
        let error = reserve::<f64>(usize::MAX).unwrap_err();
        assert!(matches!(error, SimpleError::Allocation(_)));
        assert!(error.source().is_some());
    }
}
