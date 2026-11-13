use icc_profile::simple::*;

fn near(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!((a - e).abs() <= tolerance, "{actual:?} != {expected:?}");
    }
}
fn identity() -> ColorMatrix3D {
    ColorMatrix3D {
        e: [1., 0., 0., 0., 1., 0., 0., 0., 1.],
    }
}

#[test]
fn ycbcr_independent_known_vectors_and_all_neutrals() {
    for mode in [YcbcrCoefficients::Bt601, YcbcrCoefficients::Bt709] {
        for gray in 0..=255 {
            assert_eq!(rgb8_to_ycbcr8_full([gray; 3], mode), [gray, 128, 128]);
            assert_eq!(ycbcr8_full_to_rgb8([gray, 128, 128], mode), [gray; 3]);
        }
    }
    assert_eq!(
        rgb8_to_ycbcr8_full([255, 0, 0], YcbcrCoefficients::Bt601),
        [76, 85, 255]
    );
    assert_eq!(
        rgb8_to_ycbcr8_full([255, 0, 0], YcbcrCoefficients::Bt709),
        [54, 99, 255]
    );
    assert_eq!(
        rgb8_to_ycbcr8_full([0, 0, 255], YcbcrCoefficients::Bt601),
        [29, 255, 107]
    );
    assert_eq!(
        ycbcr8_full_to_rgb8([0, 0, 0], YcbcrCoefficients::Bt601),
        [0, 135, 0]
    );
    assert_eq!(
        ycbcr8_full_to_rgb8([255, 255, 255], YcbcrCoefficients::Bt709),
        [255, 172, 255]
    );
}

#[test]
fn buffers_process_every_pixel_and_reject_tail_samples() {
    let rgb = [255, 0, 0, 0, 0, 255];
    assert_eq!(
        rgb8_to_ycbcr8_full_buffer(&rgb, YcbcrCoefficients::Bt601).unwrap(),
        [76, 85, 255, 29, 255, 107]
    );
    let m = identity();
    let white = WhitePoint::new(1., 1., 1.);
    for len in [1, 2, 4, 5, 7] {
        let bytes = vec![0; len];
        let floats = vec![0.; len];
        assert!(
            matches!(rgb8_to_ycbcr8_full_buffer(&bytes, YcbcrCoefficients::Bt601), Err(SimpleError::InvalidBufferLength { samples }) if samples == len)
        );
        assert!(matches!(
            ycbcr8_full_to_rgb8_buffer(&bytes, YcbcrCoefficients::Bt709),
            Err(SimpleError::InvalidBufferLength { .. })
        ));
        for result in [
            linear_rgb_f64_to_xyz_f64_buffer(&floats, &m),
            xyz_f64_to_linear_rgb_f64_buffer(&floats, &m),
            xyz_f64_to_lab_f64_buffer(&floats, &white),
            lab_f64_to_xyz_f64_buffer(&floats, &white),
        ] {
            assert!(matches!(
                result,
                Err(SimpleError::InvalidBufferLength { .. })
            ));
        }
    }
}

#[test]
fn matrix_direction_scale_and_unclipped_values_are_explicit() {
    let forward = ColorMatrix3D {
        e: [2., 0., 0., 0., 3., 0., 0., 0., 4.],
    };
    let reverse = ColorMatrix3D {
        e: [0.5, 0., 0., 0., 1. / 3., 0., 0., 0., 0.25],
    };
    near(
        linear_rgb_f64_to_xyz_f64([-1., 0.5, 2.], &forward).unwrap(),
        [-2., 1.5, 8.],
        0.,
    );
    near(
        xyz_f64_to_linear_rgb_f64([-2., 1.5, 8.], &reverse).unwrap(),
        [-1., 0.5, 2.],
        0.,
    );
    let xyz = linear_rgb_f64_to_xyz_f64_buffer(&[-1., 0.5, 2., 0., 1., 0.], &forward).unwrap();
    assert_eq!(xyz, [-2., 1.5, 8., 0., 3., 0.]);
    assert_eq!(
        xyz_f64_to_linear_rgb_f64_buffer(&xyz, &reverse).unwrap(),
        [-1., 0.5, 2., 0., 1., 0.]
    );
    // Non-symmetric matrix catches accidental transposition.
    let mixed = ColorMatrix3D {
        e: [1., 2., 3., 4., 5., 6., 7., 8., 9.],
    };
    near(
        linear_rgb_f64_to_xyz_f64([1., 0., 0.], &mixed).unwrap(),
        [1., 4., 7.],
        0.,
    );
}

