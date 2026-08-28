use super::super::profile::{Profile, RenderingIntent, TransformOptions};
use super::super::{Transform, TransformDirection, TransformError, TransformLimits};
use crate::allocation_probe;

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn raw_profile(
    version: u8,
    class: &[u8; 4],
    space: &[u8; 4],
    tags: Vec<(&[u8; 4], Vec<u8>)>,
) -> Vec<u8> {
    let mut bytes = vec![0; 132 + 12 * tags.len()];
    bytes[8..12].copy_from_slice(&[version, 0, 0, 0]);
    bytes[12..16].copy_from_slice(class);
    bytes[16..20].copy_from_slice(space);
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    put_u32(&mut bytes, 64, 0);
    put_u32(&mut bytes, 128, tags.len() as u32);
    for (index, (signature, data)) in tags.into_iter().enumerate() {
        let offset = (bytes.len() + 3) & !3;
        bytes.resize(offset + data.len(), 0);
        bytes[offset..offset + data.len()].copy_from_slice(&data);
        let table = 132 + index * 12;
        bytes[table..table + 4].copy_from_slice(signature);
        put_u32(&mut bytes, table + 4, offset as u32);
        put_u32(&mut bytes, table + 8, data.len() as u32);
    }
    let length = bytes.len() as u32;
    put_u32(&mut bytes, 0, length);
    bytes
}

fn xyz_tag(values: [f32; 3]) -> Vec<u8> {
    let mut tag = vec![0; 20];
    tag[..4].copy_from_slice(b"XYZ ");
    for (index, value) in values.into_iter().enumerate() {
        put_u32(
            &mut tag,
            8 + index * 4,
            ((value * 65536.0).round() as i32) as u32,
        );
    }
    tag
}

fn identity_curve() -> Vec<u8> {
    let mut curve = vec![0; 12];
    curve[..4].copy_from_slice(b"curv");
    curve
}

fn nonzero_black_curve() -> Vec<u8> {
    let mut curve = vec![0; 16];
    curve[..4].copy_from_slice(b"curv");
    put_u32(&mut curve, 8, 2);
    curve[12..14].copy_from_slice(&1u16.to_be_bytes());
    curve[14..16].copy_from_slice(&u16::MAX.to_be_bytes());
    curve
}

fn identity_mab(signature: &[u8; 4]) -> Vec<u8> {
    let mut tag = vec![0; 68];
    tag[..4].copy_from_slice(signature);
    tag[8] = 3;
    tag[9] = 3;
    put_u32(&mut tag, 12, 32);
    for index in 0..3 {
        tag[32 + index * 12..36 + index * 12].copy_from_slice(b"curv");
    }
    tag
}

fn matrix_profile(version: u8, class: &[u8; 4], curve: Vec<u8>) -> Profile {
    let tags = vec![
        (b"wtpt", xyz_tag([0.9642, 1.0, 0.8249])),
        (b"rXYZ", xyz_tag([0.9642, 0.0, 0.0])),
        (b"gXYZ", xyz_tag([0.0, 1.0, 0.0])),
        (b"bXYZ", xyz_tag([0.0, 0.0, 0.8249])),
        (b"rTRC", curve.clone()),
        (b"gTRC", curve.clone()),
        (b"bTRC", curve),
    ];
    Profile::parse(&raw_profile(version, class, b"RGB ", tags)).unwrap()
}

fn lut_profile(version: u8, class: &[u8; 4], tag: &[u8; 4]) -> Profile {
    Profile::parse(&raw_profile(
        version,
        class,
        b"RGB ",
        vec![
            (b"wtpt", xyz_tag([0.9642, 1.0, 0.8249])),
            (tag, identity_mab(b"mBA ")),
        ],
    ))
    .unwrap()
}

fn destination_profile_with_tag(
    version: u8,
    class: &[u8; 4],
    tag: &[u8; 4],
    data: Vec<u8>,
) -> Profile {
    Profile::parse(&raw_profile(
        version,
        class,
        b"RGB ",
        vec![(b"wtpt", xyz_tag([0.9642, 1.0, 0.8249])), (tag, data)],
    ))
    .unwrap()
}

fn options(intent: RenderingIntent) -> TransformOptions {
    TransformOptions {
        rendering_intent: intent,
        black_point_compensation: false,
        clamp: true,
    }
}

