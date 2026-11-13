//! Color space Management System


use crate::iccprofile::ICCNumber;
use crate::S15Fixed16Number;

pub mod transration;

/// Legacy color-buffer container.
///
/// The seven `to_*` methods are unimplemented placeholders and always return
/// `None`, even when the stored representation matches the requested one.
/// Use the explicit conversion functions in [`transration`] instead; they
/// require callers to choose the whitepoint, encoding or profile as applicable.
pub enum ColorEntries {
    Rgb24(Vec<u8>),
    Rgba32(Vec<u8>),
    Lab24(Vec<u8>),
    Lab(Vec<f64>),
    Lab48(Vec<u16>),
    Ymck(Vec<u8>),
    Xyz(Vec<f64>),
    Xyz24(Vec<u8>),
}

impl ColorEntries {
    /// Unimplemented placeholder. Always returns `None`; no conversion is performed.
    pub fn to_rgb(&self) -> Option<Vec<u8>> {
        None
    }

    /// Unimplemented placeholder. Always returns `None`; no conversion is performed.
    pub fn to_lab(&self) -> Option<Vec<f64>> {
        None
    }

    /// Unimplemented placeholder. Always returns `None`; no conversion is performed.
    pub fn to_lab24(&self) -> Option<Vec<u8>> {
        None
    }

    /// Unimplemented placeholder. Always returns `None`; no conversion is performed.
    pub fn to_lab48(&self) -> Option<Vec<u16>> {
        None
    }


    /// Unimplemented placeholder. Always returns `None`; no conversion is performed.
    pub fn to_xyz(&self) -> Option<Vec<f64>> {
        None
    }

    /// Unimplemented placeholder. Always returns `None`; no conversion is performed.
    pub fn to_xyz24(&self) -> Option<Vec<u8>> {
        None
    }

    /// Unimplemented placeholder. Always returns `None`; no conversion is performed.
    pub fn to_ymck(&self) -> Option<Vec<f64>> {
        None
    }

}

#[derive(Clone)]
pub struct ColorMatrix3D {
    pub e: [f64;9]
}

#[derive(Clone)]
pub struct ColorMatrix3DWithShift {
    pub e: [f64;12]
}


impl ColorMatrix3D {
    pub fn cie_rgb_to_xyz() -> Self {
        Self {
            e : [0.4898, 0.3101, 0.2001, 0.1769, 0.8124, 0.0107, 0.0000, 0.0100, 0.9903]
        }
    }

    pub fn d65_rgb_to_xyz() -> Self {
        Self {
            e : [0.412391,  0.357584,  0.180481,
                 0.212639,  0.715169,  0.072192,
                 0.019331,  0.119195,  0.950532]
        }
    }
    pub fn c_rgb_to_xyz() -> Self {
        Self {
            e : [0.6069, 0.1735, 0.2003, 0.2989, 0.5866, 0.1144, 0.0000, 0.0661, 1.1157]
        }
    }

    pub fn adobe_rgb_to_xyz() -> Self {
        Self {
            e : [0.5778, 0.1825, 0.1902, 0.3070, 0.6170, 0.0761, 0.0181, 0.0695, 1.0015]
        }
    }
    pub fn ntsc_rgb_to_xyz() -> Self {
        Self {
            e : [0.6070, 0.1734, 0.2006, 0.2990, 0.5864, 0.1146, 0.0000, 0.0661, 1.1175]
        }
    }

    pub fn cie_xyz_to_rgb() -> Self {
        Self {
            e : [2.3655, - 0.8971, - 0.4683, -0.5151, 1.4264, 0.0887, 0.0052, -0.0144, 1.0089]
        }
    }

    pub fn d65_xyz_to_rgb() -> Self {
        Self {
            e : [3.240970, -1.537383, -0.498611,
                -0.969244 , 1.875968,  0.041555,
                 0.055630, -0.203977,  1.056972]
        }
    }

    pub fn c_xyz_to_rgb() -> Self {
        Self {
            e : [1.9099, - 0.5324, - 0.2882, -0.9846, 1.9991, -0.0283, 0.0583, -0.1184, 0.8979]
        }
    }