#[test]
fn lab_reference_points_piecewise_thresholds_and_scale() {
    let white = WhitePoint::new(0.9642, 1., 0.8249);
    near(
        xyz_f64_to_lab_f64([white.x, white.y, white.z], &white).unwrap(),
        [100., 0., 0.],
        1e-12,
    );
    near(xyz_f64_to_lab_f64([0.; 3], &white).unwrap(), [0.; 3], 1e-12);
    near(
        lab_f64_to_xyz_f64([100., 0., 0.], &white).unwrap(),
        [white.x, white.y, white.z],
        1e-12,
    );
    // Independent CIE inverse: X is cubic while Y and Z are in the linear branch.
    let delta: f64 = 6. / 29.;
    let fy = 16. / 116.;
    let expected = [
        (fy + 0.2_f64).powi(3) * white.x,
        0.,
        3. * delta * delta * (-0.2) * white.z,
    ];
    near(
        lab_f64_to_xyz_f64([0., 100., 40.], &white).unwrap(),
        expected,
        1e-14,
    );
    for input in [[0., 100., 40.], [120., -30., 80.], [-5., -15., -10.]] {
        let xyz = lab_f64_to_xyz_f64(input, &white).unwrap();
        near(xyz_f64_to_lab_f64(xyz, &white).unwrap(), input, 1e-11);
    }
    let scaled = WhitePoint::new(white.x * 100., 100., white.z * 100.);
    near(
        xyz_f64_to_lab_f64([scaled.x, scaled.y, scaled.z], &scaled).unwrap(),
        [100., 0., 0.],
        1e-12,
    );
    let lab = xyz_f64_to_lab_f64_buffer(&[0., 0., 0., white.x, white.y, white.z], &white).unwrap();
    let xyz = lab_f64_to_xyz_f64_buffer(&lab, &white).unwrap();
    near([xyz[0], xyz[1], xyz[2]], [0.; 3], 1e-12);
    near([xyz[3], xyz[4], xyz[5]], [white.x, white.y, white.z], 1e-12);
}

#[test]
fn nonfinite_input_and_result_are_distinct() {
    let white = WhitePoint::new(1., 1., 1.);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for channel in 0..3 {
            let mut input = [0.; 3];
            input[channel] = value;
            for result in [
                linear_rgb_f64_to_xyz_f64(input, &identity()),
                xyz_f64_to_linear_rgb_f64(input, &identity()),
                xyz_f64_to_lab_f64(input, &white),
                lab_f64_to_xyz_f64(input, &white),
            ] {
                assert!(matches!(result, Err(SimpleError::NonFiniteInput)));
            }
        }
    }
    let huge = ColorMatrix3D { e: [f64::MAX; 9] };
    assert!(matches!(
        linear_rgb_f64_to_xyz_f64([2.; 3], &huge),
        Err(SimpleError::NonFiniteResult)
    ));
    assert!(matches!(
        lab_f64_to_xyz_f64([f64::MAX, 0., 0.], &white),
        Err(SimpleError::NonFiniteResult)
    ));
    let tiny = WhitePoint::new(f64::MIN_POSITIVE, 1., 1.);
    assert!(matches!(
        xyz_f64_to_lab_f64([f64::MAX, 0., 0.], &tiny),
        Err(SimpleError::NonFiniteResult)
    ));
    assert!(matches!(
        lab_f64_to_xyz_f64_buffer(&[0., 0., 0., f64::NAN, 0., 0.], &white),
        Err(SimpleError::NonFiniteInput)
    ));
}