fn expected_encoded(input: [f32; 3], apply_reference_black: bool) -> [f32; 3] {
    const BLACK: [f32; 3] = [0.003357, 0.003479, 0.002869];
    const D50: [f32; 3] = [0.9642, 1.0, 0.8249];
    let mut xyz = [input[0] * D50[0], input[1], input[2] * D50[2]];
    if apply_reference_black {
        for ((value, black), white) in xyz.iter_mut().zip(BLACK).zip(D50) {
            *value = *value * (1.0 - black / white) + black;
        }
    }
    xyz.map(|value| value * 32768.0 / 65535.0)
}

fn expected_physical(input: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|index| input[index] * [0.9642, 1.0, 0.8249][index])
}

#[test]
fn perceptual_matrix_to_b2a0_applies_reference_black_to_neutral_and_colour() {
    let source = matrix_profile(2, b"mntr", identity_curve());
    let destination = lut_profile(4, b"mntr", b"B2A0");
    let transform =
        Transform::new(&source, &destination, options(RenderingIntent::Perceptual)).unwrap();
    assert_eq!(transform.output_route_info().selected_tag(), Some(*b"B2A0"));
    assert!(!transform.output_route_info().used_fallback());

    let standalone = source
        .compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::Perceptual,
            TransformLimits::default(),
        )
        .unwrap();
    for input in [
        [0.0, 0.0, 0.0],
        [0.2, 0.4, 0.8],
        [0.25, 0.25, 0.25],
        [1.0, 1.0, 1.0],
    ] {
        let mut physical = [0.0; 3];
        standalone.transform_f32(&input, &mut physical).unwrap();
        for (actual, expected) in physical.into_iter().zip(expected_physical(input)) {
            assert!((actual - expected).abs() < 2.0e-5, "{actual} != {expected}");
        }

        let mut output = [0.0; 3];
        transform.transform_f32(&input, &mut output).unwrap();
        let expected = expected_encoded(input, true);
        for (actual, expected) in output.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 2.0e-5, "{actual} != {expected}");
        }
    }
}

