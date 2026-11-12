//! Independent regression cases for legacy floating-point helper guards.
use icc_profile::cms::transration::gamma::transration_prametic_curve;
use icc_profile::cms::ColorMatrix3D;
use icc_profile::{ParametricCurve, S15Fixed16Number};
use std::io::ErrorKind;

fn curve(kind: u16, values: &[f64]) -> ParametricCurve {
    ParametricCurve {
        funtion_type: kind,
        vals: values.iter().copied().map(S15Fixed16Number::new).collect(),
    }
}

#[test]
fn inverse_rejects_nonfinite_inputs_determinants_and_outputs() {
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for index in 0..9 {
            let mut e = identity;
            e[index] = value;
            assert!(ColorMatrix3D::invese(&ColorMatrix3D { e }).is_none());
            assert!(ColorMatrix3D::inverse(&ColorMatrix3D { e }).is_none());
        }
    }
    for e in [
        [1e200, 0.0, 0.0, 0.0, 1e200, 0.0, 0.0, 0.0, 1e200],
        [1e-310, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        [1.0, 2.0, 3.0, 2.0, 4.0, 6.0, 0.0, 0.0, 1.0],
    ] {
        assert!(ColorMatrix3D::invese(&ColorMatrix3D { e }).is_none());
    }
}

#[test]
fn inverse_preserves_finite_results_without_a_scale_dependent_epsilon() {
    let matrix = ColorMatrix3D {
        e: [1.0, 2.0, 3.0, 0.0, 1.0, 4.0, 5.0, 6.0, 0.0],
    };
    let inverse = ColorMatrix3D::invese(&matrix).unwrap();
    assert_eq!(inverse.e, ColorMatrix3D::inverse(&matrix).unwrap().e);
    assert_eq!(
        inverse.e,
        [-24.0, 18.0, 5.0, 20.0, -15.0, -4.0, -5.0, 4.0, 1.0]
    );
    let tiny = ColorMatrix3D {
        e: [1e-30, 0.0, 0.0, 0.0, 1e-30, 0.0, 0.0, 0.0, 1e-30],
    };
    let inverse = ColorMatrix3D::invese(&tiny).unwrap();
    for index in [0, 4, 8] {
        assert!((inverse.e[index] * 1e-30 - 1.0).abs() < 1e-14);
    }
}

#[test]
fn gamma_rejects_zero_divisors_invalid_powers_and_overflow() {
    for kind in [1, 2] {
        let error =
            transration_prametic_curve(&[10], 1, curve(kind, &[1.0, 0.0, 0.0, 2.0])).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
    for kind in [1, 2, 3, 4] {
        let error = transration_prametic_curve(
            &[10],
            1,
            curve(kind, &[0.5, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        )
        .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
    for (input, gamma) in [(0, -1.0), (255, 32767.0)] {
        let error = transration_prametic_curve(&[input], 1, curve(0, &[gamma])).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
}

#[test]
fn gamma_preserves_byte_domain_clamping_unknown_types_and_empty_batches() {
    assert_eq!(
        transration_prametic_curve(&[128], 1, curve(0, &[2.0])).unwrap(),
        [255]
    );
    assert_eq!(
        transration_prametic_curve(&[10], 1, curve(1, &[2.0, -1.0, 0.0])).unwrap(),
        [100]
    );
    assert_eq!(
        transration_prametic_curve(&[10], 1, curve(3, &[1.0, 0.0, 8.0, 0.0, 0.0])).unwrap(),
        [8]
    );
    assert_eq!(
        transration_prametic_curve(&[10], 1, curve(4, &[1.0, 0.0, 8.0, 0.0, 0.0, 2.0, 0.0]))
            .unwrap(),
        [10]
    );
    assert_eq!(
        transration_prametic_curve(&[0, 128, 255, 12], 3, curve(99, &[])).unwrap(),
        [0, 128, 255]
    );
    assert_eq!(
        transration_prametic_curve(&[10], 1, curve(3, &[0.5, -1.0, 0.0, 2.0, 20.0])).unwrap(),
        [20]
    );
    for kind in 0..=4 {
        assert!(transration_prametic_curve(&[], 0, curve(kind, &[]))
            .unwrap()
            .is_empty());
        assert!(transration_prametic_curve(
            &[],
            0,
            curve(kind, &[0.5, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0])
        )
        .unwrap()
        .is_empty());
    }
}
