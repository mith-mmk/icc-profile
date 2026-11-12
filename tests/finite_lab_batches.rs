use icc_profile::cms::transration::{labxyz::*, xyzlab::*, WhitePoint};
use std::io::ErrorKind;

#[test]
fn lab_batches_reject_invalid_whitepoints_and_keep_empty_behavior() {
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for index in 0..3 {
            let mut values = [1.0; 3];
            values[index] = value;
            let wp = WhitePoint::new(values[0], values[1], values[2]);
            macro_rules! check {
                ($function:ident, $input:expr) => {{
                    assert_eq!(
                        $function($input, 1, &wp).unwrap_err().kind(),
                        ErrorKind::InvalidInput
                    );
                    assert!($function(&[], 0, &wp).unwrap().is_empty());
                }};
            }
            check!(lab_to_xyz_entries, &[128, 128, 128]);
            check!(lab_to_xyz_entries_f64, &[50.0, 0.0, 0.0]);
            check!(xyz_to_lab_entries, &[128, 128, 128]);
            check!(xyz_to_lab_entries_u16, &[128, 128, 128]);
            check!(xyz_to_lab_entries_f64, &[128, 128, 128]);
        }
    }
}

#[test]
fn lab_batches_reject_nonfinite_inputs_and_overflowing_results() {
    let wp = WhitePoint::d65();
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for index in 0..3 {
            let mut values = [50.0, 0.0, 0.0];
            values[index] = value;
            assert!(lab_to_xyz_entries_f64(&values, 1, &wp).is_err());
        }
    }
    for values in [
        [f64::MAX, 0.0, 0.0],
        [50.0, f64::MAX, 0.0],
        [50.0, 0.0, -f64::MAX],
    ] {
        assert!(lab_to_xyz_entries_f64(&values, 1, &wp).is_err());
    }
    let large = WhitePoint::new(f64::MAX, f64::MAX, f64::MAX);
    assert!(lab_to_xyz_entries(&[255; 3], 1, &large).is_err());
    let tiny = WhitePoint::new(1e-310, 1.0, 1.0);
    assert!(xyz_to_lab_entries(&[255; 3], 1, &tiny).is_err());
    assert!(xyz_to_lab_entries_u16(&[255; 3], 1, &tiny).is_err());
    assert!(xyz_to_lab_entries_f64(&[255; 3], 1, &tiny).is_err());
}

#[test]
fn lab_batch_layout_rounding_clipping_and_surplus_stay_compatible() {
    let wp = WhitePoint::new(1.0, 1.0, 1.0);
    assert_eq!(
        xyz_to_lab_entries_f64(&[0, 0, 0, 255, 255, 255, 16], 2, &wp).unwrap(),
        [0.0, 0.0, 0.0, 100.0, 0.0, 0.0]
    );
    assert_eq!(
        xyz_to_lab_entries(&[0, 0, 0, 255, 255, 255], 2, &wp).unwrap(),
        [0, 128, 128, 255, 128, 128]
    );
    assert_eq!(
        xyz_to_lab_entries_u16(&[0, 0, 0, 255, 255, 255], 2, &wp).unwrap(),
        [0, 32640, 32640, 65535, 32640, 32640]
    );
    assert_eq!(
        lab_to_xyz_entries(&[0, 128, 128, 255, 128, 128, 16], 2, &wp).unwrap(),
        [0, 0, 0, 255, 255, 255]
    );
    assert_eq!(
        lab_to_xyz_entries_f64(&[0.0, 0.0, 0.0, 100.0, 0.0, 0.0, f64::NAN], 2, &wp).unwrap(),
        [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
    );
    let huge = WhitePoint::new(f64::MAX / 8.0, f64::MAX / 8.0, f64::MAX / 8.0);
    assert_eq!(
        lab_to_xyz_entries(&[255, 128, 128], 1, &huge).unwrap(),
        [255; 3]
    );
}

#[test]
fn checked_lab_scalars_and_batch_capacity_boundaries() {
    let wp = WhitePoint::new(1.0, 1.0, 1.0);
    assert_eq!(
        try_lab_to_xyz_wp(100.0, 0.0, 0.0, &wp).unwrap(),
        (1.0, 1.0, 1.0)
    );
    assert_eq!(
        try_xyz_to_lab_wp(1.0, 1.0, 1.0, &wp).unwrap(),
        (100.0, 0.0, 0.0)
    );
    assert!(try_xyz_to_lab_wp(0.0, f64::NAN, 0.0, &wp).is_err());
    assert!(try_lab_to_xyz_wp(0.0, 0.0, f64::INFINITY, &wp).is_err());
    assert!(try_xyz_to_lab_wp(f64::MAX, 0.0, 0.0, &WhitePoint::new(1e-310, 1.0, 1.0)).is_err());
    // Finite out-of-gamut coordinates are valid; they are not silently clamped.
    assert!(try_lab_to_xyz_wp(-10.0, 0.0, 0.0, &wp).unwrap().1 < 0.0);
    assert!(try_xyz_to_lab_wp(-0.1, -0.1, -0.1, &wp).unwrap().0 < 0.0);
    for entries in [usize::MAX, usize::MAX / 3 + 1, 1] {
        assert!(lab_to_xyz_entries(&[], entries, &wp).is_err());
        assert!(lab_to_xyz_entries_f64(&[], entries, &wp).is_err());
        assert!(xyz_to_lab_entries(&[], entries, &wp).is_err());
        assert!(xyz_to_lab_entries_u16(&[], entries, &wp).is_err());
        assert!(xyz_to_lab_entries_f64(&[], entries, &wp).is_err());
    }
}
