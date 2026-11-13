//! Regression tests for the legacy decoder envelope and simple batch boundaries.
use icc_profile::cms::transration::{
    rgbxyz::*, rgbyuv::*, xyzlab::*, xyzrgb::*, yuvrgb::*, WhitePoint,
};
use icc_profile::DecodedICCProfile;
use std::panic::catch_unwind;

fn profile(size: usize, count: u32, offset: u32, length: u32) -> Vec<u8> {
    let mut data = vec![0; size];
    if size >= 4 {
        data[..4].copy_from_slice(&(size as u32).to_be_bytes());
    }
    if size >= 132 {
        data[128..132].copy_from_slice(&count.to_be_bytes());
    }
    if size >= 144 {
        data[132..136].copy_from_slice(b"rXYZ");
        data[136..140].copy_from_slice(&offset.to_be_bytes());
        data[140..144].copy_from_slice(&length.to_be_bytes());
    }
    data
}

#[test]
fn malformed_decoder_envelopes_return_errors_without_panics() {
    for size in [0, 4, 127, 128, 129, 130, 131] {
        let data = profile(size, 0, 0, 0);
        assert!(catch_unwind(|| DecodedICCProfile::new(&data))
            .expect("decoder panicked")
            .is_err());
    }
    for (size, count, offset, length) in [
        (144, 1, 1000, 12),
        (144, 1, 144, 12),
        (144, 1, 140, 12),
        (144, 1, u32::MAX, u32::MAX),
        (144, 2, 0, 0),
        (144, u32::MAX, 0, 0),
        (144, 1, 144, 0),
    ] {
        let data = profile(size, count, offset, length);
        assert!(catch_unwind(|| DecodedICCProfile::new(&data))
            .expect("decoder panicked")
            .is_err());
    }
}

#[test]
fn valid_decoder_envelopes_and_bounded_payloads() {
    assert!(DecodedICCProfile::new(&profile(132, 0, 0, 0))
        .unwrap()
        .tags
        .is_empty());
    let mut data = profile(164, 1, 144, 20);
    data[144..148].copy_from_slice(b"XYZ ");
    assert!(DecodedICCProfile::new(&data)
        .unwrap()
        .tags
        .contains_key("rXYZ"));
    // A tag must not borrow bytes from the following payload to satisfy its own reads.
    data[140..144].copy_from_slice(&12u32.to_be_bytes());
    assert!(catch_unwind(|| DecodedICCProfile::new(&data))
        .unwrap()
        .is_err());
}

#[test]
fn all_simple_batches_check_overflow_short_empty_and_surplus_input() {
    macro_rules! check {
        ($function:ident, $mode:expr, $input:expr, $output:expr) => {{
            let mode = $mode;
            for entries in [usize::MAX, usize::MAX / 3 + 1, usize::MAX / 4 + 1, 1] {
                assert!(
                    catch_unwind(|| $function(&[], entries, &mode))
                        .expect("batch panicked")
                        .is_err(),
                    stringify!($function)
                );
            }
            assert!($function(&[], 0, &mode).unwrap().is_empty());
            assert!($function(&[0; $input - 1], 1, &mode).is_err());
            let exact = $function(&[32; $input], 1, &mode).unwrap();
            assert_eq!(exact.len(), $output);
            assert_eq!(exact, $function(&[32; $input + 7], 1, &mode).unwrap());
        }};
    }
    check!(rgb_to_xyz_entries_f64, RGBToXYZCoefficient::SrgbD65, 3, 3);
    check!(rgba_to_xyz_entries_f64, RGBToXYZCoefficient::SrgbD65, 4, 3);
    check!(rgb_to_xyz_entries, RGBToXYZCoefficient::SrgbD65, 3, 3);
    check!(rgba_to_xyz_entries, RGBToXYZCoefficient::SrgbD65, 4, 3);
    check!(xyz_to_rgb_entries, XYZtoRGBCoefficient::SrgbD65, 3, 3);
    check!(xyz_to_rgba_entries, XYZtoRGBCoefficient::SrgbD65, 4, 4);
    check!(rgb_to_yuv_entries, RGBToYUVCoefficient::Bt601, 3, 3);
    check!(
        yuv_to_rgba_entries_from_rgb,
        RGBToYUVCoefficient::Bt601,
        4,
        3
    );
    check!(yuv_to_rgb_entries, YUVToRGBCoefficient::Bt601, 3, 3);
    check!(
        yuv_to_rgba_entries_from_yuv,
        YUVToRGBCoefficient::Bt601,
        3,
        4
    );
    check!(xyz_to_lab_entries, WhitePoint::d65(), 3, 3);
    check!(xyz_to_lab_entries_u16, WhitePoint::d65(), 3, 3);
    check!(xyz_to_lab_entries_f64, WhitePoint::d65(), 3, 3);
}

#[test]
fn bt601_rounding_and_alpha_stay_profile_free() {
    assert_eq!(
        rgb_to_yuv_entries(&[255, 0, 0], 1, &RGBToYUVCoefficient::Bt601).unwrap(),
        [76, 85, 255]
    );
    assert_eq!(
        yuv_to_rgb_entries(&[128, 128, 128], 1, &YUVToRGBCoefficient::Bt601).unwrap(),
        [128, 128, 128]
    );
    assert_eq!(
        yuv_to_rgba_entries_from_yuv(&[128, 128, 128], 1, &YUVToRGBCoefficient::Bt601).unwrap(),
        [128, 128, 128, 255]
    );
    assert_eq!(
        xyz_to_rgba_entries(&[0, 0, 0, 17], 1, &XYZtoRGBCoefficient::SrgbD65).unwrap(),
        [0, 0, 0, 255]
    );
}

#[test]
fn legacy_gamma_checks_coefficients_but_keeps_byte_domain_and_empty_behavior() {
    use icc_profile::cms::transration::gamma::transration_prametic_curve;
    use icc_profile::{ParametricCurve, S15Fixed16Number};
    for (kind, required) in [(0, 1), (1, 3), (2, 4), (3, 5), (4, 7)] {
        for count in 0..required {
            let curve = || ParametricCurve {
                funtion_type: kind,
                vals: (0..count).map(|_| S15Fixed16Number::new(1.0)).collect(),
            };
            assert!(
                catch_unwind(|| transration_prametic_curve(&[128], 1, curve()))
                    .unwrap()
                    .is_err()
            );
            assert!(transration_prametic_curve(&[], 0, curve())
                .unwrap()
                .is_empty());
        }
    }
    let curve = ParametricCurve {
        funtion_type: 0,
        vals: vec![S15Fixed16Number::new(2.0)],
    };
    assert_eq!(transration_prametic_curve(&[128], 1, curve).unwrap(), [255]);
    let curve = ParametricCurve {
        funtion_type: 99,
        vals: vec![],
    };
    assert_eq!(transration_prametic_curve(&[128], 1, curve).unwrap(), [128]);
}
