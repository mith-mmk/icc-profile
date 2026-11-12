//! Nonempty batches validate whitepoints, inputs and calculated results.
//! Invalid data and output reservation failures return errors; empty batches
//! skip evaluation. Finite byte outputs retain legacy rounding and clipping.
//!
use super::WhitePoint;
use crate::utils::checked_batch_len;
use std::io::{Error,ErrorKind};
use std::io::Result;

pub fn lab_to_xyz(l:f64,a:f64,b:f64) -> (f64,f64,f64) {
    lab_to_xyz_wp(l,a,b,&WhitePoint::d65())
}

/// Convert finite Lab with a finite, strictly positive XYZ whitepoint.
///
/// Returns `InvalidInput` for invalid inputs or nonfinite arithmetic results.
/// Finite values outside the usual color range are accepted, without clamping.
/// The legacy tuple-returning [`lab_to_xyz_wp`] remains unchecked.
pub fn try_lab_to_xyz_wp(l: f64, a: f64, b: f64, wp: &WhitePoint) -> Result<(f64, f64, f64)> {
    wp.validate()?;
    if ![l, a, b].iter().all(|value| value.is_finite()) {
        return Err(Error::new(ErrorKind::InvalidInput, "Nonfinite Lab input"));
    }
    let (x, y, z) = lab_to_xyz_wp(l, a, b, wp);
    if ![x, y, z].iter().all(|value| value.is_finite()) {
        return Err(Error::new(ErrorKind::InvalidInput, "Nonfinite XYZ result"));
    }
    Ok((x, y, z))
}

/// Legacy unchecked conversion; may produce NaN or infinity.
pub fn lab_to_xyz_wp(l:f64,a:f64,b:f64,wp: &WhitePoint) -> (f64,f64,f64) {
    let th: f64 = 6.0/29.0;
    let ti = 3.0 * th * th;
    let fy = (l +16.0) / 116.0;
    let fx =  fy + (a /500.0);
    let fz =  fy - (b /200.0);

    let x =  if fx > th {fx.powi(3) * wp.x} else {ti * (fx - 4.0/29.0) * wp.x };
    let y =  if fy > th {fy.powi(3) * wp.y} else {ti * (fy - 4.0/29.0) * wp.y };
    let z =  if fz > th {fz.powi(3) * wp.z} else {ti * (fz - 4.0/29.0) * wp.z };

    (x,y,z)
}


// xyz =0.0-1.0
pub fn lab_to_xyz_entries_f64(buf:&[f64],entries: usize,wp: &WhitePoint)  -> Result<Vec<f64>> {
    let output_len = checked_batch_len::<f64>(buf.len(), entries, 3, 3)?;
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(output_len).map_err(Error::other)?;

    for i in 0..entries {
        let ptr = i * 3;
        let l = buf[ptr];
        let a = buf[ptr + 1];
        let b = buf[ptr + 2];
        let (x,y,z) = try_lab_to_xyz_wp(l,a,b,wp)?;

        buffer.push(x);
        buffer.push(y);
        buffer.push(z);
    }

    Ok(buffer)

}

// lab 0-255 / xyz = 0-255

pub fn lab_to_xyz_entries(buf:&[u8],entries: usize,wp: &WhitePoint)  -> Result<Vec<u8>> {
    let output_len = checked_batch_len::<u8>(buf.len(), entries, 3, 3)?;
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(output_len).map_err(Error::other)?;

    for i in 0..entries {
        let ptr = i * 3;
        let l = buf[ptr]     as f64 / 255.0 * 100.0;
        let a = buf[ptr + 1] as f64 - 128.0;
        let b = buf[ptr + 2] as f64 - 128.0;
        let (x,y,z) = try_lab_to_xyz_wp(l,a,b,wp)?;

        let x = (x.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        let y = (y.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        let z = (z.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;

        buffer.push(x);
        buffer.push(y);
        buffer.push(z);
    }

    Ok(buffer)
}

