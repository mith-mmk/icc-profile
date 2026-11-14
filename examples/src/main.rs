// icc profile reader
use icc_profile::iccprofile::*;
use icc_profile::utils::decoded_print;

use std::env;

pub fn main() -> std::io::Result<()> {
    let mut is_fast = true;
    for argument in env::args() {
        if is_fast {
            is_fast = false;
            continue;
        }
        println!("{}", argument);
        let icc_profile = icc_profile::utils::load(argument)?;
        let decoded = DecodedICCProfile::new(&icc_profile.data)?;
        println!("{}", decoded_print(&decoded, 0)?);
    }
    Ok(())
}
