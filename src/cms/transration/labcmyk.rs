// La*b* 0-100 -127-127 -127-127
use crate::cms::transration::try_d3_to_d4_lut16;
use crate::cms::transration::try_d3_to_d4_lut8;
use crate::{Mft1, Mft2};
use std::io::Result;

/// Compatibility wrapper for [`try_lab_to_cmyk_lut16_u8`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn lab_to_cmyk_lut16_u8(l: u8, a: u8, b: u8, lut: &Mft2) -> (u8, u8, u8, u8) {
    try_lab_to_cmyk_lut16_u8(l, a, b, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_lab_to_cmyk_lut16_u8(l: u8, a: u8, b: u8, lut: &Mft2) -> Result<(u8, u8, u8, u8)> {
    let (c, m, y, k) = try_lab_to_cmyk_lut16(l, a, b, lut)?;

    let c = (c / 100.0 * 255.0 + 0.5) as u8;
    let m = (m / 100.0 * 255.0 + 0.5) as u8;
    let y = (y / 100.0 * 255.0 + 0.5) as u8;
    let k = (k / 100.0 * 255.0 + 0.5) as u8;
    Ok((c, m, y, k))
}

/// Compatibility wrapper for [`try_lab_to_cmyk_lut16`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn lab_to_cmyk_lut16(l: u8, a: u8, b: u8, lut: &Mft2) -> (f64, f64, f64, f64) {
    try_lab_to_cmyk_lut16(l, a, b, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_lab_to_cmyk_lut16(l: u8, a: u8, b: u8, lut: &Mft2) -> Result<(f64, f64, f64, f64)> {
    let (c, m, y, k) = try_d3_to_d4_lut16(l, a, b, lut)?;

    let c = c / 65535.0 * 100.0;
    let m = m / 65535.0 * 100.0;
    let y = y / 65535.0 * 100.0;
    let k = k / 65535.0 * 100.0;
    Ok((c, m, y, k))
}

/// Compatibility wrapper for [`try_lab_f64_to_cmyk_lut16`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn lab_f64_to_cmyk_lut16(l: f64, a: f64, b: f64, lut: &Mft2) -> (f64, f64, f64, f64) {
    try_lab_f64_to_cmyk_lut16(l, a, b, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_lab_f64_to_cmyk_lut16(
    l: f64,
    a: f64,
    b: f64,
    lut: &Mft2,
) -> Result<(f64, f64, f64, f64)> {
    super::lut_convert::check_lab(l, a, b)?;
    let l = (l / 100.0 * 255.0) as u8;
    let a = (a + 127.5) as u8;
    let b = (b + 127.5) as u8;
    let (c, m, y, k) = try_d3_to_d4_lut16(l, a, b, lut)?;

    let c = c / 65535.0 * 100.0;
    let m = m / 65535.0 * 100.0;
    let y = y / 65535.0 * 100.0;
    let k = k / 65535.0 * 100.0;
    Ok((c, m, y, k))
}

/// Compatibility wrapper for [`try_lab_to_cmyk_lut8`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn lab_to_cmyk_lut8(l: u8, a: u8, b: u8, lut: &Mft1) -> (f64, f64, f64, f64) {
    try_lab_to_cmyk_lut8(l, a, b, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_lab_to_cmyk_lut8(l: u8, a: u8, b: u8, lut: &Mft1) -> Result<(f64, f64, f64, f64)> {
    let (c, m, y, k) = try_d3_to_d4_lut8(l, a, b, lut)?;

    let c = c as f64 / 255.0 * 100.0;
    let m = m as f64 / 255.0 * 100.0;
    let y = y as f64 / 255.0 * 100.0;
    let k = k as f64 / 255.0 * 100.0;
    Ok((c, m, y, k))
}

/// Compatibility wrapper for [`try_lab_f64_to_cmyk_lut8`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn lab_f64_to_cmyk_lut8(l: f64, a: f64, b: f64, lut: &Mft1) -> (f64, f64, f64, f64) {
    try_lab_f64_to_cmyk_lut8(l, a, b, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_lab_f64_to_cmyk_lut8(
    l: f64,
    a: f64,
    b: f64,
    lut: &Mft1,
) -> Result<(f64, f64, f64, f64)> {
    super::lut_convert::check_lab(l, a, b)?;
    let l = (l / 100.0 * 255.0 + 0.5) as u8;
    let a = (a + 127.5) as u8;
    let b = (b + 127.5) as u8;
    try_lab_to_cmyk_lut8(l, a, b, lut)
}

// no test
/// Compatibility wrapper for [`try_lab_to_cmyk_lut8_u8`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn lab_to_cmyk_lut8_u8(l: u8, a: u8, b: u8, lut: &Mft1) -> (u8, u8, u8, u8) {
    try_lab_to_cmyk_lut8_u8(l, a, b, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_lab_to_cmyk_lut8_u8(l: u8, a: u8, b: u8, lut: &Mft1) -> Result<(u8, u8, u8, u8)> {
    try_d3_to_d4_lut8(l, a, b, lut)
}

/// Compatibility wrapper for [`try_lab_f64_to_cmyk_lut8_u8`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn lab_f64_to_cmyk_lut8_u8(l: f64, a: f64, b: f64, lut: &Mft1) -> (u8, u8, u8, u8) {
    try_lab_f64_to_cmyk_lut8_u8(l, a, b, lut).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_lab_f64_to_cmyk_lut8_u8(l: f64, a: f64, b: f64, lut: &Mft1) -> Result<(u8, u8, u8, u8)> {
    super::lut_convert::check_lab(l, a, b)?;
    let l = (l / 100.0 * 255.0 + 0.5) as u8;
    let a = (a + 127.5) as u8;
    let b = (b + 127.5) as u8;
    try_d3_to_d4_lut8(l, a, b, lut)
}

pub fn lab_to_cmyk_entries_lut16_u8(buf: &[u8], entries: usize, lut: &Mft2) -> Result<Vec<u8>> {
    if entries != 0 {
        super::lut_convert::check16(lut, 3, 4)?;
    }
    let mut buffer = super::lut_convert::batch_buffer(buf.len(), entries, 3, 4)?;

    for i in 0..entries {
        let ptr = i * 3;
        let l = buf[ptr];
        let a = buf[ptr + 1];
        let b = buf[ptr + 2];
        let (c, m, y, k) = try_lab_to_cmyk_lut16_u8(l, a, b, lut)?;

        buffer.push(c);
        buffer.push(m);
        buffer.push(y);
        buffer.push(k);
    }

    Ok(buffer)
}

pub fn lab_to_cmyk_entries_lut16(buf: &[u8], entries: usize, lut: &Mft2) -> Result<Vec<f64>> {
    if entries != 0 {
        super::lut_convert::check16(lut, 3, 4)?;
    }
    let mut buffer = super::lut_convert::batch_buffer(buf.len(), entries, 3, 4)?;

    for i in 0..entries {
        let ptr = i * 3;
        let l = buf[ptr];
        let a = buf[ptr + 1];
        let b = buf[ptr + 2];
        let (c, m, y, k) = try_lab_to_cmyk_lut16(l, a, b, lut)?;

        buffer.push(c);
        buffer.push(m);
        buffer.push(y);
        buffer.push(k);
    }

    Ok(buffer)
}

pub fn lab_to_cmyk_entries_lut8(buf: &[u8], entries: usize, lut: &Mft1) -> Result<Vec<f64>> {
    if entries != 0 {
        super::lut_convert::check8(lut, 3, 4)?;
    }
    let mut buffer = super::lut_convert::batch_buffer(buf.len(), entries, 3, 4)?;

    for i in 0..entries {
        let ptr = i * 3;
        let l = buf[ptr];
        let a = buf[ptr + 1];
        let b = buf[ptr + 2];
        let (c, m, y, k) = try_lab_to_cmyk_lut8(l, a, b, lut)?;

        buffer.push(c);
        buffer.push(m);
        buffer.push(y);
        buffer.push(k);
    }

    Ok(buffer)
}

pub fn lab_to_cmyk_entries_lut8_u8(buf: &[u8], entries: usize, lut: &Mft1) -> Result<Vec<u8>> {
    if entries != 0 {
        super::lut_convert::check8(lut, 3, 4)?;
    }
    let mut buffer = super::lut_convert::batch_buffer(buf.len(), entries, 3, 4)?;

    for i in 0..entries {
        let ptr = i * 3;
        let l = buf[ptr];
        let a = buf[ptr + 1];
        let b = buf[ptr + 2];
        let (c, m, y, k) = try_lab_to_cmyk_lut8_u8(l, a, b, lut)?;

        buffer.push(c);
        buffer.push(m);
        buffer.push(y);
        buffer.push(k);
    }

    Ok(buffer)
}

pub fn lab_f64_to_cmyk_entries_lut8(buf: &[f64], entries: usize, lut: &Mft1) -> Result<Vec<f64>> {
    if entries != 0 {
        super::lut_convert::check8(lut, 3, 4)?;
    }
    let mut buffer = super::lut_convert::batch_buffer(buf.len(), entries, 3, 4)?;

    for i in 0..entries {
        let ptr = i * 3;
        let l = buf[ptr];
        let a = buf[ptr + 1];
        let b = buf[ptr + 2];
        let (c, m, y, k) = try_lab_f64_to_cmyk_lut8(l, a, b, lut)?;

        buffer.push(c);
        buffer.push(m);
        buffer.push(y);
        buffer.push(k);
    }

    Ok(buffer)
}

pub fn lab_f64_to_cmyk_entries_lut8_u8(buf: &[f64], entries: usize, lut: &Mft1) -> Result<Vec<u8>> {
    if entries != 0 {
        super::lut_convert::check8(lut, 3, 4)?;
    }
    let mut buffer = super::lut_convert::batch_buffer(buf.len(), entries, 3, 4)?;

    for i in 0..entries {
        let ptr = i * 3;
        let l = buf[ptr];
        let a = buf[ptr + 1];
        let b = buf[ptr + 2];
        let (c, m, y, k) = try_lab_f64_to_cmyk_lut8_u8(l, a, b, lut)?;

        buffer.push(c);
        buffer.push(m);
        buffer.push(y);
        buffer.push(k);
    }

    Ok(buffer)
}
