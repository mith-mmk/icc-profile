//! # Color space transrator
//! - YUV(YCbCr) <--> RGB (BT.601/BT.709/Pal/Custom)
//! - RGB <--> XYZ
//! - XYZ <--> L*a*b     It need XYZ white point(default d65)
//! - L*a*b <--> CMYK    But it must need YCMK color space ICC Profile.
//! - CMYK --> RGB
//! 
//! # Color space transrator entries
//! There functions trunsrat color space,number of entry pixels.
//! Buffer size is larger than entries * sample per pixels(RGB=3,CMYK=4)

pub mod yuvrgb;
pub use yuvrgb::*;
pub mod rgbyuv;
pub use rgbyuv::*;
pub mod xyzrgb;
pub use xyzrgb::*;
pub mod lut_convert;
pub use lut_convert::*;
pub mod xyzlab;
pub use xyzlab::*;
pub mod labxyz;
pub use labxyz::*;
pub mod labcmyk;
pub use labcmyk::*;
pub mod rgbxyz;
pub use rgbxyz::*;
pub mod cmyklab;
pub use cmyklab::*;
pub mod cmykrgb;
pub use cmykrgb::*;
pub mod whitepoint;
pub use whitepoint::*;
pub mod gamma;
pub use gamma::*;