#[test]
fn saturation_uses_b2a0_fallback_but_relative_does_not_apply_candidate() {
    let source = matrix_profile(2, b"mntr", identity_curve());
    let destination = lut_profile(4, b"mntr", b"B2A0");
    let saturation =
        Transform::new(&source, &destination, options(RenderingIntent::Saturation)).unwrap();
    assert!(saturation.output_route_info().used_fallback());
    let mut saturation_output = [0.0; 3];
    saturation
        .transform_f32(&[0.2, 0.4, 0.8], &mut saturation_output)
        .unwrap();
    let expected = expected_encoded([0.2, 0.4, 0.8], true);
    assert!(saturation_output
        .into_iter()
        .zip(expected)
        .all(|(actual, expected)| (actual - expected).abs() < 2.0e-5));

    let relative_destination = lut_profile(4, b"mntr", b"B2A1");
    let relative = Transform::new(
        &source,
        &relative_destination,
        options(RenderingIntent::RelativeColorimetric),
    )
    .unwrap();
    let mut relative_output = [0.0; 3];
    relative
        .transform_f32(&[0.2, 0.4, 0.8], &mut relative_output)
        .unwrap();
    let expected = expected_encoded([0.2, 0.4, 0.8], false);
    assert!(relative_output
        .into_iter()
        .zip(expected)
        .all(|(actual, expected)| (actual - expected).abs() < 2.0e-5));

    let relative_fallback_destination = lut_profile(4, b"mntr", b"B2A0");
    let relative_fallback = Transform::new(
        &source,
        &relative_fallback_destination,
        options(RenderingIntent::RelativeColorimetric),
    )
    .unwrap();
    assert!(relative_fallback.output_route_info().used_fallback());
    let mut relative_fallback_output = [0.0; 3];
    relative_fallback
        .transform_f32(&[0.2, 0.4, 0.8], &mut relative_fallback_output)
        .unwrap();
    assert!(relative_fallback_output
        .into_iter()
        .zip(expected)
        .all(|(actual, expected)| (actual - expected).abs() < 2.0e-5));

    let absolute = Transform::new(
        &source,
        &relative_destination,
        options(RenderingIntent::AbsoluteColorimetric),
    )
    .unwrap();
    assert_eq!(absolute.output_route_info().selected_tag(), Some(*b"B2A1"));
    let mut absolute_output = [0.0; 3];
    absolute
        .transform_f32(&[0.2, 0.4, 0.8], &mut absolute_output)
        .unwrap();
    assert!(absolute_output
        .into_iter()
        .zip(expected)
        .all(|(actual, expected)| (actual - expected).abs() < 2.0e-5));

    // The bridge is selected by the complete reverse route, profile versions,
    // and intent.  In particular, a matching matrix route or a v4->v4 route
    // must not acquire the v2->v4 B2A0 adjustment as a side effect.
    let input = [0.2, 0.4, 0.8];
    for source_version in [2, 4] {
        for destination_version in [2, 4] {
            for (intent, tag) in [
                (RenderingIntent::Perceptual, b"B2A0"),
                (RenderingIntent::Saturation, b"B2A2"),
                (RenderingIntent::RelativeColorimetric, b"B2A1"),
                (RenderingIntent::AbsoluteColorimetric, b"B2A1"),
            ] {
                let source = matrix_profile(source_version, b"mntr", identity_curve());
                let destination = lut_profile(destination_version, b"mntr", tag);
                let transform = Transform::new(&source, &destination, options(intent)).unwrap();
                let mut output = [0.0; 3];
                transform.transform_f32(&input, &mut output).unwrap();
                let bridge = source_version == 2
                    && destination_version == 4
                    && intent == RenderingIntent::Perceptual;
                let expected = expected_encoded(input, bridge);
                for (actual, expected) in output.into_iter().zip(expected) {
                    assert!((actual - expected).abs() < 2.0e-5);
                }
            }
        }
    }

    let matrix_destination = matrix_profile(4, b"mntr", identity_curve());
    let source_v2 = matrix_profile(2, b"mntr", identity_curve());
    for intent in [
        RenderingIntent::Perceptual,
        RenderingIntent::Saturation,
        RenderingIntent::RelativeColorimetric,
        RenderingIntent::AbsoluteColorimetric,
    ] {
        let transform = Transform::new(&source_v2, &matrix_destination, options(intent)).unwrap();
        let mut output = [0.0; 3];
        transform.transform_f32(&input, &mut output).unwrap();
        for (actual, expected) in output.into_iter().zip(input) {
            assert!((actual - expected).abs() < 2.0e-5);
        }
    }
}

#[test]
fn nonzero_black_and_unproved_classes_are_rejected_during_pair_planning() {
    let destination = lut_profile(4, b"mntr", b"B2A0");
    let nonzero = matrix_profile(2, b"mntr", nonzero_black_curve());
    let (result, _, bytes) = allocation_probe::watch(1, || {
        Transform::new(&nonzero, &destination, options(RenderingIntent::Perceptual))
    });
    assert!(matches!(
        result,
        Err(TransformError::UnsupportedProfileFeature(_))
    ));
    assert_eq!(
        bytes, 0,
        "nonzero-black rejection must precede materialization"
    );

    for class in [b"prtr", b"scnr", b"spac"] {
        let source = matrix_profile(2, class, identity_curve());
        let (result, _, bytes) = allocation_probe::watch(1, || {
            Transform::new(&source, &destination, options(RenderingIntent::Perceptual))
        });
        assert!(matches!(
            result,
            Err(TransformError::UnsupportedProfileFeature(_))
        ));
        assert_eq!(
            bytes, 0,
            "unproved source class must reject before materialization"
        );
    }

    let source = matrix_profile(2, b"mntr", identity_curve());
    for class in [b"prtr", b"scnr", b"spac"] {
        let destination = lut_profile(4, class, b"B2A0");
        let (result, _, bytes) = allocation_probe::watch(1, || {
            Transform::new(&source, &destination, options(RenderingIntent::Saturation))
        });
        assert!(matches!(
            result,
            Err(TransformError::UnsupportedProfileFeature(_))
        ));
        assert_eq!(
            bytes, 0,
            "unproved destination class must reject before materialization"
        );
    }
}

