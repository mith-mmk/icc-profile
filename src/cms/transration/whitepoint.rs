
use crate::iccprofile::ICCNumber;
use crate::DecodedICCProfile;

pub struct WhitePoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl WhitePoint {
    pub fn new(x:f64,y:f64,z:f64) -> Self {
        Self {
            x,
            y,
            z
        }
    }

    pub(crate) fn validate(&self) -> std::io::Result<()> {
        if [self.x, self.y, self.z].iter().all(|value| value.is_finite() && *value > 0.0) {
            Ok(())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Whitepoint must be finite and positive"))
        }
    }

    pub fn get(&self) -> &Self {
        &self
    }

    pub fn a() -> Self {
        Self{x:1.0985, y:1.0000, z:0.3558}
    }

    pub fn d50() -> Self {
        Self{x:0.9642, y:1.0000, z:0.8251}
    }

    pub fn d55() -> Self {
        Self{x:0.9568, y:1.0000, z:0.9214}
    }

    pub fn d65() -> Self {
        Self{x:0.9504, y:1.0000, z:1.0888}
    }

    pub fn from(x:f64,y:f64,z:f64) -> Self {
        Self{x:x
            ,y:y
            ,z:z
        }
    }

    /// ICC PCS D50 reference white (distinct from the rounded CIE D50 above).
    pub fn icc() -> Self {
        Self{x:0.9642, y:1.000, z:0.8249}
    }

    /// Read the profile header illuminant, not the media whitepoint `wtpt` tag.
    ///
    /// This legacy constructor does not validate the result or perform adaptation.
    pub fn from_profile(decoded:&DecodedICCProfile) -> Self {
        let illuminate = &decoded.illuminate;
        let x = illuminate.x.as_f64();
        let y = illuminate.y.as_f64();
        let z = illuminate.z.as_f64();
        Self::from(x,y,z)  
    }

}
#[cfg(test)]
mod allocation_tests {
    use crate::allocation_probe::{fail_once, failure_hits};
    use crate::cms::transration::*;

    #[test]
    fn lab_batch_output_reservation_failures_return_errors() {
        let wp = WhitePoint::d65();
        macro_rules! check {
            ($bytes:expr, $call:expr) => {{
                let result = fail_once($bytes, || $call);
                assert!(result.is_err());
                assert_eq!(failure_hits(), 1);
            }};
        }
        check!(3, lab_to_xyz_entries(&[128; 3], 1, &wp));
        check!(24, lab_to_xyz_entries_f64(&[50.0, 0.0, 0.0], 1, &wp));
        check!(3, xyz_to_lab_entries(&[128; 3], 1, &wp));
        check!(6, xyz_to_lab_entries_u16(&[128; 3], 1, &wp));
        check!(24, xyz_to_lab_entries_f64(&[128; 3], 1, &wp));
    }
}