#[test]
fn invalid_parameters_are_rejected_even_for_empty_input() {
    for value in [0., -1., f64::NAN, f64::INFINITY] {
        for channel in 0..3 {
            let mut xyz = [1.; 3];
            xyz[channel] = value;
            let white = WhitePoint::new(xyz[0], xyz[1], xyz[2]);
            assert!(matches!(
                xyz_f64_to_lab_f64_buffer(&[], &white),
                Err(SimpleError::InvalidWhitePoint)
            ));
            assert!(matches!(
                lab_f64_to_xyz_f64_buffer(&[], &white),
                Err(SimpleError::InvalidWhitePoint)
            ));
        }
    }
    for index in 0..9 {
        let mut matrix = identity();
        matrix.e[index] = f64::NAN;
        assert!(matches!(
            linear_rgb_f64_to_xyz_f64_buffer(&[], &matrix),
            Err(SimpleError::InvalidMatrix)
        ));
        assert!(matches!(
            xyz_f64_to_linear_rgb_f64_buffer(&[], &matrix),
            Err(SimpleError::InvalidMatrix)
        ));
    }
    let white = WhitePoint::new(1., 1., 1.);
    assert!(rgb8_to_ycbcr8_full_buffer(&[], YcbcrCoefficients::Bt601)
        .unwrap()
        .is_empty());
    assert!(ycbcr8_full_to_rgb8_buffer(&[], YcbcrCoefficients::Bt709)
        .unwrap()
        .is_empty());
    for result in [
        linear_rgb_f64_to_xyz_f64_buffer(&[], &identity()),
        xyz_f64_to_linear_rgb_f64_buffer(&[], &identity()),
        xyz_f64_to_lab_f64_buffer(&[], &white),
        lab_f64_to_xyz_f64_buffer(&[], &white),
    ] {
        assert!(result.unwrap().is_empty());
    }
}

#[test]
fn d65_matrix_preserves_unclipped_white_xyz() {
    // The built-in matrix is applied directly, with no adaptation or clipping.
    let xyz = linear_rgb_f64_to_xyz_f64([1.; 3], &ColorMatrix3D::d65_rgb_to_xyz()).unwrap();
    near(xyz, [0.950456, 1.0, 1.089058], 1e-12);
    assert!(xyz[2] > 1.);
}

#[test]
fn reverse_bt709_buffer_matches_every_scalar_pixel() {
    let input = [0, 128, 128, 255, 255, 255, 90, 10, 240, 170, 190, 25];
    let output = ycbcr8_full_to_rgb8_buffer(&input, YcbcrCoefficients::Bt709).unwrap();
    assert_eq!(output.len(), input.len());
    for (pixel, converted) in input.chunks_exact(3).zip(output.chunks_exact(3)) {
        assert_eq!(
            converted,
            ycbcr8_full_to_rgb8([pixel[0], pixel[1], pixel[2]], YcbcrCoefficients::Bt709)
        );
    }
}

#[test]
fn all_float_buffers_reject_late_nonfinite_samples() {
    let matrix = identity();
    let white = WhitePoint::new(1., 1., 1.);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for channel in 0..3 {
            let mut input = [0.; 6];
            input[3 + channel] = value;
            for result in [
                linear_rgb_f64_to_xyz_f64_buffer(&input, &matrix),
                xyz_f64_to_linear_rgb_f64_buffer(&input, &matrix),
                xyz_f64_to_lab_f64_buffer(&input, &white),
                lab_f64_to_xyz_f64_buffer(&input, &white),
            ] {
                assert!(matches!(result, Err(SimpleError::NonFiniteInput)));
            }
        }
    }
}
