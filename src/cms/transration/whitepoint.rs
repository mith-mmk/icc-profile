
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

    pub fn from_profile(decoded:&DecodedICCProfile) -> Self {
        let illuminate = &decoded.illuminate;
        let x = illuminate.x.as_f64();
        let y = illuminate.y.as_f64();
        let z = illuminate.z.as_f64();
        Self::from(x,y,z)  
    }

}