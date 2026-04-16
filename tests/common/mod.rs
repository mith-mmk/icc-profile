use icc_profile::utils::{delta_e76, ciede2000};

/// ΔE76で色が許容範囲内か確認 (パニック)
pub fn assert_de76_within(
    actual: &(f64, f64, f64),
    expected: &(f64, f64, f64),
    tolerance: f64,
    name: &str,
) {
    let de = delta_e76(expected, actual);
    assert!(
        de <= tolerance,
        "{}: ΔE76={:.3} exceeds tolerance {:.3}",
        name, de, tolerance
    );
}

/// CIEDE2000で色が許容範囲内か確認 (パニック)
pub fn assert_de2000_within(
    actual: &(f64, f64, f64),
    expected: &(f64, f64, f64),
    tolerance: f64,
    name: &str,
) {
    let de = ciede2000(expected, actual);
    assert!(
        de <= tolerance,
        "{}: ΔE00={:.3} exceeds tolerance {:.3}",
        name, de, tolerance
    );
}

/// ΔE76で色が許容範囲内か (bool返却)
pub fn check_de76_within(
    actual: &(f64, f64, f64),
    expected: &(f64, f64, f64),
    tolerance: f64,
) -> bool {
    let de = delta_e76(expected, actual);
    de <= tolerance
}

/// CIEDE2000で色が許容範囲内か (bool返却)
pub fn check_de2000_within(
    actual: &(f64, f64, f64),
    expected: &(f64, f64, f64),
    tolerance: f64,
) -> bool {
    let de = ciede2000(expected, actual);
    de <= tolerance
}
