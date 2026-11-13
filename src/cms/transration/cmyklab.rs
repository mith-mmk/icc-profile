// icc profile reader
use crate::cms::transration::try_d4_to_d3_lut16;
use crate::cms::transration::try_d4_to_d3_lut8;
pub use crate::iccprofile::*;
use std::io::Result;

// La*b* 0-100 -127-127 -127-127
/// Compatibility wrapper for [`try_cmyk_to_lab_lut16_u8`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn cmyk_to_lab_lut16_u8(c: u8, m: u8, y: u8, k: u8, lut: &Mft2) -> (u8, u8, u8) {
    try_cmyk_to_lab_lut16_u8(c, m, y, k, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_cmyk_to_lab_lut16_u8(c: u8, m: u8, y: u8, k: u8, lut: &Mft2) -> Result<(u8, u8, u8)> {
    let (l, a, b) = try_cmyk_to_lab_lut16(c, m, y, k, lut)?;
    let l = ((l * 255.0 / 100.0) as i16).clamp(0, 255) as u8;
    let a = ((a + 128.0) as i16).clamp(0, 255) as u8;
    let b = ((b + 128.0) as i16).clamp(0, 255) as u8;
    Ok((l, a, b))
}

// YMCK -> Lab conversion 0-65536
/// Compatibility wrapper for [`try_cmyk_to_lab_lut16`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn cmyk_to_lab_lut16(c: u8, m: u8, y: u8, k: u8, lut: &Mft2) -> (f64, f64, f64) {
    try_cmyk_to_lab_lut16(c, m, y, k, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_cmyk_to_lab_lut16(c: u8, m: u8, y: u8, k: u8, lut: &Mft2) -> Result<(f64, f64, f64)> {
    let (l, a, b) = try_d4_to_d3_lut16(c, m, y, k, lut)?;

    let l = l * 100.0 / 65535.0;
    let a = a * 255.0 / 65535.0 - 127.5;
    let b = b * 255.0 / 65535.0 - 127.5;
    Ok((l, a, b))
}

// La*b* 0-100 -127-127 -127-127
/// Compatibility wrapper for [`try_cmyk_to_lab_lut8`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn cmyk_to_lab_lut8(c: u8, m: u8, y: u8, k: u8, lut: &Mft1) -> (f64, f64, f64) {
    try_cmyk_to_lab_lut8(c, m, y, k, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_cmyk_to_lab_lut8(c: u8, m: u8, y: u8, k: u8, lut: &Mft1) -> Result<(f64, f64, f64)> {
    let (l, a, b) = try_d4_to_d3_lut8(c, m, y, k, lut)?;
    let l = l as f64 / 255.0 * 100.0;
    let a = a as f64 - 127.0;
    let b = b as f64 - 127.0;

    Ok((l, a, b))
}

// no test
/// Compatibility wrapper for [`try_cmyk_to_lab_lut8_u8`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn cmyk_to_lab_lut8_u8(c: u8, m: u8, y: u8, k: u8, lut: &Mft1) -> (u8, u8, u8) {
    try_cmyk_to_lab_lut8_u8(c, m, y, k, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_cmyk_to_lab_lut8_u8(c: u8, m: u8, y: u8, k: u8, lut: &Mft1) -> Result<(u8, u8, u8)> {
    let (l, a, b) = try_d4_to_d3_lut8(c, m, y, k, lut)?;
    Ok((l, a, b))
}

pub fn cmyk_to_lab_entries_lut16(buf: &[u8], entries: usize, lut: &Mft2) -> Result<Vec<f64>> {
    if entries != 0 {
        super::lut_convert::check16(lut, 4, 3)?;
    }
    let mut buffer = super::lut_convert::batch_buffer(buf.len(), entries, 4, 3)?;

    for i in 0..entries {
        let ptr = i * 4;
        let c = buf[ptr];
        let m = buf[ptr + 1];
        let y = buf[ptr + 2];
        let k = buf[ptr + 3];
        let (l, a, b);
        (l, a, b) = try_cmyk_to_lab_lut16(c, m, y, k, lut)?;

        buffer.push(l);
        buffer.push(a);
        buffer.push(b);
    }

    Ok(buffer)
}

pub fn cmyk_to_lab_entries_lut8(buf: &[u8], entries: usize, lut: &Mft1) -> Result<Vec<f64>> {
    if entries != 0 {
        super::lut_convert::check8(lut, 4, 3)?;
    }
    let mut buffer = super::lut_convert::batch_buffer(buf.len(), entries, 4, 3)?;

    for i in 0..entries {
        let ptr = i * 4;
        let c = buf[ptr];
        let m = buf[ptr + 1];
        let y = buf[ptr + 2];
        let k = buf[ptr + 3];
        let (l, a, b);
        (l, a, b) = try_cmyk_to_lab_lut8(c, m, y, k, lut)?;

        buffer.push(l);
        buffer.push(a);
        buffer.push(b);
    }

    Ok(buffer)
}
