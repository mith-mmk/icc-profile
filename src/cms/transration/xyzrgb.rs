//! Nonempty batch conversions reject nonfinite custom matrices and arithmetic
//! results with `InvalidInput`; finite outputs retain their existing clipping.
//! Output reservation failures are returned as errors.
//! Empty batches do not evaluate the matrix. Legacy scalar tuple APIs are unchecked.
//!
use std::io::Result;
use crate::utils::checked_batch_len;
use crate::cms::ColorMatrix3D;

pub enum XYZtoRGBCoefficient {
    CieRgb,
    SrgbD65,
    SrgbC,
    AdobeRgb,
    NtscRgb,
    Other(ColorMatrix3D)
}

impl XYZtoRGBCoefficient {
    pub fn get(&self) -> ColorMatrix3D {
        match self {
            XYZtoRGBCoefficient::CieRgb => {
                ColorMatrix3D::cie_xyz_to_rgb()
            },
            XYZtoRGBCoefficient::SrgbD65 => {
                ColorMatrix3D::d65_xyz_to_rgb()
            },
            XYZtoRGBCoefficient::SrgbC => {
                ColorMatrix3D::c_xyz_to_rgb()
            },
            XYZtoRGBCoefficient::AdobeRgb => {
                ColorMatrix3D::adobe_xyz_to_rgb()
            },
            XYZtoRGBCoefficient::NtscRgb => {
                ColorMatrix3D::ntsc_xyz_to_rgb()
            },
            XYZtoRGBCoefficient::Other(matrix) => {
                matrix.clone()
            },
        }
    }
}

pub fn xyz_to_rgb_to_f64(x:f64,y:f64,z:f64) -> (f64,f64,f64) {
    let matrix = XYZtoRGBCoefficient::SrgbD65.get();
    let (r,g,b) = matrix.convert_3d(x, y, z);
    (r,g,b)
}


pub fn xyz_to_rgb(x:f64,y:f64,z:f64) -> (u8,u8,u8) {
    let matrix = XYZtoRGBCoefficient::SrgbD65.get();
    let (r,g,b) = matrix.convert_3d_f64_u8(x, y, z);
    (r,g,b)
}


pub fn xyz_to_rgb_entries (buf:&[u8],entries: usize,mode: &XYZtoRGBCoefficient) -> Result<Vec<u8>> {
    let output_len = checked_batch_len::<u8>(buf.len(), entries, 3, 3)?;
    let matrix = mode.get();
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(output_len).map_err(std::io::Error::other)?;

    for i in 0..entries {
        let ptr = i * 3;
        let x = buf[ptr];
        let y = buf[ptr + 1];
        let z = buf[ptr + 2];

        let (r,g,b) = matrix.try_convert_3d(x as f64, y as f64, z as f64)?;
        let (r,g,b) = (r.clamp(0.0, 255.0) as u8, g.clamp(0.0, 255.0) as u8, b.clamp(0.0, 255.0) as u8);

        buffer.push(r);
        buffer.push(g);
        buffer.push(b);

    }

    Ok(buffer)
}

pub fn xyz_to_rgba_entries (buf:&[u8],entries: usize,mode: &XYZtoRGBCoefficient) -> Result<Vec<u8>> {
    let output_len = checked_batch_len::<u8>(buf.len(), entries, 4, 4)?;
    let matrix = mode.get();
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(output_len).map_err(std::io::Error::other)?;

    for i in 0..entries {
        let ptr = i * 4;
        let x = buf[ptr];
        let y = buf[ptr + 1];
        let z = buf[ptr + 2];

        let (r,g,b) = matrix.try_convert_3d(x as f64, y as f64, z as f64)?;
        let (r,g,b) = (r.clamp(0.0, 255.0) as u8, g.clamp(0.0, 255.0) as u8, b.clamp(0.0, 255.0) as u8);

        buffer.push(r);
        buffer.push(g);
        buffer.push(b);
        buffer.push(0xff);
    }


    Ok(buffer)
}