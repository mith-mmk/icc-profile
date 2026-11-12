use icc_profile::cms::transration::{rgbxyz::*, rgbyuv::*, xyzrgb::*, yuvrgb::*};
use icc_profile::cms::{ColorEntries, ColorMatrix3D};

#[test]
fn rgba_to_ycbcr_alias_preserves_layout_alpha_discard_and_errors() {
    let input = [255, 0, 0, 0, 0, 0, 255, 17, 19];
    let mode = RGBToYUVCoefficient::Bt601;
    assert_eq!(
        rgba_to_yuv_entries(&input, 2, &mode).unwrap(),
        [76, 85, 255, 29, 255, 107]
    );
    assert_eq!(
        rgba_to_yuv_entries(&input, 2, &mode).unwrap(),
        yuv_to_rgba_entries_from_rgb(&input, 2, &mode).unwrap()
    );
    for count in [1, usize::MAX] {
        assert!(rgba_to_yuv_entries(&[], count, &mode).is_err());
    }
    assert!(rgba_to_yuv_entries(&[], 0, &mode).unwrap().is_empty());
    let invalid = RGBToYUVCoefficient::Other(ColorMatrix3D { e: [f64::NAN; 9] });
    assert!(rgba_to_yuv_entries(&input, 1, &invalid).is_err());
}

#[test]
fn both_rgba_outputs_are_opaque_and_xyz_input_keeps_four_byte_stride() {
    assert_eq!(
        yuv_to_rgba_entries_from_yuv(&[0, 128, 128, 76, 85, 255], 2, &YUVToRGBCoefficient::Bt601)
            .unwrap(),
        [0, 0, 0, 255, 254, 0, 0, 255]
    );
    let identity = XYZtoRGBCoefficient::Other(ColorMatrix3D {
        e: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    });
    assert_eq!(
        xyz_to_rgba_entries(&[11, 22, 33, 77, 44, 55, 66, 99], 2, &identity).unwrap(),
        [11, 22, 33, 255, 44, 55, 66, 255]
    );
    assert!(xyz_to_rgba_entries(&[11, 22, 33, 44, 55, 66], 2, &identity).is_err());
    assert_eq!(
        xyz_to_rgb_entries(&[11, 22, 33, 44, 55, 66], 2, &identity).unwrap(),
        [11, 22, 33, 44, 55, 66]
    );
}

#[test]
fn rgb_xyz_stays_matrix_only_linear_light_with_clipped_xyz_bytes() {
    let (_, y, _) = rgb_to_xyz(128, 128, 128);
    assert!((y - 128.0 / 255.0).abs() < 1e-12);
    let white = rgb_to_xyz_from_f64(1.0, 1.0, 1.0);
    assert!(white.2 > 1.0);
    let (r, g, b) = xyz_to_rgb_to_f64(white.0 * 0.5, white.1 * 0.5, white.2 * 0.5);
    for value in [r, g, b] {
        assert!((value - 0.5).abs() < 2e-6);
    }
    assert_eq!(
        rgb_to_xyz_entries(&[255; 3], 1, &RGBToXYZCoefficient::SrgbD65).unwrap()[2],
        255
    );
}

#[test]
fn color_entries_methods_remain_explicitly_unimplemented() {
    let entry = ColorEntries::Rgb24(vec![1, 2, 3]);
    assert!(entry.to_rgb().is_none());
    assert!(entry.to_lab().is_none());
    assert!(entry.to_lab24().is_none());
    assert!(entry.to_lab48().is_none());
    assert!(entry.to_xyz().is_none());
    assert!(entry.to_xyz24().is_none());
    assert!(entry.to_ymck().is_none());
}

#[test]
fn bare_cmyk_preserves_inverted_additive_ymck_convention() {
    use icc_profile::cms::transration::cmykrgb::cmyk_to_rgb;
    assert_eq!(cmyk_to_rgb(0, 0, 0, 0), (0, 0, 0));
    assert_eq!(cmyk_to_rgb(255, 255, 255, 255), (255, 255, 255));
    assert_eq!(cmyk_to_rgb(0, 0, 255, 255), (255, 0, 0));
    assert_eq!(cmyk_to_rgb(255, 0, 0, 255), (0, 0, 255));
    assert_eq!(cmyk_to_rgb(200, 100, 0, 0), (0, 0, 0));
}
