
use crate::cms::transration::*;
use crate::Mft2;

pub fn cmyk_to_rgb(y:u8,m:u8,c:u8,k:u8) -> (u8,u8,u8) {
    let r = (c as i16 + k as i16 - 255).clamp(0, 255) as u8;
    let g = (m as i16 + k as i16 - 255).clamp(0, 255) as u8;
    let b = (y as i16 + k as i16 - 255).clamp(0, 255) as u8;
    (r,g,b)
}

pub fn cmyk_to_rgb_lut16(c:u8,m:u8,y:u8,k:u8,lut:&Mft2,wp:&WhitePoint) -> (u8,u8,u8) {
    let (l,a,b) = cmyk_to_lab_lut16(y,m,c,k,lut);
    let (x,y,z) = lab_to_xyz_wp(l,a,b,wp);
    let (r,g,b) = xyz_to_rgb(x as f64,y as f64,z as f64);

    (r,g,b)
}

pub fn cmyk_to_rgb_lut8(c:u8,m:u8,y:u8,k:u8,lut:&Mft1,wp:&WhitePoint) -> (u8,u8,u8) {
    let (l,a,b) = cmyk_to_lab_lut8(y,m,c,k,lut);
    let (x,y,z) = lab_to_xyz_wp(l,a,b,wp);
    let (r,g,b) = xyz_to_rgb(x as f64,y as f64,z as f64);

    (r,g,b)
}


pub fn cmyk_to_rgb_from_profile(c:u8,m:u8,y:u8,k:u8,decoded:&DecodedICCProfile) -> (u8,u8,u8) {
    if decoded.color_space == 0x434d594b {  // CMYK
        let lut = decoded.tags.get("A2B0");
        let wp = WhitePoint::from_profile(decoded);
        if let Some(lut) = lut {
            match lut {
                Data::Lut16(lut16) => {
                    return cmyk_to_rgb_lut16(c,m,y,k,lut16,&wp)
                },
                Data::Lut8(lut8) => {
                    return cmyk_to_rgb_lut8(c,m,y,k,lut8,&wp)

                },
                _ => {
                }
            }
        }
    }
    // not has profile
    cmyk_to_rgb(c, m, y, k)
}