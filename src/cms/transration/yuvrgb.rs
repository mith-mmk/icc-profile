use crate::cms::ColorMatrix3D;
use std::io::Result;
use std::io::{Error,ErrorKind};


pub enum YUVToRGBCoefficient {
    Bt601,
    Pal,
    Bt709,
    Other(ColorMatrix3D)
}

impl YUVToRGBCoefficient {
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


/// (Y,Cb,Cr) -> (R, G, B)
/// Cb, Cr は中性点 128 のオフセット値 (0-255)
pub fn yuv_to_rgb (y:u8,cb:u8,cr:u8) -> (u8,u8,u8) {
    let matrix = YUVToRGBCoefficient::Bt601.get();
    let (r,g,b) = matrix.convert_3d(y as f64, cb as f64 - 128.0, cr as f64 - 128.0);
    let r = (r + 0.5).clamp(0.0, 255.0) as u8;
    let g = (g + 0.5).clamp(0.0, 255.0) as u8;
    let b = (b + 0.5).clamp(0.0, 255.0) as u8;
    (r,g,b)
}

pub fn yuv_to_rgb_with_mode (y:u8,cb:u8,cr:u8,mode: &YUVToRGBCoefficient) -> (u8,u8,u8) {
    let matrix = mode.get();
    let (r,g,b) = matrix.convert_3d(y as f64, cb as f64 - 128.0, cr as f64 - 128.0);
    let r = (r + 0.5).clamp(0.0, 255.0) as u8;
    let g = (g + 0.5).clamp(0.0, 255.0) as u8;
    let b = (b + 0.5).clamp(0.0, 255.0) as u8;
    (r,g,b)
}


pub fn yuv_to_rgb_entries (buf:&[u8],entries: usize,mode: &YUVToRGBCoefficient) -> Result<Vec<u8>> {
    if buf.len() < entries *3 {
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

pub fn yuv_to_rgba_entries_from_yuv (buf:&[u8],entries: usize,mode: &YUVToRGBCoefficient) -> Result<Vec<u8>> {
    if buf.len() < entries *3 {
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