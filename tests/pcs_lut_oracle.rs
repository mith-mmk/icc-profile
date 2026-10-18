//! Independent LittleCMS reference for the paired Lab-PCS LUT route.
//! See fixtures/README.md for generation, flags and tolerance rationale.

use icc_profile::{Profile, Transform, TransformOptions};

#[test]
fn paired_lab_luts_match_littlecms_reference_at_each_precision() {
    let profile = Profile::from_bytes(include_bytes!("fixtures/lab-pcs-rgb.icc")).unwrap();
    let transform = Transform::new(&profile, &profile, TransformOptions::default()).unwrap();
    for line in include_str!("fixtures/lab-pcs-rgb.csv").lines().skip(1) {
        let values: Vec<u16> = line.split(',').map(|v| v.parse().unwrap()).collect();
        let mut output16 = [0; 3];
        transform
            .transform_u16(&values[..3], &mut output16)
            .unwrap();
        let input32: Vec<f32> = values[..3].iter().map(|&v| v as f32 / 65535.0).collect();
        let mut output32 = [0.0; 3];
        transform.transform_f32(&input32, &mut output32).unwrap();
        let input8: Vec<u8> = values[..3]
            .iter()
            .map(|&v| (v as f32 / 257.0).round() as u8)
            .collect();
        let mut output8 = [0; 3];
        transform.transform_u8(&input8, &mut output8).unwrap();
        for channel in 0..3 {
            let reference = values[channel + 3];
            // Finite-grid interpolation near gamut boundaries differs from
            // LittleCMS. The analytic PCS test separately uses 1e-4 bounds.
            assert!(
                output16[channel].abs_diff(reference) <= 1024,
                "u16 input={:?}: {output16:?}, expected {:?}",
                &values[..3],
                &values[3..]
            );
            assert!(
                (output32[channel] - reference as f32 / 65535.0).abs() <= 1024.0 / 65535.0,
                "f32 input={input32:?}: {output32:?}, reference={reference}"
            );
            assert!(
                (output8[channel] as f32 - reference as f32 / 257.0).abs() <= 5.0,
                "u8 input={input8:?}: {output8:?}, reference={reference}"
            );
        }
    }
}
