use super::WhitePoint;
use std::io::{Error,ErrorKind};
use std::io::Result;

pub fn lab_to_xyz(l:f64,a:f64,b:f64) -> (f64,f64,f64) {
    lab_to_xyz_wp(l,a,b,&WhitePoint::d65())
}

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
    let required = entries.checked_mul(3).ok_or_else(||
        Error::new(ErrorKind::InvalidInput, "Entry count overflow"))?;
    if buf.len() < required {
        return Err(Error::new(ErrorKind::Other, "Data shotage"))
    }
    let mut buffer = Vec::with_capacity(required);

    for i in 0..entries {
        let ptr = i * 3;
        let l = buf[ptr];
        let a = buf[ptr + 1];
        let b = buf[ptr + 2];
        let (x,y,z) = lab_to_xyz_wp(l,a,b,wp);

        buffer.push(x);
        buffer.push(y);
        buffer.push(z);
    }

    Ok(buffer)

}

// lab 0-255 / xyz = 0-255

pub fn lab_to_xyz_entries(buf:&[u8],entries: usize,wp: &WhitePoint)  -> Result<Vec<u8>> {
    let required = entries.checked_mul(3).ok_or_else(||
        Error::new(ErrorKind::InvalidInput, "Entry count overflow"))?;
    if buf.len() < required {
        return Err(Error::new(ErrorKind::Other, "Data shotage"))
    }
    let mut buffer = Vec::with_capacity(required);

    for i in 0..entries {
        let ptr = i * 3;
        let l = buf[ptr]     as f64 / 255.0 * 100.0;
        let a = buf[ptr + 1] as f64 - 128.0;
        let b = buf[ptr + 2] as f64 - 128.0;
        let (x,y,z) = lab_to_xyz_wp(l,a,b,wp);

        let x = ((x * 255.0 + 0.5) as i16).clamp(0,255) as u8;
        let y = ((y * 255.0 + 0.5) as i16).clamp(0,255) as u8;
        let z = ((z * 255.0 + 0.5) as i16).clamp(0,255) as u8;

        buffer.push(x);
        buffer.push(y);
        buffer.push(z);
    }

    Ok(buffer)
}