    pub fn adobe_xyz_to_rgb() -> Self {
        Self {
            e : [2.0416, -0.5650, -0.3447, -1.0199, 1.9171, 0.0481, 0.0340, - 0.1229, 1.0014]
        }
    }

    pub fn ntsc_xyz_to_rgb() -> Self {
        Self {
            e : [1.9097, - 0.5324, - 0.2882, -0.9850, 1.9998, - 0.0283, 0.0582, - 0.1182, 0.8966]
        }
    }

    pub fn from(e:&[f64]) -> Option<Self> {
        if e.len() != 9 {None}
        else {
            Some(Self {
                e: [e[0],e[1],e[2],e[3],e[4],e[5],e[6],e[7],e[8]]
            })
        }
    }

    pub fn from_s15_fixed16_number(e:&[S15Fixed16Number])-> Option<Self> {
        if e.len() != 9 {None}
        else {
            Some(Self {
                e: [e[0].as_f64(),e[1].as_f64(),e[2].as_f64(),
                    e[3].as_f64(),e[4].as_f64(),e[5].as_f64(),
                    e[6].as_f64(),e[7].as_f64(),e[8].as_f64()]
            })
        }
    }

    /// Compatibility spelling of [`Self::inverse`].
    pub fn invese(matrix:&Self) -> Option<Self> {
        Self::inverse(matrix)
    }

    /// Invert a finite matrix using the legacy cofactor calculation.
    ///
    /// Returns `None` for nonfinite input, a zero or nonfinite determinant, or
    /// a nonfinite result. This is not a condition-number check: no absolute
    /// determinant tolerance is imposed, and very small finite determinants
    /// remain valid when the calculation produces finite inverse entries.
    pub fn inverse(matrix:&Self) -> Option<Self> {
        if !matrix.e.iter().all(|value| value.is_finite()) {
            return None;
        }
        let a11 = matrix.e[0];
        let a12 = matrix.e[1];
        let a13 = matrix.e[2];
        let a21 = matrix.e[3];
        let a22 = matrix.e[4];
        let a23 = matrix.e[5];
        let a31 = matrix.e[6];
        let a32 = matrix.e[7];
        let a33 = matrix.e[8];
        
        let delta =  a11 * a22 * a33 + a12 *a23 * a31 + a13 * a21 * a32
            - a13 * a22 * a31 - a11 * a23 * a32 - a12 * a21 *a33;
        if delta == 0.0 || !delta.is_finite() {
            None
        } else {
            let inverse = Self{
                e:[
                (a22 * a33 - a23 * a32) /delta,
                (a13 * a32 - a12 * a33) /delta,
                (a12 * a23 - a13 * a22) /delta,

                (a23 * a31 - a21 * a33) /delta,
                (a11 * a33 - a13 * a31) /delta,
                (a13 * a21 - a11 * a23) /delta,
                
                (a21 * a32 - a22 * a31) /delta,
                (a12 * a31 - a11 * a32) /delta,
                (a11 * a22 - a12 * a21) /delta
                ]
            };
            if inverse.e.iter().all(|value| value.is_finite()) {
                Some(inverse)
            } else {
                None
            }
        }
    }

    /// Multiply three finite components by a finite matrix without clamping.
    ///
    /// Returns `InvalidInput` if any input or matrix coefficient is nonfinite,
    /// or if arithmetic produces a nonfinite result. Finite out-of-gamut values
    /// are accepted. Byte callers can apply their existing rounding and clipping
    /// policy after this check.
    pub fn try_convert_3d(&self, x: f64, y: f64, z: f64) -> std::io::Result<(f64, f64, f64)> {
        if ![x, y, z].iter().all(|value| value.is_finite())
            || !self.e.iter().all(|value| value.is_finite())
        {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Nonfinite matrix or input"));
        }
        let (a, b, c) = self.convert_3d(x, y, z);
        if ![a, b, c].iter().all(|value| value.is_finite()) {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Nonfinite matrix result"));
        }
        Ok((a, b, c))
    }

