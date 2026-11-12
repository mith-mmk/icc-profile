use icc_profile::cms::transration::{rgbxyz::*, rgbyuv::*, xyzrgb::*, yuvrgb::*};
use icc_profile::cms::ColorMatrix3D;
use std::io::ErrorKind;

fn diagonal(value: f64) -> ColorMatrix3D {
    ColorMatrix3D {
        e: [value, 0.0, 0.0, 0.0, value, 0.0, 0.0, 0.0, value],
    }
}

#[test]
fn all_matrix_batches_reject_invalid_coefficients_and_nonfinite_results() {
    for matrix in [
        diagonal(f64::NAN),
        diagonal(f64::INFINITY),
        diagonal(f64::NEG_INFINITY),
        ColorMatrix3D { e: [f64::MAX; 9] },
    ] {
        let rgbxyz = RGBToXYZCoefficient::Other(matrix.clone());
        let xyzrgb = XYZtoRGBCoefficient::Other(matrix.clone());
        let rgbyuv = RGBToYUVCoefficient::Other(matrix.clone());
        let yuvrgb = YUVToRGBCoefficient::Other(matrix);
        macro_rules! invalid {
            ($call:expr) => {
                assert_eq!($call.unwrap_err().kind(), ErrorKind::InvalidInput);
            };
        }
        invalid!(rgb_to_xyz_entries_f64(&[255; 3], 1, &rgbxyz));
        invalid!(rgba_to_xyz_entries_f64(&[255; 4], 1, &rgbxyz));
        invalid!(rgb_to_xyz_entries(&[255; 3], 1, &rgbxyz));
        invalid!(rgba_to_xyz_entries(&[255; 4], 1, &rgbxyz));
        invalid!(xyz_to_rgb_entries(&[255; 3], 1, &xyzrgb));
        invalid!(xyz_to_rgba_entries(&[255; 4], 1, &xyzrgb));
        invalid!(rgb_to_yuv_entries(&[255; 3], 1, &rgbyuv));
        invalid!(yuv_to_rgba_entries_from_rgb(&[255; 4], 1, &rgbyuv));
        invalid!(yuv_to_rgb_entries(&[255; 3], 1, &yuvrgb));
        invalid!(yuv_to_rgba_entries_from_yuv(&[255; 3], 1, &yuvrgb));
        assert!(rgb_to_xyz_entries(&[], 0, &rgbxyz).unwrap().is_empty());
        assert!(rgb_to_xyz_entries_f64(&[], 0, &rgbxyz).unwrap().is_empty());
        assert!(rgba_to_xyz_entries(&[], 0, &rgbxyz).unwrap().is_empty());
        assert!(rgba_to_xyz_entries_f64(&[], 0, &rgbxyz).unwrap().is_empty());
        assert!(xyz_to_rgb_entries(&[], 0, &xyzrgb).unwrap().is_empty());
        assert!(xyz_to_rgba_entries(&[], 0, &xyzrgb).unwrap().is_empty());
        assert!(rgb_to_yuv_entries(&[], 0, &rgbyuv).unwrap().is_empty());
        assert!(yuv_to_rgba_entries_from_rgb(&[], 0, &rgbyuv)
            .unwrap()
            .is_empty());
        assert!(yuv_to_rgb_entries(&[], 0, &yuvrgb).unwrap().is_empty());
        assert!(yuv_to_rgba_entries_from_yuv(&[], 0, &yuvrgb)
            .unwrap()
            .is_empty());
    }
}

#[test]
fn matrix_batches_keep_finite_clamping_truncation_and_layout() {
    let e = [2.0, 0.0, 0.0, 0.0, -2.0, 0.0, 0.0, 0.0, 0.5];
    let rgbxyz = RGBToXYZCoefficient::Other(ColorMatrix3D { e });
    let xyzrgb = XYZtoRGBCoefficient::Other(ColorMatrix3D { e });
    assert_eq!(
        rgb_to_xyz_entries(&[200, 100, 3], 1, &rgbxyz).unwrap(),
        [255, 0, 1]
    );
    assert_eq!(
        rgba_to_xyz_entries(&[200, 100, 3, 27], 1, &rgbxyz).unwrap(),
        [255, 0, 1]
    );
    assert_eq!(
        xyz_to_rgb_entries(&[200, 100, 3], 1, &xyzrgb).unwrap(),
        [255, 0, 1]
    );
    assert_eq!(
        xyz_to_rgba_entries(&[200, 100, 3, 27], 1, &xyzrgb).unwrap(),
        [255, 0, 1, 255]
    );
    let normalized = [400.0 / 255.0, -200.0 / 255.0, 1.5 / 255.0];
    assert_eq!(
        rgb_to_xyz_entries_f64(&[200, 100, 3], 1, &rgbxyz).unwrap(),
        normalized
    );
    assert_eq!(
        rgba_to_xyz_entries_f64(&[200, 100, 3, 27], 1, &rgbxyz).unwrap(),
        normalized
    );
    assert_eq!(
        rgb_to_yuv_entries(&[255, 0, 0], 1, &RGBToYUVCoefficient::Bt601).unwrap(),
        [76, 85, 255]
    );
    assert_eq!(
        yuv_to_rgb_entries(&[76, 85, 255], 1, &YUVToRGBCoefficient::Bt601).unwrap(),
        [254, 0, 0]
    );
    assert_eq!(
        yuv_to_rgba_entries_from_yuv(&[76, 85, 255], 1, &YUVToRGBCoefficient::Bt601).unwrap(),
        [254, 0, 0, 255]
    );
}

#[test]
fn checked_matrix_scalar_rejects_each_invalid_component_and_finite_overflow() {
    let identity = diagonal(1.0);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for input in [(value, 1.0, 1.0), (1.0, value, 1.0), (1.0, 1.0, value)] {
            assert_eq!(
                identity
                    .try_convert_3d(input.0, input.1, input.2)
                    .unwrap_err()
                    .kind(),
                ErrorKind::InvalidInput
            );
        }
        for index in 0..9 {
            let mut matrix = identity.clone();
            matrix.e[index] = value;
            assert!(matrix.try_convert_3d(1.0, 1.0, 1.0).is_err());
        }
    }
    assert!(diagonal(2.0).try_convert_3d(f64::MAX, 0.0, 0.0).is_err());
    assert_eq!(
        identity.try_convert_3d(-10.0, 500.0, 0.25).unwrap(),
        (-10.0, 500.0, 0.25)
    );
    // The old scalar remains an unchecked IEEE operation.
    assert!(identity.convert_3d(f64::NAN, 0.0, 0.0).0.is_nan());
}
