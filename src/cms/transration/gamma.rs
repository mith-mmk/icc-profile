use crate::{ICCNumber, ParametricCurve};
use std::io::{Error,ErrorKind};

/// Evaluate the legacy byte-domain parametric curve helper.
///
/// Input bytes are used directly as `x` in `0..=255`, not normalized ICC
/// coordinates. Finite results are rounded and clamped to a byte; for example,
/// type 0 with gamma 2 maps 128 to 255. Unknown function types remain identity.
/// Empty batches succeed without evaluating or validating the curve.
///
/// # Errors
///
/// Returns an error for short input, missing coefficients, a zero divisor `a`
/// in types 1 or 2, or a nonfinite evaluated result (including invalid real
/// power domains and overflow), or an output reservation failure.
/// Types 3 and 4 do not divide by `a` and may use
/// zero `a` when the selected branch yields a finite result.
pub fn transration_prametic_curve(buf:&[u8] ,entry:usize,prametic_curve:ParametricCurve) -> Result<Vec<u8>, Error>{
    if buf.len() < entry {
        return Err(Error::new(ErrorKind::Other, "Data shotage"))
    }
    // Empty batches historically accept even incomplete curves without evaluating them.
    if entry == 0 {
        return Ok(Vec::new());
    }
    let required = match prametic_curve.funtion_type {
        0 => 1,
        1 => 3,
        2 => 4,
        3 => 5,
        4 => 7,
        _ => 0,
    };
    if entry != 0 && prametic_curve.vals.len() < required {
        return Err(Error::new(ErrorKind::InvalidInput, "Parametric curve coefficients shortage"));
    }
    // At most seven coefficients are used; ignore surplus data without allocating.
    let mut p = [0.0; 7];
    for (index, val) in prametic_curve.vals.iter().take(required).enumerate() {
        p[index] = val.as_f64();
    }
    if entry != 0 && matches!(prametic_curve.funtion_type, 1 | 2) && p[1] == 0.0 {
        return Err(Error::new(ErrorKind::InvalidInput, "Parametric curve has zero divisor a"));
    }
    let mut data = Vec::new();
    data.try_reserve_exact(entry).map_err(Error::other)?;
 
    for i in 0..entry {
        let x = buf[i] as f64;
        let y;
        match prametic_curve.funtion_type {
            0x000 => {
                let gamma = p[0];
                y = x.powf(gamma); 
            },
            0x001 => {
                let gamma = p[0];
                let a = p[1];
                let b = p[2];
                y = if x >= - b / a {
                    (a * x + b).powf(gamma)
                } else {
                    0.0
                };
            },
            0x002 => {
                let gamma = p[0];
                let a = p[1];
                let b = p[2];
                let c = p[3];
                y = if x >= - b / a {
                    (a * x + b).powf(gamma) + c
                } else {
                    c
                };
            },
            0x003 => {
                let gamma = p[0];
                let a = p[1];
                let b = p[2];
                let c = p[3];
                let d = p[4];
                y = if x >= d {
                    (a * x + b).powf(gamma)
                } else {
                    c * x
                };
            },
            0x004 => {
                let gamma = p[0];
                let a = p[1];
                let b = p[2];
                let c = p[3];
                let d = p[4];
                let e = p[5];
                let f = p[6];
                y = if x >= d {
                    (a * x + b).powf(gamma) + e
                } else {
                    c * x + f
                };
            },
            _ => { y = x; }
        }
        if !y.is_finite() {
            return Err(Error::new(ErrorKind::InvalidInput, "Parametric curve result is not finite"));
        }
        data.push(((y + 0.5) as i16).clamp(0, 255) as u8);
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iccprofile::{ParametricCurve, S15Fixed16Number};

    fn s15(val: f64) -> S15Fixed16Number {
        S15Fixed16Number::new(val)
    }

    #[test]
    fn output_reservation_failure_returns_error() {
        let curve = ParametricCurve { funtion_type: 0, vals: vec![s15(1.0)] };
        let result = crate::allocation_probe::fail_once(3, ||
            transration_prametic_curve(&[0, 128, 255], 3, curve));
        assert!(result.is_err());
        assert_eq!(crate::allocation_probe::failure_hits(), 1);
    }

    // --- type 0x000: y = x^gamma ---

    #[test]
    fn test_type0_gamma1_identity() {
        // gamma=1.0 → y = x
        let curve = ParametricCurve { funtion_type: 0x000, vals: vec![s15(1.0)] };
        let buf = vec![0u8, 100u8, 255u8];
        let result = transration_prametic_curve(&buf, 3, curve).unwrap();
        assert_eq!(result, vec![0, 100, 255]);
    }

    #[test]
    fn test_type0_gamma2() {
        // gamma=2.0, x=10 → y=100
        let curve = ParametricCurve { funtion_type: 0x000, vals: vec![s15(2.0)] };
        let result = transration_prametic_curve(&[10u8], 1, curve).unwrap();
        assert_eq!(result[0], 100);
    }

    #[test]
    fn test_type0_clamp_overflow() {
        // gamma=2.0, x=200 → y=40000 → clamp to 255
        let curve = ParametricCurve { funtion_type: 0x000, vals: vec![s15(2.0)] };
        let result = transration_prametic_curve(&[200u8], 1, curve).unwrap();
        assert_eq!(result[0], 255);
    }

    // --- type 0x001: y = (a*x + b)^gamma if x >= -b/a, else 0 ---

    #[test]
    fn test_type1_linear_branch() {
        // gamma=1.0, a=1.0, b=0.0 → y = x for x >= 0
        let curve = ParametricCurve {
            funtion_type: 0x001,
            vals: vec![s15(1.0), s15(1.0), s15(0.0)],
        };
        let result = transration_prametic_curve(&[100u8], 1, curve).unwrap();
        assert_eq!(result[0], 100);
    }

    #[test]
    fn test_type1_below_threshold() {
        // gamma=2.0, a=1.0, b=200.0 → threshold = -200/1 = -200
        // x=100 >= -200 → y = (100 + 200)^2 = 90000 → clamp 255
        let curve = ParametricCurve {
            funtion_type: 0x001,
            vals: vec![s15(2.0), s15(1.0), s15(200.0)],
        };
        let result = transration_prametic_curve(&[100u8], 1, curve).unwrap();
        assert_eq!(result[0], 255);
    }

    // --- type 0x002: y = (a*x + b)^gamma + c if x >= -b/a, else c ---

    #[test]
    fn test_type2_above_threshold() {
        // gamma=1.0, a=1.0, b=0.0, c=10.0 → y = x + 10 for x >= 0
        let curve = ParametricCurve {
            funtion_type: 0x002,
            vals: vec![s15(1.0), s15(1.0), s15(0.0), s15(10.0)],
        };
        let result = transration_prametic_curve(&[50u8], 1, curve).unwrap();
        assert_eq!(result[0], 60);
    }

    #[test]
    fn test_type2_below_threshold_returns_c() {
        // a=1.0, b=-200.0 → threshold = 200; x=100 < 200 → y = c = 50
        let curve = ParametricCurve {
            funtion_type: 0x002,
            vals: vec![s15(1.0), s15(1.0), s15(-200.0), s15(50.0)],
        };
        let result = transration_prametic_curve(&[100u8], 1, curve).unwrap();
        assert_eq!(result[0], 50);
    }

    // --- type 0x003: y = (a*x + b)^gamma if x >= d, else c*x ---

    #[test]
    fn test_type3_above_threshold() {
        // gamma=1.0, a=1.0, b=0.0, c=0.5, d=50.0
        // x=100 >= 50 → y = (100)^1 = 100
        let curve = ParametricCurve {
            funtion_type: 0x003,
            vals: vec![s15(1.0), s15(1.0), s15(0.0), s15(0.5), s15(50.0)],
        };
        let result = transration_prametic_curve(&[100u8], 1, curve).unwrap();
        assert_eq!(result[0], 100);
    }

    #[test]
    fn test_type3_below_threshold() {
        // gamma=1.0, a=1.0, b=0.0, c=2.0, d=50.0
        // x=10 < 50 → y = 2.0 * 10 = 20
        let curve = ParametricCurve {
            funtion_type: 0x003,
            vals: vec![s15(1.0), s15(1.0), s15(0.0), s15(2.0), s15(50.0)],
        };
        let result = transration_prametic_curve(&[10u8], 1, curve).unwrap();
        assert_eq!(result[0], 20);
    }

    // --- type 0x004: y = (a*x + b)^gamma + e if x >= d, else c*x + f ---

    #[test]
    fn test_type4_above_threshold() {
        // gamma=1.0, a=1.0, b=0.0, c=1.0, d=50.0, e=5.0, f=0.0
        // x=100 >= 50 → y = (100)^1 + 5 = 105
        let curve = ParametricCurve {
            funtion_type: 0x004,
            vals: vec![s15(1.0), s15(1.0), s15(0.0), s15(1.0), s15(50.0), s15(5.0), s15(0.0)],
        };
        let result = transration_prametic_curve(&[100u8], 1, curve).unwrap();
        assert_eq!(result[0], 105);
    }

    #[test]
    fn test_type4_below_threshold() {
        // gamma=1.0, a=1.0, b=0.0, c=1.0, d=50.0, e=0.0, f=10.0
        // x=20 < 50 → y = 1*20 + 10 = 30
        let curve = ParametricCurve {
            funtion_type: 0x004,
            vals: vec![s15(1.0), s15(1.0), s15(0.0), s15(1.0), s15(50.0), s15(0.0), s15(10.0)],
        };
        let result = transration_prametic_curve(&[20u8], 1, curve).unwrap();
        assert_eq!(result[0], 30);
    }

    // --- default (unknown type): y = x ---

    #[test]
    fn test_unknown_type_passthrough() {
        let curve = ParametricCurve { funtion_type: 0xFF, vals: vec![] };
        let buf = vec![42u8, 128u8];
        let result = transration_prametic_curve(&buf, 2, curve).unwrap();
        assert_eq!(result, vec![42, 128]);
    }

    // --- error case ---

    #[test]
    fn test_data_shortage_error() {
        let curve = ParametricCurve { funtion_type: 0x000, vals: vec![s15(1.0)] };
        let buf = vec![1u8, 2u8];
        let result = transration_prametic_curve(&buf, 5, curve);
        assert!(result.is_err());
    }
}

