//! YUV (YCbCr) to RGB color space conversion
//!
//! This module provides conversion functions from YUV/YCbCr color space to RGB.
//! Supports multiple video standards: BT.601, PAL, BT.709, and custom matrices.
//!
//! # Color Space Details
//!
//! YUV (luma-chroma) representation:
//! - **Y (luma)**: Brightness component [0-255]
//! - **Cb (chroma blue)**: Blue difference [0-255], neutral point = 128
//! - **Cr (chroma red)**: Red difference [0-255], neutral point = 128
//!
//! Neutral pixel (gray): Y=any, Cb=128, Cr=128 converts to (Y, Y, Y) in RGB
//!
//! # Standards
//!
//! - **BT.601**: ITU-R Rec.601 (SDTV, JPEG, MPEG-1, older video)
//! - **PAL**: Analog PAL/SECAM color system
//! - **BT.709**: ITU-R Rec.709 (HDTV, modern standard)
//!
//! # Examples
//!
//! ```ignore
//! use icc_profile::cms::transration::yuvrgb::{yuv_to_rgb, YUVToRGBCoefficient};
//!
//! // BT.601 conversion (default)
//! let (r, g, b) = yuv_to_rgb(128, 128, 128);  // Gray pixel
//!
//! // BT.709 (HDTV)
//! let (r, g, b) = yuv_to_rgb_with_mode(255, 128, 128, &YUVToRGBCoefficient::Bt709);
//! ```

use crate::cms::ColorMatrix3D;
use std::io::Result;
use std::io::{Error, ErrorKind};

/// YUV to RGB conversion coefficient presets
///
/// Selects the transformation matrix for YUV→RGB conversion.
/// Different standards use different luma and chroma coefficients optimized
/// for their respective color gamuts and viewing conditions.
///
/// # Variants
///
/// - `Bt601`: ITU-R Rec.601 (SDTV, JPEG, older video)
/// - `Pal`: Analog PAL/SECAM system
/// - `Bt709`: ITU-R Rec.709 (HDTV, modern standard)
/// - `Other`: Custom transformation matrix
#[derive(Clone)]
pub enum YUVToRGBCoefficient {
    Bt601,
    Pal,
    Bt709,
    Other(ColorMatrix3D),
}

impl YUVToRGBCoefficient {
    /// Get the transformation matrix for this coefficient preset
    ///
    /// Returns a 3×3 color matrix with coefficients optimized for the selected
    /// video standard (BT.601, PAL, BT.709, or custom).
    ///
    /// # Returns
    ///
    /// A `ColorMatrix3D` containing the YUV→RGB transformation matrix
    pub fn get(&self) -> ColorMatrix3D {
        match self {
            YUVToRGBCoefficient::Bt601 => {
                let crr = 1.402;
                let cbg = -0.34414;
                let crg = -0.71414;
                let cbb = 1.772;
                ColorMatrix3D::from(
                    &[1.0, 0.0, crr,
                     1.0, cbg, crg,
                     1.0, cbb, 0.0]).unwrap()
            },
            YUVToRGBCoefficient::Pal => {
                let crr = 1.1398;
                let cbg = -0.39465;
                let crg = -0.5806;
                let cbb = 2.03211;
                ColorMatrix3D::from(
                    &[1.0, 0.0, crr,
                     1.0, cbg, crg,
                     1.0, cbb, 0.0]).unwrap()
            },
            YUVToRGBCoefficient::Bt709 => {
                let crr = 1.5748;
                let cbg = -0.187324;
                let crg = -0.468124;
                let cbb = 1.8556;
                ColorMatrix3D::from(
                    &[1.0, 0.0, crr,
                     1.0, cbg, crg,
                     1.0, cbb, 0.0]).unwrap()
            },
            YUVToRGBCoefficient::Other(matrix) => {
                matrix.clone()
            }
        }
    }
}


/// Convert a single YUV pixel to RGB using BT.601 standard
///
/// Uses ITU-R Rec.601 luma and chroma coefficients. This is the most
/// commonly used standard for JPEG and older video formats.
///
/// # Arguments
///
/// * `y` - Luma (brightness) component [0-255]
/// * `cb` - Chroma blue component [0-255]
///   - Value 128 represents neutral (no blue difference)
///   - Values < 128: more blue, > 128: more yellow
/// * `cr` - Chroma red component [0-255]
///   - Value 128 represents neutral (no red difference)
///   - Values < 128: more cyan, > 128: more red
///
/// # Returns
///
/// RGB tuple `(r, g, b)` with each component in [0-255]
///
/// # Examples
///
/// ```ignore
/// // Gray pixel (Y can be any value, Cb=128, Cr=128)
/// let (r, g, b) = yuv_to_rgb(128, 128, 128);  // (128, 128, 128)
/// let (r, g, b) = yuv_to_rgb(255, 128, 128);  // (255, 255, 255) white
/// let (r, g, b) = yuv_to_rgb(0, 128, 128);    // (0, 0, 0) black
/// ```
pub fn yuv_to_rgb(y: u8, cb: u8, cr: u8) -> (u8, u8, u8) {
    let matrix = YUVToRGBCoefficient::Bt601.get();
    let (r,g,b) = matrix.convert_3d(y as f64, cb as f64 - 128.0, cr as f64 - 128.0);
    let r = (r + 0.5).clamp(0.0, 255.0) as u8;
    let g = (g + 0.5).clamp(0.0, 255.0) as u8;
    let b = (b + 0.5).clamp(0.0, 255.0) as u8;
    (r,g,b)
}

