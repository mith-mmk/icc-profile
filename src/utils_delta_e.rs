/// ΔE76: L*a*b* 空間上のユークリッド距離
pub fn delta_e76(lab_ref: &(f64, f64, f64), lab_test: &(f64, f64, f64)) -> f64 {
    let (l_ref, a_ref, b_ref) = lab_ref;
    let (l_test, a_test, b_test) = lab_test;
    let dl = l_ref - l_test;
    let da = a_ref - a_test;
    let db = b_ref - b_test;
    (dl.powi(2) + da.powi(2) + db.powi(2)).sqrt()
}

/// CIEDE2000: ICC標準の色差
pub fn ciede2000(lab_ref: &(f64, f64, f64), lab_test: &(f64, f64, f64)) -> f64 {
    let (l_ref, a_ref, b_ref) = lab_ref;
    let (l_test, a_test, b_test) = lab_test;
    let c_ab_ref = (a_ref.powi(2) + b_ref.powi(2)).sqrt();
    let c_ab_test = (a_test.powi(2) + b_test.powi(2)).sqrt();
    let c_ab_mean = (c_ab_ref + c_ab_test) / 2.0;
    let g = 0.5 * (1.0 - (c_ab_mean.powi(7) / (c_ab_mean.powi(7) + 25.0_f64.powi(7))).sqrt());
    let a_ref_prime = (1.0 + g) * a_ref;
    let a_test_prime = (1.0 + g) * a_test;
    let c_ab_prime_ref = (a_ref_prime.powi(2) + b_ref.powi(2)).sqrt();
    let c_ab_prime_test = (a_test_prime.powi(2) + b_test.powi(2)).sqrt();
    let h_ab_prime_ref = b_ref.atan2(a_ref_prime).to_degrees();
    let h_ab_prime_test = b_test.atan2(a_test_prime).to_degrees();
    let h_ab_prime_ref = if h_ab_prime_ref < 0.0 { h_ab_prime_ref + 360.0 } else { h_ab_prime_ref };
    let h_ab_prime_test = if h_ab_prime_test < 0.0 { h_ab_prime_test + 360.0 } else { h_ab_prime_test };
    let dl = l_test - l_ref;
    let dc_ab_prime = c_ab_prime_test - c_ab_prime_ref;
    let mut dh_ab_prime_deg = h_ab_prime_test - h_ab_prime_ref;
    if dh_ab_prime_deg > 180.0 { dh_ab_prime_deg -= 360.0; } else if dh_ab_prime_deg < -180.0 { dh_ab_prime_deg += 360.0; }
    let dh_ab_prime = dh_ab_prime_deg.to_radians();
    let dh_total = 2.0 * c_ab_prime_ref * c_ab_prime_test * (dh_ab_prime / 2.0).sin();
    let l_mean = (l_ref + l_test) / 2.0;
    let c_ab_prime_mean = (c_ab_prime_ref + c_ab_prime_test) / 2.0;
    let mut h_ab_prime_mean = (h_ab_prime_ref + h_ab_prime_test) / 2.0;
    if (h_ab_prime_ref - h_ab_prime_test).abs() > 180.0 {
        h_ab_prime_mean = if h_ab_prime_mean < 180.0 { h_ab_prime_mean + 180.0 } else { h_ab_prime_mean - 180.0 };
    }
    let sl = 1.0 + 0.015 * (l_mean - 50.0).powi(2) / (20.0 + (l_mean - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * c_ab_prime_mean;
    let h_rad = h_ab_prime_mean.to_radians();
    let t = if h_rad >= 0.0 && h_rad <= 30.0_f64.to_radians() {
        0.56 + 0.2 * (h_rad - 30.0_f64.to_radians()).cos().abs()
    } else if h_rad > 30.0_f64.to_radians() && h_rad <= 90.0_f64.to_radians() {
        0.56 + 0.2 * (h_rad - 30.0_f64.to_radians()).cos().abs()
    } else if h_rad > 90.0_f64.to_radians() && h_rad <= 165.0_f64.to_radians() {
        0.34 + 0.2 * (h_rad - 165.0_f64.to_radians()).cos().abs()
    } else if h_rad > 165.0_f64.to_radians() && h_rad <= 345.0_f64.to_radians() {
        0.34 + 0.2 * (h_rad - 165.0_f64.to_radians()).cos().abs()
    } else {
        0.56 + 0.2 * (h_rad + 30.0_f64.to_radians()).cos().abs()
    };
    let sh = 1.0 + 0.015 * c_ab_prime_mean * t;
    let rc = 2.0 * (c_ab_prime_mean.powi(7) / (c_ab_prime_mean.powi(7) + 25.0_f64.powi(7))).sqrt();
    let rt = -rc * (2.0 * h_ab_prime_mean.to_radians()).sin();
    let de_l = dl / sl;
    let de_c = dc_ab_prime / sc;
    let de_h = dh_total / sh;
    (de_l.powi(2) + de_c.powi(2) + de_h.powi(2) + rt * de_c * de_h).sqrt()
}
