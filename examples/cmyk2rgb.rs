// icc profile reader
use icc_profile::cms::transration::*;
use icc_profile::utils::decoded_print;
pub use icc_profile::iccprofile::*;

use std::env;

pub fn main() -> std::io::Result<()> {
    let mut is_fast = true;
    for argument in env::args() {
        if is_fast {
            is_fast = false;
            continue
        }
        println!("{}",argument);
        let icc_profile = icc_profile::utils::load(argument)?;
        let decoded = DecodedICCProfile::new(&icc_profile.data)?;
        println!("{}",decoded_print(&decoded, 0)?);
        let lut = decoded.tags.get("A2B0").unwrap_or_else(|| {
            println!("A2B0 tabel is empty");
            std::process::exit(0);
        });

        let lut16 = if let Data::Lut16(lut16) = lut {
            lut16
        } else {
            return Ok(())
        };

//        let wp = WhitePoint::from_profile(&decoded);
        let wp = WhitePoint::d65();

        for c in 0..=20 { 
            for m in 0..=20 { 
                for y in 0..=20 { 
                    for k in 0..=20 { 
                        let cc = ((c as usize * 255) / 20).clamp(0,255);
                        let cm = ((m as usize * 255) / 20).clamp(0,255);
                        let cy = ((y as usize * 255) / 20).clamp(0,255);
                        let ck = ((k as usize * 255) / 20).clamp(0,255);

                        let (l,la,lb) = cmyk_to_lab_lut16(cc as u8,cm as u8,cy as u8,ck as u8,lut16);
                        let (x,y,z) = lab_to_xyz_wp(l,la,lb,&wp);
                        let (r,g,b) = xyz_to_rgb(x,y,z);
                        println!("CMYK {} {} {} {}",cc,cm,cy,ck);
                        println!("Lab {} {} {}",l,la,lb);
                        println!("XYZ {} {} {}",x,y,z);
                        println!("RGB {} {} {}",r,g,b);
                    }
                }
            }
        }
    }
    Ok(())
}