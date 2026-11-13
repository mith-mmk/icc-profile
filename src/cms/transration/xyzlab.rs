use crate::cms::transration::WhitePoint;
use crate::utils::checked_batch_len;
use std::io::Result;



pub fn xyz_to_lab(x:f64,y:f64,z:f64) -> (f64,f64,f64) {
    xyz_to_lab_wp(x,y,z,&WhitePoint::d65())
}

pub fn xyz_to_lab_wp(x:f64,y:f64,z:f64,wp: &WhitePoint) -> (f64,f64,f64) {
    let th = 6.0_f64/29.0_f64;
    let ti = 3.0 * th * th;
    let th_3 = th.powi(3);

    let fx = if x / wp.x > th_3 { (x / wp.x).cbrt() } else { (x / wp.x) / ti + 4.0 / 29.0 };
    let fy = if y / wp.y > th_3 { (y / wp.y).cbrt() } else { (y / wp.y) / ti + 4.0 / 29.0 };
    let fz = if z / wp.z > th_3 { (z / wp.z).cbrt() } else { (z / wp.z) / ti + 4.0 / 29.0 };

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);
    (l,a,b)
}

pub fn xyz_to_lab_entries(buf:&[u8],entries: usize,wp: &WhitePoint) -> Result<Vec<u8>> {
    let output_len = checked_batch_len::<u8>(buf.len(), entries, 3, 3)?;
    let mut buffer = Vec::with_capacity(output_len);

    for i in 0..entries {
        let ptr = i * 3;
        let x = buf[ptr]     as f64 / 255.0;
        let y = buf[ptr + 1] as f64 / 255.0;
        let z = buf[ptr + 2] as f64 / 255.0;
        let (l,a,b) = xyz_to_lab_wp(x,y,z,wp);

        let l = ((l / 100.0 * 255.0) as i16).clamp(0,255) as u8;
        let a = ((a + 128.0) as i16).clamp(0,255) as u8;
        let b = ((b + 128.0) as i16).clamp(0,255) as u8;

        buffer.push(l);
        buffer.push(a);
        buffer.push(b);
    }

    Ok(buffer)
}

pub fn xyz_to_lab_entries_u16(buf:&[u8],entries: usize,mode: &WhitePoint) -> Result<Vec<u16>> {
    let output_len = checked_batch_len::<u16>(buf.len(), entries, 3, 3)?;
    let wp = mode.get();
    let mut buffer = Vec::with_capacity(output_len);

    for i in 0..entries {
        let ptr = i * 3;
        let x = buf[ptr]     as f64 / 255.0;
        let y = buf[ptr + 1] as f64 / 255.0;
        let z = buf[ptr + 2] as f64 / 255.0;

        let (l,a,b) = xyz_to_lab_wp(x,y,z,wp);       

        let l = ((l / 100.0 * 65535.0) as i32).clamp(0,65535) as u16;
        let a = (((a + 128.0) * 255.0) as i32).clamp(0,65535) as u16;
        let b = (((b + 128.0) * 255.0) as i32).clamp(0,65535) as u16;

        buffer.push(l);
        buffer.push(a);
        buffer.push(b);
    }

    Ok(buffer)
}

pub fn xyz_to_lab_entries_f64 (buf:&[u8],entries: usize,mode: &WhitePoint) -> Result<Vec<f64>> {
    let output_len = checked_batch_len::<f64>(buf.len(), entries, 3, 3)?;
    let wp = mode.get();
    let mut buffer = Vec::with_capacity(output_len);

    for i in 0..entries {
        let ptr = i * 3;
        let x = buf[ptr]     as f64 / 255.0;
        let y = buf[ptr + 1] as f64 / 255.0;
        let z = buf[ptr + 2] as f64 / 255.0;

        let (l,a,b) = xyz_to_lab_wp(x,y,z,wp);

        buffer.push(l);
        buffer.push(a);
        buffer.push(b);
    }

    Ok(buffer)
}