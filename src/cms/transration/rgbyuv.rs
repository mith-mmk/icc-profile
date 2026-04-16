//! RGB to YUV (YCbCr) color space conversion
//!
//! This module provides conversion functions from RGB color space to YUV/YCbCr.
//! Supports multiple video standards: BT.601, PAL, BT.709, and custom matrices.
//!
//! # Color Space Details
//!
//! YUV (luma-chroma) representation:
//! - **Y (luma)**: Brightness component [0-255]
//! - **Cb (chroma blue)**: Blue difference [0-255], neutral point = 128
//! - **Cr (chroma red)**: Red difference [0-255], neutral point = 128
//!
//! Neutral pixel (gray): (R, R, R) converts to (R, 128, 128) in YUV
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
//! use icc_profile::cms::transration::rgbyuv::{rgb_to_yuv, RGBToYUVCoefficient};
//!
//! // BT.601 conversion (default)
//! let (y, cb, cr) = rgb_to_yuv(128, 128, 128);  // Gray pixel
//!
//! // BT.709 (HDTV)
//! let (y, cb, cr) = rgb_to_yuv_with_mode(255, 255, 255, &RGBToYUVCoefficient::Bt709);
//! ```

use crate::cms::ColorMatrix3D;
use std::io::Result;
use std::io::{Error, ErrorKind};

/// RGB to YUV conversion coefficient presets
///
/// Selects the transformation matrix for RGB→YUV conversion.
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
pub enum RGBToYUVCoefficient {
    Bt601,
    Pal,
    Bt709,
    Other(ColorMatrix3D),
}

impl RGBToYUVCoefficient {
    /// Get the transformation matrix for this coefficient preset
    ///
    /// Returns a 3×3 color matrix with coefficients optimized for the selected
    /// video standard (BT.601, PAL, BT.709, or custom).
    ///
    /// # Returns
    ///
    /// A `ColorMatrix3D` containing the RGB→YUV transformation matrix
    pub fn get(&self) -> ColorMatrix3D {
        match self {
            RGBToYUVCoefficient::Bt601 => {
                ColorMatrix3D::from(
                    &[0.299, 0.587, 0.114,
                    -0.168736, -0.331264, 0.5,
                    0.5, -0.418688, -0.081312]).unwrap()
            },
            RGBToYUVCoefficient::Pal => {
                ColorMatrix3D::from(
                    &[0.299, 0.587, 0.114,
                     -0.14713, -0.28886, 0.436,
                     0.615, -0.51499, -0.10001]).unwrap()     
            },
            RGBToYUVCoefficient::Bt709 => {
                ColorMatrix3D::from(
                    &[0.2126, 0.7152, 0.0722,
                     -0.114572, -0.385428, 0.5,
                     0.5, -0.454153, -0.045847]).unwrap()      
            },
            RGBToYUVCoefficient::Other(matrix) => {
                matrix.clone()
            }

        }
    }

}

/// Convert a single RGB pixel to YUV using BT.601 standard
///
/// Uses ITU-R Rec.601 luma and chroma coefficients. This is the most
/// commonly used standard for JPEG and older video formats.
///
/// # Arguments
///
/// * `r` - Red component [0-255]
/// * `g` - Green component [0-255]
/// * `b` - Blue component [0-255]
///
/// # Returns
///
/// YUV tuple `(y, cb, cr)` with:
/// - `y` - Luma [0-255]
/// - `cb` - Chroma blue [0-255], value 128 = neutral (gray)
/// - `cr` - Chroma red [0-255], value 128 = neutral (gray)
///
/// # Examples
///
/// ```ignore
/// // Gray pixel
/// let (y, cb, cr) = rgb_to_yuv(128, 128, 128);  // (128, 128, 128)
/// let (y, cb, cr) = rgb_to_yuv(255, 255, 255);  // (255, 128, 128) white
/// let (y, cb, cr) = rgb_to_yuv(0, 0, 0);        // (0, 128, 128) black
/// ```
pub fn rgb_to_yuv(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let matrix = RGBToYUVCoefficient::Bt601.get();
    let (y, cb, cr) = matrix.convert_3d(r as f64, g as f64, b as f64);
    let y  = (y  + 0.5).clamp(0.0, 255.0) as u8;
    let cb = (cb + 128.5).clamp(0.0, 255.0) as u8;
    let cr = (cr + 128.5).clamp(0.0, 255.0) as u8;
    (y, cb, cr)
}

