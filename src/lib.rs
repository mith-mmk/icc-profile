//! ICC profile reader crate
//! ``` 
//! use icc_profile::utils::decoded_print;
//! use icc_profile::iccprofile::*;
//! 
//! use std::env;
//! 
//! pub fn main() -> std::io::Result<()> {
//!     let mut is_fast = true;
//!     for argument in env::args() {
//!         if is_fast {
//!             is_fast = false;
//!             continue
//!         }
//!         println!("{}",argument);
//!         let icc_profile = icc_profile::utils::load(argument)?;
//!         let decoded = DecodedICCProfile::new(&icc_profile.data)?;
//!         println!("{}",decoded_print(&decoded, 0)?);
//!     }
//!     Ok(())
//! }
//! 
//! ```
//! # Default Color spaces ranges
//! - RGB        0..255,0..255,0..255 (u8)
//! - YUV(YCbCr) 0..255,0..255,0..255 (u8)
//! - XYZ 0.0..1.0,0.0..1.0,0.0..1.0 (f64)
//! - L*a*b* 0.0-100.0,-127.0..127.0,-127.0..127.0 (f64)
//! - CMYK 0..255,0..255,0..255,0..255 (u8)

pub use crate::iccprofile::*;
mod color_diff;
pub mod utils;
pub mod iccprofile;
pub mod cms;
pub mod transform;

pub use transform::{
    ColorSpace, ParseLimits, Pcs, Profile, RenderingIntent, Transform, TransformError,
    TransformOptions, TransformWorker,
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
