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
            println!("A2B0 tabel is empty");
            return Ok(())
        };

        let lut = decoded.tags.get("B2A0").unwrap();
        let lut8 = if let Data::Lut8(lut8) = lut {
            lut8
        } else {
            println!("B2A0 tabel is empty");
            return Ok(())
        };

        for c in 0..=20 { 
            for m in 0..=20 { 
                for y in 0..=20 { 
                    for k in 0..=20 { 
                        let c0 = ((c as usize * 255) / 20).clamp(0,255);
                        let m0 = ((m as usize * 255) / 20).clamp(0,255);
                        let y0 = ((y as usize * 255) / 20).clamp(0,255);
                        let k0 = ((k as usize * 255) / 20).clamp(0,255);

                        let (l,la,lb) = cmyk_to_lab_lut16(c0 as u8,m0 as u8,y0 as u8,k0 as u8,lut16);
                        let (c1,m1,y1,k1) = lab_f64_to_cmyk_lut8_u8(l,la,la,lut8);
                        println!("CMYK {} {} {} {}",c0,m0,y0,k0);
                        println!("Lab {} {} {}",l,la,lb);
                        println!("CMYK {} {} {} {}",c1,m1,y1,k1);
                        println!("");
                    }
                }
            }
        }
    }
    Ok(())
}