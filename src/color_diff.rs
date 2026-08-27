//! CIE color-difference metrics.
//!
//! The CIEDE2000 implementation follows the equations in Sharma, Wu, and
//! Dalal, *The CIEDE2000 Color-Difference Formula: Implementation Notes,
//! Supplementary Test Data, and Mathematical Observations* (2005).  Keeping
//! the implementation here gives the legacy utility APIs one canonical
//! implementation.

/// CIE 1976 color difference in L*a*b* space.
pub fn delta_e76(lab_ref: &(f64, f64, f64), lab_test: &(f64, f64, f64)) -> f64 {
    let dl = lab_test.0 - lab_ref.0;
    let da = lab_test.1 - lab_ref.1;
    let db = lab_test.2 - lab_ref.2;
    dl.hypot(da).hypot(db)
}

/// CIEDE2000 color difference in L*a*b* space.
///
/// This API has historically returned an `f64`, so non-finite input is
/// represented by `NaN` rather than changing the public API to a `Result`.
pub fn ciede2000(lab_ref: &(f64, f64, f64), lab_test: &(f64, f64, f64)) -> f64 {
    if !lab_ref.0.is_finite()
        || !lab_ref.1.is_finite()
        || !lab_ref.2.is_finite()
        || !lab_test.0.is_finite()
        || !lab_test.1.is_finite()
        || !lab_test.2.is_finite()
    {
        return f64::NAN;
    }

    let (l1, a1, b1) = *lab_ref;
    let (l2, a2, b2) = *lab_test;
    let c1 = a1.hypot(b1);
    let c2 = a2.hypot(b2);
    let c_bar = (c1 + c2) * 0.5;
    let c_bar7 = c_bar.powi(7);
    let twenty_five7 = 25.0_f64.powi(7);
    let g = 0.5 * (1.0 - (c_bar7 / (c_bar7 + twenty_five7)).sqrt());

    let a1_prime = (1.0 + g) * a1;
    let a2_prime = (1.0 + g) * a2;
    let c1_prime = a1_prime.hypot(b1);
    let c2_prime = a2_prime.hypot(b2);
    let h1_prime = hue_degrees(a1_prime, b1, c1_prime);
    let h2_prime = hue_degrees(a2_prime, b2, c2_prime);

    let delta_l_prime = l2 - l1;
    let delta_c_prime = c2_prime - c1_prime;
    let delta_h_prime = hue_difference(h1_prime, h2_prime, c1_prime * c2_prime);
    let delta_h_term =
        2.0 * (c1_prime * c2_prime).sqrt() * (0.5 * delta_h_prime.to_radians()).sin();

    let l_bar = (l1 + l2) * 0.5;
    let c_bar_prime = (c1_prime + c2_prime) * 0.5;
    let h_bar_prime = mean_hue(h1_prime, h2_prime, c1_prime * c2_prime);
    let h_bar_radians = h_bar_prime.to_radians();

    let t = 1.0 - 0.17 * (h_bar_radians - 30.0_f64.to_radians()).cos()
        + 0.24 * (2.0 * h_bar_radians).cos()
        + 0.32 * (3.0 * h_bar_radians + 6.0_f64.to_radians()).cos()
        - 0.20 * (4.0 * h_bar_radians - 63.0_f64.to_radians()).cos();
    let l_delta = l_bar - 50.0;
    let s_l = 1.0 + 0.015 * l_delta.powi(2) / (20.0 + l_delta.powi(2)).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_prime;
    let s_h = 1.0 + 0.015 * c_bar_prime * t;

    let c_bar_prime7 = c_bar_prime.powi(7);
    let r_c = 2.0 * (c_bar_prime7 / (c_bar_prime7 + twenty_five7)).sqrt();
    let delta_theta = 30.0 * (-(h_bar_prime - 275.0).powi(2) / 625.0).exp();
    let r_t = -r_c * (2.0 * delta_theta.to_radians()).sin();

    let l_term = delta_l_prime / s_l;
    let c_term = delta_c_prime / s_c;
    let h_term = delta_h_term / s_h;
    (l_term.powi(2) + c_term.powi(2) + h_term.powi(2) + r_t * c_term * h_term).sqrt()
}

fn hue_degrees(a: f64, b: f64, chroma: f64) -> f64 {
    if chroma == 0.0 {
        0.0
    } else {
        let hue = b.atan2(a).to_degrees();
        if hue < 0.0 {
            hue + 360.0
        } else {
            hue
        }
    }
}

fn hue_difference(h1: f64, h2: f64, chroma_product: f64) -> f64 {
    if chroma_product == 0.0 {
        0.0
    } else {
        let difference = h2 - h1;
        if difference.abs() <= 180.0 {
            difference
        } else if difference > 180.0 {
            difference - 360.0
        } else {
            difference + 360.0
        }
    }
}

fn mean_hue(h1: f64, h2: f64, chroma_product: f64) -> f64 {
    if chroma_product == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() <= 180.0 {
        (h1 + h2) * 0.5
    } else if h1 + h2 < 360.0 {
        (h1 + h2 + 360.0) * 0.5
    } else {
        (h1 + h2 - 360.0) * 0.5
    }
}
