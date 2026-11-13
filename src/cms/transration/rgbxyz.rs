//! Matrix-only conversion from linear RGB to XYZ.
//!
//! `SrgbD65` selects primaries and reference white, not the sRGB transfer
//! function. Decode encoded RGB before calling; no EOTF, TRC or chromatic
//! adaptation is applied. Float RGB uses normalized linear components; byte
//! RGB represents linear components scaled by 255. Float XYZ is not clamped.
//! Byte XYZ uses a simplified 0..1 scale mapped to 0..255, truncates and clips
//! larger values (including D65 white Z). It is not ICC PCS XYZ encoding.
//!
//! Nonempty batch conversions reject nonfinite custom matrices and arithmetic
//! results with `InvalidInput`; finite outputs retain their existing clipping.
//! Output reservation failures are returned as errors.
//! Empty batches do not evaluate the matrix. Legacy scalar tuple APIs are unchecked.
//!
use crate::cms::ColorMatrix3D;
use crate::utils::checked_batch_len;
use std::io::Result;

pub enum RGBToXYZCoefficient {
    CieRgb,
    SrgbD65,
    SrgbC,
    AdobeRgb,
    NtscRgb,
    Other(ColorMatrix3D),
}

impl RGBToXYZCoefficient {
    pub fn get(&self) -> ColorMatrix3D {
        match self {
            RGBToXYZCoefficient::CieRgb => ColorMatrix3D::cie_rgb_to_xyz(),
            RGBToXYZCoefficient::SrgbD65 => ColorMatrix3D::d65_rgb_to_xyz(),
            RGBToXYZCoefficient::SrgbC => ColorMatrix3D::c_rgb_to_xyz(),
            RGBToXYZCoefficient::AdobeRgb => ColorMatrix3D::adobe_rgb_to_xyz(),
            RGBToXYZCoefficient::NtscRgb => ColorMatrix3D::ntsc_rgb_to_xyz(),
            RGBToXYZCoefficient::Other(matrix) => matrix.clone(),
        }
    }
}

pub fn rgb_to_xyz(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let matrix = RGBToXYZCoefficient::SrgbD65.get();
    matrix.convert_3d_u8_f64(r, g, b)
}

pub fn rgb_to_xyz_from_f64(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let matrix = RGBToXYZCoefficient::SrgbD65.get();
    matrix.convert_3d(r, g, b)
}

pub fn rgb_to_xyz_entries_f64(
    buf: &[u8],
    entries: usize,
    mode: &RGBToXYZCoefficient,
) -> Result<Vec<f64>> {
    let output_len = checked_batch_len::<f64>(buf.len(), entries, 3, 3)?;
    let matrix = mode.get();
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(output_len)
        .map_err(std::io::Error::other)?;

    for i in 0..entries {
        let ptr = i * 3;
        let r = buf[ptr];
        let g = buf[ptr + 1];
        let b = buf[ptr + 2];

        let (x, y, z) =
            matrix.try_convert_3d(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)?;

        buffer.push(x);
        buffer.push(y);
        buffer.push(z);
    }

    Ok(buffer)
}

pub fn rgba_to_xyz_entries_f64(
    buf: &[u8],
    entries: usize,
    mode: &RGBToXYZCoefficient,
) -> Result<Vec<f64>> {
    let output_len = checked_batch_len::<f64>(buf.len(), entries, 4, 3)?;
    let matrix = mode.get();
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(output_len)
        .map_err(std::io::Error::other)?;

    for i in 0..entries {
        let ptr = i * 4;
        let r = buf[ptr];
        let g = buf[ptr + 1];
        let b = buf[ptr + 2];

        let (x, y, z) =
            matrix.try_convert_3d(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)?;

        buffer.push(x);
        buffer.push(y);
        buffer.push(z);
    }

    Ok(buffer)
}

pub fn rgb_to_xyz_entries(
    buf: &[u8],
    entries: usize,
    mode: &RGBToXYZCoefficient,
) -> Result<Vec<u8>> {
    let output_len = checked_batch_len::<u8>(buf.len(), entries, 3, 3)?;
    let matrix = mode.get();
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(output_len)
        .map_err(std::io::Error::other)?;

    for i in 0..entries {
        let ptr = i * 3;
        let r = buf[ptr];
        let g = buf[ptr + 1];
        let b = buf[ptr + 2];

        let (x, y, z) = matrix.try_convert_3d(r as f64, g as f64, b as f64)?;
        let (x, y, z) = (
            x.clamp(0.0, 255.0) as u8,
            y.clamp(0.0, 255.0) as u8,
            z.clamp(0.0, 255.0) as u8,
        );

        buffer.push(x);
        buffer.push(y);
        buffer.push(z);
    }

    Ok(buffer)
}

pub fn rgba_to_xyz_entries(
    buf: &[u8],
    entries: usize,
    mode: &RGBToXYZCoefficient,
) -> Result<Vec<u8>> {
    let output_len = checked_batch_len::<u8>(buf.len(), entries, 4, 3)?;
    let matrix = mode.get();
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(output_len)
        .map_err(std::io::Error::other)?;

    for i in 0..entries {
        let ptr = i * 4;
        let r = buf[ptr];
        let g = buf[ptr + 1];
        let b = buf[ptr + 2];

        let (x, y, z) = matrix.try_convert_3d(r as f64, g as f64, b as f64)?;
        let (x, y, z) = (
            x.clamp(0.0, 255.0) as u8,
            y.clamp(0.0, 255.0) as u8,
            z.clamp(0.0, 255.0) as u8,
        );

        buffer.push(x);
        buffer.push(y);
        buffer.push(z);
    }

    Ok(buffer)
}