    /// Multiply components with ordinary floating-point arithmetic.
    ///
    /// This legacy scalar API may return NaN or infinity. Use
    /// [`Self::try_convert_3d`] when invalid input or overflow must be rejected.
    pub fn convert_3d(&self,x:f64,y:f64,z:f64) -> (f64,f64,f64) {
        let e = self.e;
        let (x,y,z) = (x,y,z);
        
        let a = x * e[0] + y * e[1] + z * e[2];    
        let b = x * e[3] + y * e[4] + z * e[5];    
        let c = x * e[6] + y * e[7] + z * e[8];    
        
        (a,b,c)
    }

    pub fn convert_3d_u8(&self,x:u8,y:u8,z:u8) -> (u8,u8,u8) {
        let e = self.e;
        let (x,y,z) = (x as f64,y as f64,z as f64);
        
        let a = x * e[0] + y * e[1] + z * e[2];    
        let b = x * e[3] + y * e[4] + z * e[5];    
        let c = x * e[6] + y * e[7] + z * e[8];    
        
        let a = (a as i16).clamp(0,255) as u8;
        let b = (b as i16).clamp(0,255) as u8;
        let c = (c as i16).clamp(0,255) as u8;
        (a,b,c)
    }

    pub fn convert_3d_f64_u8(&self,x:f64,y:f64,z:f64) -> (u8,u8,u8) {
        let e = self.e;
        let (x,y,z) = (x as f64,y as f64,z as f64);
        
        let a = x * e[0] + y * e[1] + z * e[2];    
        let b = x * e[3] + y * e[4] + z * e[5];    
        let c = x * e[6] + y * e[7] + z * e[8];    
        
        let a = ((a * 255.0 + 0.5) as i16).clamp(0,255) as u8;
        let b = ((b * 255.0 + 0.5) as i16).clamp(0,255) as u8;
        let c = ((c * 255.0 + 0.5) as i16).clamp(0,255) as u8;
        (a,b,c)
    }

    pub fn convert_3d_u8_f64(&self,x:u8,y:u8,z:u8) -> (f64,f64,f64) {
        let e = self.e;
        let (x,y,z) = (x as f64/255.0,y as f64/255.0,z as f64/255.0);
        
        let a = x * e[0] + y * e[1] + z * e[2];    
        let b = x * e[3] + y * e[4] + z * e[5];    
        let c = x * e[6] + y * e[7] + z * e[8];    
        
        (a,b,c)
    }

}


#[cfg(test)]
mod matrix_batch_allocation_tests {
    use super::transration::*;
    use crate::allocation_probe::{fail_once, failure_hits};

    #[test]
    fn output_reservation_failures_return_errors_for_all_matrix_batches() {
        macro_rules! check {
            ($bytes:expr, $call:expr) => {{
                let result = fail_once($bytes, || $call);
                assert!(result.is_err());
                assert_eq!(failure_hits(), 1);
            }};
        }
        check!(24, rgb_to_xyz_entries_f64(&[128; 3], 1, &RGBToXYZCoefficient::SrgbD65));
        check!(24, rgba_to_xyz_entries_f64(&[128; 4], 1, &RGBToXYZCoefficient::SrgbD65));
        check!(3, rgb_to_xyz_entries(&[128; 3], 1, &RGBToXYZCoefficient::SrgbD65));
        check!(3, rgba_to_xyz_entries(&[128; 4], 1, &RGBToXYZCoefficient::SrgbD65));
        check!(3, xyz_to_rgb_entries(&[128; 3], 1, &XYZtoRGBCoefficient::SrgbD65));
        check!(4, xyz_to_rgba_entries(&[128; 4], 1, &XYZtoRGBCoefficient::SrgbD65));
        check!(3, rgb_to_yuv_entries(&[128; 3], 1, &RGBToYUVCoefficient::Bt601));
        check!(3, yuv_to_rgba_entries_from_rgb(&[128; 4], 1, &RGBToYUVCoefficient::Bt601));
        check!(3, yuv_to_rgb_entries(&[128; 3], 1, &YUVToRGBCoefficient::Bt601));
        check!(4, yuv_to_rgba_entries_from_yuv(&[128; 3], 1, &YUVToRGBCoefficient::Bt601));
    }
}