fn mft2_curved() -> Vec<u8> {
    let input_channels = 3usize;
    let output_channels = 3usize;
    let grid_points = 2usize;
    let value_count = input_channels * 2
        + grid_points.pow(input_channels as u32) * output_channels
        + output_channels * 2;
    let mut tag = vec![0; 52 + 2 * value_count];
    tag[..4].copy_from_slice(b"mft2");
    tag[8] = input_channels as u8;
    tag[9] = output_channels as u8;
    tag[10] = grid_points as u8;
    for index in 0..3 {
        put_u32(&mut tag, 12 + index * 16, 65536);
    }
    tag[48..50].copy_from_slice(&2u16.to_be_bytes());
    tag[50..52].copy_from_slice(&2u16.to_be_bytes());

    let mut values = vec![0u16, 65535, 0, 65535, 0, 65535];
    for x in 0..2u16 {
        for y in 0..2u16 {
            for z in 0..2u16 {
                values.extend([x * y * 65535, y * z * 65535, z * x * 65535]);
            }
        }
    }
    values.extend([0, 65535, 0, 65535, 0, 65535]);
    for (index, value) in values.into_iter().enumerate() {
        tag[52 + 2 * index..54 + 2 * index].copy_from_slice(&value.to_be_bytes());
    }
    tag
}

fn parametric_curve(kind: u16, values: &[f32]) -> Vec<u8> {
    let mut curve = vec![0; 12];
    curve[..4].copy_from_slice(b"para");
    curve[8..10].copy_from_slice(&kind.to_be_bytes());
    for value in values {
        curve.extend(((*value * 65536.0) as i32).to_be_bytes());
    }
    curve
}

#[test]
fn reverse_bridge_curved_lut_uses_tetrahedral_interpolation() {
    let source = matrix_profile(2, b"mntr", identity_curve());
    let destination = destination_profile_with_tag(4, b"mntr", b"B2A0", mft2_curved());
    for input in [[0.2, 0.4, 0.8], [0.8, 0.1, 0.3], [0.3, 0.8, 0.1]] {
        let physical = expected_encoded(input, true);
        let tetrahedral = [
            physical[0].min(physical[1]),
            physical[1].min(physical[2]),
            physical[2].min(physical[0]),
        ];
        let transform =
            Transform::new(&source, &destination, options(RenderingIntent::Perceptual)).unwrap();
        let mut output = [0.0; 3];
        transform.transform_f32(&input, &mut output).unwrap();
        for (actual, expected) in output.into_iter().zip(tetrahedral) {
            assert!((actual - expected).abs() < 2.0e-5, "{actual} != {expected}");
        }
        assert!(
            (tetrahedral[0] - physical[0] * physical[1]).abs() > 0.01,
            "fixture must distinguish interpolation methods"
        );
    }
}

#[test]
fn reverse_bridge_parametric_zero_endpoint_supports_curve_kinds_zero_through_four() {
    let destination = lut_profile(4, b"mntr", b"B2A0");
    let supported = [
        vec![1.0],
        vec![1.0, 1.0, 0.0],
        vec![1.0, 1.0, 0.0, 0.0],
        vec![1.0, 1.0, 0.0, 1.0, 0.5],
        vec![1.0, 1.0, 0.0, 1.0, 0.5, 0.0, 0.0],
    ];
    for (kind, values) in supported.into_iter().enumerate() {
        let source = matrix_profile(2, b"mntr", parametric_curve(kind as u16, &values));
        let transform =
            Transform::new(&source, &destination, options(RenderingIntent::Perceptual)).unwrap();
        let mut output = [0.0; 3];
        transform.transform_f32(&[0.0; 3], &mut output).unwrap();
        let expected = [0.003357, 0.003479, 0.002869].map(|value| value * 32768.0 / 65535.0);
        for (actual, expected) in output.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 2.0e-5);
        }
    }

    for (kind, values) in [
        (1u16, vec![1.0, 1.0, 0.25]),
        (2, vec![1.0, 1.0, 0.0, 0.25]),
        (4, vec![1.0, 1.0, 0.0, 1.0, 0.5, 0.0, 0.25]),
    ] {
        let source = matrix_profile(2, b"mntr", parametric_curve(kind, &values));
        assert!(matches!(
            Transform::new(&source, &destination, options(RenderingIntent::Perceptual)),
            Err(TransformError::UnsupportedProfileFeature(_))
        ));
    }
}
