use icc_profile::utils::{ciede2000, delta_e76};
use std::path::{Path, PathBuf};

type Lab = (f64, f64, f64);

fn official_cases() -> Vec<(Lab, Lab, f64)> {
    let path = std::env::var_os("CIEDE2000_TEST_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("test_data")
                .join("ciede2000testdata.txt")
        });
    std::fs::read_to_string(path)
        .expect("the Sharma supplementary data file must be present")
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.is_empty() && !line.starts_with('#')
        })
        .map(|line| {
            let values: Vec<f64> = line
                .split_whitespace()
                .map(|value| value.parse().expect("official data must be numeric"))
                .collect();
            assert_eq!(values.len(), 7, "unexpected official data row: {line}");
            (
                (values[0], values[1], values[2]),
                (values[3], values[4], values[5]),
                values[6],
            )
        })
        .collect()
}

#[test]
#[ignore = "requires the ignored Sharma supplementary fixture"]
fn sharma_supplementary_cases() {
    let cases = official_cases();
    assert_eq!(cases.len(), 34);
    let mut max_error = 0.0_f64;
    for (first, second, expected) in cases {
        let actual = ciede2000(&first, &second);
        let reverse = ciede2000(&second, &first);
        max_error = max_error.max((actual - expected).abs());
        max_error = max_error.max((reverse - expected).abs());
        assert!(
            (actual - expected).abs() <= 5e-5,
            "CIEDE2000 mismatch: {first:?} vs {second:?}: got {actual:.8}, expected {expected:.4}"
        );
        assert!(
            (reverse - expected).abs() <= 5e-5,
            "reverse CIEDE2000 mismatch: {second:?} vs {first:?}: got {reverse:.8}, expected {expected:.4}"
        );
    }
    eprintln!("Sharma CIEDE2000 maximum absolute error (both directions): {max_error:.8}");
}

#[test]
fn zero_chroma_and_delta_e76_are_supported() {
    let gray_a = (50.0, 0.0, 0.0);
    let gray_b = (60.0, 0.0, 0.0);
    let gray_difference = ciede2000(&gray_a, &gray_b);
    assert!(gray_difference.is_finite() && gray_difference > 0.0);
    assert_eq!(gray_difference, ciede2000(&gray_b, &gray_a));
    assert_eq!(delta_e76(&gray_a, &gray_b), 10.0);

    let gray_and_color = (50.0, 0.0, 0.0);
    let color = (50.0, 3.0, 4.0);
    assert!(ciede2000(&gray_and_color, &color).is_finite());
}

#[test]
fn signed_zero_has_no_chromatic_difference() {
    let positive = (50.0, 0.0, 0.0);
    let negative_a = (50.0, -0.0, 0.0);
    let negative_b = (50.0, 0.0, -0.0);
    assert_eq!(ciede2000(&positive, &negative_a), 0.0);
    assert_eq!(ciede2000(&positive, &negative_b), 0.0);
}

#[test]
fn hue_wrap_and_symmetry_are_stable() {
    let near_zero = (
        50.0,
        40.0 * (1.0_f64.to_radians()).cos(),
        40.0 * (1.0_f64.to_radians()).sin(),
    );
    let near_360 = (
        50.0,
        40.0 * (-1.0_f64.to_radians()).cos(),
        40.0 * (-1.0_f64.to_radians()).sin(),
    );
    let wrapped = ciede2000(&near_zero, &near_360);
    let reverse = ciede2000(&near_360, &near_zero);
    assert!(wrapped.is_finite() && wrapped < 1.0);
    assert!((wrapped - reverse).abs() < 1e-12);
}

#[test]
fn hue_boundary_is_finite_on_both_sides() {
    let at_boundary = (50.0, -40.0, 0.0);
    let just_before = (
        50.0,
        -40.0 * (179.999_f64.to_radians()).cos(),
        40.0 * 179.999_f64.to_radians().sin(),
    );
    let just_after = (
        50.0,
        -40.0 * (180.001_f64.to_radians()).cos(),
        40.0 * 180.001_f64.to_radians().sin(),
    );
    for other in [just_before, just_after] {
        assert!(ciede2000(&at_boundary, &other).is_finite());
        assert!(ciede2000(&other, &at_boundary).is_finite());
    }
}

#[test]
fn hue_mean_discontinuity_follows_the_specification() {
    fn lab_at_hue(hue_degrees: f64) -> Lab {
        let hue = hue_degrees.to_radians();
        (50.0, 2.5 * hue.cos(), 2.5 * hue.sin())
    }

    let reference = lab_at_hue(143.0);
    let below = lab_at_hue(323.0 - 1e-6_f64.to_degrees());
    let above = lab_at_hue(323.0 + 1e-6_f64.to_degrees());
    let below_difference = ciede2000(&reference, &below);
    let above_difference = ciede2000(&reference, &above);
    let discontinuity = (below_difference - above_difference).abs();
    assert!(
        (discontinuity - 0.2734).abs() < 0.001,
        "unexpected hue boundary jump: {discontinuity}"
    );
}

#[test]
fn non_finite_input_is_reported_without_changing_the_api() {
    let finite = (50.0, 1.0, 2.0);
    assert!(ciede2000(&finite, &finite).is_finite());
    assert!(ciede2000(&(f64::NAN, 0.0, 0.0), &finite).is_nan());
    assert!(ciede2000(&finite, &(f64::INFINITY, 0.0, 0.0)).is_nan());
}