/// Convert a single YUV pixel to RGB with custom coefficient standard
///
/// # Arguments
///
/// * `y` - Luma component [0-255]
/// * `cb` - Chroma blue component [0-255], neutral = 128
/// * `cr` - Chroma red component [0-255], neutral = 128
/// * `mode` - Coefficient standard: Bt601, Pal, Bt709, or custom
///
/// # Returns
///
/// RGB tuple `(r, g, b)` with each component in [0-255]
///
/// # Examples
///
/// ```ignore
/// use icc_profile::cms::transration::yuvrgb::YUVToRGBCoefficient;
///
/// // HDTV conversion
/// let (r, g, b) = yuv_to_rgb_with_mode(128, 128, 128, &YUVToRGBCoefficient::Bt709);
/// ```
pub fn yuv_to_rgb_with_mode(y: u8, cb: u8, cr: u8, mode: &YUVToRGBCoefficient) -> (u8, u8, u8) {
    let matrix = mode.get();
    let (r,g,b) = matrix.convert_3d(y as f64, cb as f64 - 128.0, cr as f64 - 128.0);
    let r = (r + 0.5).clamp(0.0, 255.0) as u8;
    let g = (g + 0.5).clamp(0.0, 255.0) as u8;
    let b = (b + 0.5).clamp(0.0, 255.0) as u8;
    (r,g,b)
}


/// Batch convert YUV pixel buffer to RGB
///
/// Processes multiple YUV pixels efficiently in a single call.
/// Input buffer format: [Y1, Cb1, Cr1, Y2, Cb2, Cr2, ...]
/// Output buffer format: [R1, G1, B1, R2, G2, B2, ...]
///
/// # Arguments
///
/// * `buf` - Input buffer containing YCbCr triplets, must be at least `entries * 3` bytes
/// * `entries` - Number of pixels to convert
/// * `mode` - Coefficient standard (BT.601/709/PAL/Custom)
///
/// # Returns
///
/// `Ok(Vec<u8>)` containing RGB triplets (3 bytes per pixel) or `Err` if input too small
///
/// # Errors
///
/// Returns error if `buf.len() < entries * 3`
///
/// # Examples
///
/// ```ignore
/// let yuv = vec![128, 128, 128, 255, 128, 128, 0, 128, 128];  // 3 gray pixels
/// let rgb = yuv_to_rgb_entries(&yuv, 3, &YUVToRGBCoefficient::Bt601)?;
/// // rgb = [128, 128, 128, 255, 255, 255, 0, 0, 0]
/// ```
pub fn yuv_to_rgb_entries(buf: &[u8], entries: usize, mode: &YUVToRGBCoefficient) -> Result<Vec<u8>> {
    if buf.len() < entries * 3 {
        return Err(Error::new(ErrorKind::Other, "Data shotage"))
    }
    let matrix = mode.get();

    let mut buffer = Vec::with_capacity(entries * 3);

    for i in 0..entries {
        let ptr = i * 3;
        let y  = buf[ptr] as f64;
        let cb = buf[ptr + 1] as f64 - 128.0;
        let cr = buf[ptr + 2] as f64 - 128.0;

        let (r,g,b) = matrix.convert_3d(y, cb, cr);
        let r = (r + 0.5).clamp(0.0, 255.0) as u8;
        let g = (g + 0.5).clamp(0.0, 255.0) as u8;
        let b = (b + 0.5).clamp(0.0, 255.0) as u8;

        buffer.push(r);
        buffer.push(g);
        buffer.push(b);
    }

    Ok(buffer)
}

/// Convert YUV pixel buffer to RGBA (with alpha channel)
///
/// Converts YUV pixels to RGBA format, preserving original Y value as alpha.
/// Useful for video with alpha blending support.
/// Input format: [Y1, Cb1, Cr1, Y2, Cb2, Cr2, ...]
/// Output format: [R1, G1, B1, A1, R2, G2, B2, A2, ...] where A=Y
///
/// # Arguments
///
/// * `buf` - Input buffer: [Y1, Cb1, Cr1, Y2, Cb2, Cr2, ...]
/// * `entries` - Number of pixels to convert
/// * `mode` - Coefficient standard
///
/// # Returns
///
/// RGBA data where alpha channel equals the Y (luma) value
///
/// # Errors
///
/// Returns error if `buf.len() < entries * 3`
pub fn yuv_to_rgba_entries_from_yuv(buf: &[u8], entries: usize, mode: &YUVToRGBCoefficient) -> Result<Vec<u8>> {
    if buf.len() < entries * 3 {
        return Err(Error::new(ErrorKind::Other, "Data shotage"))
    }
    let matrix = mode.get();
    let mut buffer = Vec::with_capacity(entries * 4);

    for i in 0..entries {
        let ptr = i * 3;
        let y  = buf[ptr] as f64;
        let cb = buf[ptr + 1] as f64 - 128.0;
        let cr = buf[ptr + 2] as f64 - 128.0;

        let (r,g,b) = matrix.convert_3d(y, cb, cr);
        let r = (r + 0.5).clamp(0.0, 255.0) as u8;
        let g = (g + 0.5).clamp(0.0, 255.0) as u8;
        let b = (b + 0.5).clamp(0.0, 255.0) as u8;

        buffer.push(r);
        buffer.push(g);
        buffer.push(b);
        buffer.push(0xff);
    }

    Ok(buffer)
}