/// Convert a single RGB pixel to YUV with custom coefficient standard
///
/// # Arguments
///
/// * `r` - Red component [0-255]
/// * `g` - Green component [0-255]
/// * `b` - Blue component [0-255]
/// * `mode` - Coefficient standard: Bt601, Pal, Bt709, or custom
///
/// # Returns
///
/// YUV tuple `(y, cb, cr)` with each component in [0-255]
///
/// # Examples
///
/// ```ignore
/// use icc_profile::cms::transration::rgbyuv::RGBToYUVCoefficient;
///
/// // HDTV conversion
/// let (y, cb, cr) = rgb_to_yuv_with_mode(128, 128, 128, &RGBToYUVCoefficient::Bt709);
/// ```
pub fn rgb_to_yuv_with_mode(r: u8, g: u8, b: u8, mode: &RGBToYUVCoefficient) -> (u8, u8, u8) {
    let matrix = mode.get();
    let (y, cb, cr) = matrix.convert_3d(r as f64, g as f64, b as f64);
    let y  = (y  + 0.5).clamp(0.0, 255.0) as u8;
    let cb = (cb + 128.5).clamp(0.0, 255.0) as u8;
    let cr = (cr + 128.5).clamp(0.0, 255.0) as u8;
    (y, cb, cr)
}


/// Batch convert RGB pixel buffer to YUV
///
/// Processes multiple RGB pixels efficiently in a single call.
/// Input buffer format: [R1, G1, B1, R2, G2, B2, ...]
/// Output buffer format: [Y1, Cb1, Cr1, Y2, Cb2, Cr2, ...]
///
/// # Arguments
///
/// * `buf` - Input buffer containing RGB triplets, must be at least `entries * 3` bytes
/// * `entries` - Number of pixels to convert
/// * `mode` - Coefficient standard (BT.601/709/PAL/Custom)
///
/// # Returns
///
/// `Ok(Vec<u8>)` containing YUV triplets (3 bytes per pixel) or `Err` if input too small
///
/// # Errors
///
/// Returns error if `buf.len() < entries * 3`
///
/// # Examples
///
/// ```ignore
/// let rgb = vec![128, 128, 128, 255, 255, 255, 0, 0, 0];  // 3 pixels: gray, white, black
/// let yuv = rgb_to_yuv_entries(&rgb, 3, &RGBToYUVCoefficient::Bt601)?;
/// // yuv = [128, 128, 128, 255, 128, 128, 0, 128, 128]
/// ```
pub fn rgb_to_yuv_entries(buf: &[u8], entries: usize, mode: &RGBToYUVCoefficient) -> Result<Vec<u8>> {
    if buf.len() < entries * 3 {
        return Err(Error::new(ErrorKind::Other, "Data shotage"))
    }
    let index = (entries / 3) as usize;
    let mut buffer = Vec::with_capacity(index * 3);
    let matrix = mode.get();

    for i in 0..entries {
        let ptr = i * 3;
        let r = buf[ptr];
        let g = buf[ptr + 1];
        let b = buf[ptr + 2];

        let (y, u, v) = matrix.convert_3d(r as f64, g as f64, b as f64);
        let y = (y + 0.5).clamp(0.0, 255.0) as u8;
        let u = (u + 128.5).clamp(0.0, 255.0) as u8;
        let v = (v + 128.5).clamp(0.0, 255.0) as u8;

        buffer.push(y);
        buffer.push(u);
        buffer.push(v);
    }

    Ok(buffer)
}

/// Convert RGBA pixel buffer to YUV
///
/// Processes RGBA input (with alpha channel) and outputs YUV triplets,
/// effectively extracting RGB and converting to YUV.
/// Input format: [R1, G1, B1, A1, R2, G2, B2, A2, ...]
/// Output format: [Y1, Cb1, Cr1, Y2, Cb2, Cr2, ...]
///
/// # Arguments
///
/// * `buf` - Input buffer: [R1, G1, B1, A1, R2, G2, B2, A2, ...], must be at least `entries * 4` bytes
/// * `entries` - Number of pixels to convert
/// * `mode` - Coefficient standard
///
/// # Returns
///
/// YUV data (3 bytes per pixel) from input RGBA (4 bytes per pixel)
///
/// # Errors
///
/// Returns error if `buf.len() < entries * 4`
pub fn yuv_to_rgba_entries_from_rgb(buf: &[u8], entries: usize, mode: &RGBToYUVCoefficient) -> Result<Vec<u8>> {
    if buf.len() < entries * 4 {
        return Err(Error::new(ErrorKind::Other, "Data shotage"))
    }
    let index = (entries / 4) as usize;
    let mut buffer = Vec::with_capacity(index * 3);
    let matrix = mode.get();

    for i in 0..entries {
        let ptr = i * 4;
        let r  = buf[ptr];
        let g = buf[ptr + 1];
        let b = buf[ptr + 2];

        let (y, u, v) = matrix.convert_3d(r as f64, g as f64, b as f64);
        let y = (y + 0.5).clamp(0.0, 255.0) as u8;
        let u = (u + 128.5).clamp(0.0, 255.0) as u8;
        let v = (v + 128.5).clamp(0.0, 255.0) as u8;

        buffer.push(y);
        buffer.push(u);
        buffer.push(v);
    }


    Ok(buffer)
}