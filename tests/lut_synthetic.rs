//! Small, tracked LUT fixtures.  These deliberately exercise the binary
//! layout independently of the legacy CMYK fixtures.

use icc_profile::{
    ColorSpace, Profile, RenderingIntent, Transform, TransformDirection, TransformError,
    TransformLimits, TransformOptions,
};

fn u32_at(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
}

fn profile(color_space: [u8; 4], tags: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
    profile_with_pcs(color_space, *b"XYZ ", tags)
}

fn profile_with_pcs(color_space: [u8; 4], pcs: [u8; 4], tags: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
    let mut result = vec![0; 132 + tags.len() * 12];
    result[8..12].copy_from_slice(&[4, 0, 0, 0]);
    result[12..16].copy_from_slice(b"mntr");
    result[16..20].copy_from_slice(&color_space);
    result[20..24].copy_from_slice(&pcs);
    result[36..40].copy_from_slice(b"acsp");
    u32_at(&mut result, 64, 1);
    u32_at(&mut result, 128, tags.len() as u32);
    let mut offset = result.len();
    for (index, (signature, payload)) in tags.into_iter().enumerate() {
        offset = (offset + 3) & !3;
        result.resize(offset + payload.len(), 0);
        result[offset..offset + payload.len()].copy_from_slice(&payload);
        let entry = 132 + index * 12;
        u32_at(&mut result, entry, u32::from_be_bytes(signature));
        u32_at(&mut result, entry + 4, offset as u32);
        u32_at(&mut result, entry + 8, payload.len() as u32);
        offset += payload.len();
    }
    let length = result.len() as u32;
    u32_at(&mut result, 0, length);
    result
}

#[test]
fn compiled_lut_limits_are_checked_before_materialization() {
    let bytes = profile(*b"RGB ", vec![(*b"A2B0", identity_mft2())]);
    let profile = Profile::parse(&bytes).unwrap();
    let limits = TransformLimits::builder()
        .max_compiled_bytes(64)
        .max_curve_entries(4096)
        .max_clut_entries(4096)
        .build()
        .unwrap();
    assert!(matches!(
        profile.compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::Perceptual,
            limits
        ),
        Err(TransformError::ResourceLimit(_))
    ));
}

fn identity_mft2() -> Vec<u8> {
    let mut tag = vec![0; 52 + 3 * 2 * 2 + 8 * 3 * 2 + 3 * 2 * 2];
    tag[0..4].copy_from_slice(b"mft2");
    tag[8] = 3;
    tag[9] = 3;
    tag[10] = 2;
    for i in 0..3 {
        u32_at(&mut tag, 12 + i * 16, 0x0001_0000);
    }
    tag[48..50].copy_from_slice(&2u16.to_be_bytes());
    tag[50..52].copy_from_slice(&2u16.to_be_bytes());
    let mut at = 52;
    for _ in 0..3 {
        tag[at..at + 2].copy_from_slice(&0u16.to_be_bytes());
        tag[at + 2..at + 4].copy_from_slice(&65535u16.to_be_bytes());
        at += 4;
    }
    // CLUT order is first axis slowest: x, then y, then z.
    for x in 0..2u8 {
        for y in 0..2u8 {
            for z in 0..2u8 {
                for value in [x, y, z] {
                    let value: u16 = if value == 0 { 0 } else { 65535 };
                    tag[at..at + 2].copy_from_slice(&value.to_be_bytes());
                    at += 2;
                }
            }
        }
    }
    for _ in 0..3 {
        tag[at..at + 2].copy_from_slice(&0u16.to_be_bytes());
        tag[at + 2..at + 4].copy_from_slice(&65535u16.to_be_bytes());
        at += 4;
    }
    tag
}

fn padded_gamma_curve() -> Vec<u8> {
    let mut curve = vec![0; 16];
    curve[0..4].copy_from_slice(b"curv");
    curve[8..12].copy_from_slice(&1u32.to_be_bytes());
    curve[12..14].copy_from_slice(&256u16.to_be_bytes());
    curve
}

fn gray_mba() -> Vec<u8> {
    let mut tag = vec![0; 124];
    tag[0..4].copy_from_slice(b"mBA ");
    tag[8] = 3;
    tag[9] = 1;
    u32_at(&mut tag, 12, 32); // B curves (three identity curves)
    u32_at(&mut tag, 24, 80); // CLUT
    u32_at(&mut tag, 28, 108); // A curve
    for at in [32, 48, 64, 108] {
        tag[at..at + 16].copy_from_slice(&padded_gamma_curve());
    }
    tag[80] = 2;
    tag[81] = 2;
    tag[82] = 2;
    tag[96] = 1; // 8-bit CLUT samples
                 // Select the first (slowest) axis.  The other seven entries are zero.
    tag[104] = 255;
    tag
}

fn gray_full_mba() -> Vec<u8> {
    let mut tag = vec![0; 220];
    tag[0..4].copy_from_slice(b"mBA ");
    tag[8] = 3;
    tag[9] = 1;
    u32_at(&mut tag, 12, 32); // B curves
    u32_at(&mut tag, 16, 80); // matrix
    u32_at(&mut tag, 20, 128); // M curves
    u32_at(&mut tag, 24, 192); // CLUT
    u32_at(&mut tag, 28, 176); // A curve
    for at in [32, 48, 64, 128, 144, 160, 176] {
        tag[at..at + 16].copy_from_slice(&padded_gamma_curve());
    }
    for i in 0..3 {
        u32_at(&mut tag, 80 + i * 16, 0x0001_0000);
    }
    tag[192..195].copy_from_slice(&[2, 2, 2]);
    tag[208] = 1; // 8-bit CLUT samples
                  // A single output channel, with a non-symmetric corner to catch the
                  // historical use of the matrix channel count here.
    tag[212..220].copy_from_slice(&[255; 8]);
    tag
}

fn rgb_mft_profile() -> Vec<u8> {
    profile(
        *b"RGB ",
        vec![(*b"A2B0", identity_mft2()), (*b"B2A0", identity_mft2())],
    )
}

fn xyz(x: f32, y: f32, z: f32) -> Vec<u8> {
    let mut tag = vec![0; 20];
    tag[0..4].copy_from_slice(b"XYZ ");
    for (at, value) in [(8, x), (12, y), (16, z)] {
        tag[at..at + 4].copy_from_slice(&((value * 65536.0).round() as i32).to_be_bytes());
    }
    tag
}

fn gamma_with_value(value: u16) -> Vec<u8> {
    let mut tag = vec![0; 16];
    tag[0..4].copy_from_slice(b"curv");
    tag[8..12].copy_from_slice(&1u32.to_be_bytes());
    tag[12..14].copy_from_slice(&value.to_be_bytes());
    tag
}

fn identity_curve() -> Vec<u8> {
    let mut curve = vec![0; 12];
    curve[0..4].copy_from_slice(b"curv");
    curve
}

fn matrix_profile_with(gamma_value: u16, columns: [[f32; 3]; 3]) -> Vec<u8> {
    profile(
        *b"RGB ",
        vec![
            (*b"rXYZ", xyz(columns[0][0], columns[1][0], columns[2][0])),
            (*b"gXYZ", xyz(columns[0][1], columns[1][1], columns[2][1])),
            (*b"bXYZ", xyz(columns[0][2], columns[1][2], columns[2][2])),
            (*b"rTRC", gamma_with_value(gamma_value)),
            (*b"gTRC", gamma_with_value(gamma_value)),
            (*b"bTRC", gamma_with_value(gamma_value)),
            (*b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        ],
    )
}

fn sampled_curve(values: &[u16]) -> Vec<u8> {
    let mut curve = vec![0; 12 + values.len() * 2];
    curve[0..4].copy_from_slice(b"curv");
    curve[8..12].copy_from_slice(&(values.len() as u32).to_be_bytes());
    for (index, value) in values.iter().enumerate() {
        curve[12 + index * 2..14 + index * 2].copy_from_slice(&value.to_be_bytes());
    }
    curve
}

fn forward_only_non_monotonic_matrix_profile() -> Vec<u8> {
    profile(
        *b"RGB ",
        vec![
            (*b"rXYZ", xyz(0.9642, 0.0, 0.0)),
            (*b"gXYZ", xyz(0.0, 1.0, 0.0)),
            (*b"bXYZ", xyz(0.0, 0.0, 0.8249)),
            (*b"rTRC", sampled_curve(&[0, 65535, 0])),
            (*b"gTRC", sampled_curve(&[0, 65535, 0])),
            (*b"bTRC", sampled_curve(&[0, 65535, 0])),
            (*b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        ],
    )
}

fn matrix_rgb_profile() -> Vec<u8> {
    matrix_profile_with(
        256,
        [
            [0.4124, 0.3576, 0.1805],
            [0.2126, 0.7152, 0.0722],
            [0.0193, 0.1192, 0.9505],
        ],
    )
}

fn d50_matrix_rgb_profile() -> Vec<u8> {
    matrix_profile_with(
        256,
        [[0.9642, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.8249]],
    )
}

fn matrix_rgb_profile_with_white(white: [f32; 3]) -> Vec<u8> {
    profile(
        *b"RGB ",
        vec![
            (*b"rXYZ", xyz(0.9642, 0.0, 0.0)),
            (*b"gXYZ", xyz(0.0, 1.0, 0.0)),
            (*b"bXYZ", xyz(0.0, 0.0, 0.8249)),
            (*b"rTRC", gamma_with_value(256)),
            (*b"gTRC", gamma_with_value(256)),
            (*b"bTRC", gamma_with_value(256)),
            (*b"wtpt", xyz(white[0], white[1], white[2])),
        ],
    )
}

fn gray_matrix_profile(pcs: [u8; 4]) -> Vec<u8> {
    profile_with_pcs(*b"GRAY", pcs, vec![(*b"kTRC", identity_curve())])
}

fn mft2_d50_white() -> Vec<u8> {
    // ICC legacy XYZ uses 1.15 fixed-point encoding: round the D50
    // reference values before constructing the synthetic tag.
    mft2_constant([31595, 32768, 27030])
}

fn mft2_xyz_vector() -> Vec<u8> {
    mft2_constant([10000, 18000, 24000])
}

fn mft2_lab_white() -> Vec<u8> {
    mft2_constant([65535, 32768, 32768])
}

fn mft2_lab_non_neutral() -> Vec<u8> {
    mft2_constant([32640, 43008, 25088])
}

fn mft2_constant(values: [u16; 3]) -> Vec<u8> {
    let mut tag = vec![0; 124];
    tag[0..4].copy_from_slice(b"mft2");
    tag[8] = 3;
    tag[9] = 3;
    tag[10] = 2;
    for i in 0..3 {
        u32_at(&mut tag, 12 + i * 16, 0x0001_0000);
    }
    tag[48..50].copy_from_slice(&2u16.to_be_bytes());
    tag[50..52].copy_from_slice(&2u16.to_be_bytes());
    let mut at = 52;
    for _ in 0..3 {
        tag[at..at + 2].copy_from_slice(&0u16.to_be_bytes());
        tag[at + 2..at + 4].copy_from_slice(&65535u16.to_be_bytes());
        at += 4;
    }
    for _ in 0..8 {
        for value in values {
            tag[at..at + 2].copy_from_slice(&value.to_be_bytes());
            at += 2;
        }
    }
    for _ in 0..3 {
        tag[at..at + 2].copy_from_slice(&0u16.to_be_bytes());
        tag[at + 2..at + 4].copy_from_slice(&65535u16.to_be_bytes());
        at += 4;
    }
    tag
}

fn mab_lab_white() -> Vec<u8> {
    let mut tag = vec![0; 172];
    tag[0..4].copy_from_slice(b"mAB ");
    tag[8] = 3;
    tag[9] = 3;
    u32_at(&mut tag, 12, 32); // B curves are required; A/M/matrix are optional.
    u32_at(&mut tag, 24, 68); // CLUT
    u32_at(&mut tag, 28, 136); // A curves are paired with the CLUT.
    for at in [32, 44, 56] {
        tag[at..at + 12].copy_from_slice(&identity_curve());
    }
    tag[68] = 2;
    tag[69] = 2;
    tag[70] = 2;
    tag[84] = 2; // 16-bit CLUT samples
    let mut at = 88;
    for _ in 0..8 {
        for value in [65535u16, 32896, 32896] {
            tag[at..at + 2].copy_from_slice(&value.to_be_bytes());
            at += 2;
        }
    }
    for at in [136, 148, 160] {
        tag[at..at + 12].copy_from_slice(&identity_curve());
    }
    tag
}

fn mab_b_only() -> Vec<u8> {
    let mut tag = vec![0; 68];
    tag[0..4].copy_from_slice(b"mAB ");
    tag[8] = 3;
    tag[9] = 3;
    u32_at(&mut tag, 12, 32);
    for at in [32, 44, 56] {
        tag[at..at + 12].copy_from_slice(&identity_curve());
    }
    tag
}

fn mab_b_matrix_m() -> Vec<u8> {
    mab_b_matrix_m_with_scale(1.0)
}

fn mab_b_matrix_m_with_scale(scale: f32) -> Vec<u8> {
    let mut tag = vec![0; 152];
    tag[0..4].copy_from_slice(b"mAB ");
    tag[8] = 3;
    tag[9] = 3;
    u32_at(&mut tag, 12, 32); // B
    u32_at(&mut tag, 16, 68); // matrix
    u32_at(&mut tag, 20, 116); // M
    for at in [32, 44, 56, 116, 128, 140] {
        tag[at..at + 12].copy_from_slice(&identity_curve());
    }
    for i in 0..3 {
        u32_at(&mut tag, 68 + i * 16, (scale * 65536.0).round() as u32);
    }
    tag
}

fn mba_b_matrix_m_with_scale(scale: f32) -> Vec<u8> {
    let mut tag = mab_b_matrix_m_with_scale(scale);
    tag[0..4].copy_from_slice(b"mBA ");
    tag
}

fn mab_b2a_identity() -> Vec<u8> {
    let mut tag = vec![0; 172];
    tag[0..4].copy_from_slice(b"mBA ");
    tag[8] = 3;
    tag[9] = 3;
    u32_at(&mut tag, 12, 32);
    u32_at(&mut tag, 24, 68);
    u32_at(&mut tag, 28, 136); // A curves are paired with the CLUT.
    for at in [32, 44, 56] {
        tag[at..at + 12].copy_from_slice(&identity_curve());
    }
    tag[68] = 2;
    tag[69] = 2;
    tag[70] = 2;
    tag[84] = 2;
    let mut at = 88;
    for x in 0..2u16 {
        for y in 0..2u16 {
            for z in 0..2u16 {
                for value in [x * 65535, y * 65535, z * 65535] {
                    tag[at..at + 2].copy_from_slice(&value.to_be_bytes());
                    at += 2;
                }
            }
        }
    }
    for at in [136, 148, 160] {
        tag[at..at + 12].copy_from_slice(&identity_curve());
    }
    tag
}

fn append_aligned(tag: &mut Vec<u8>, payload: &[u8]) -> u32 {
    let offset = (tag.len() + 3) & !3;
    tag.resize(offset, 0);
    tag.extend_from_slice(payload);
    offset as u32
}

fn curve_set(count: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(count * 16);
    for _ in 0..count {
        result.extend_from_slice(&padded_gamma_curve());
    }
    result
}

fn matrix_stage() -> Vec<u8> {
    let mut result = vec![0; 48];
    for i in 0..3 {
        u32_at(&mut result, i * 16, 0x0001_0000);
    }
    result
}

fn clut_stage(input_channels: usize, output_channels: usize) -> Vec<u8> {
    let points = 1usize << input_channels;
    let mut result = vec![0; 20 + points * output_channels * 2];
    for i in 0..input_channels {
        result[i] = 2;
    }
    result[16] = 2;
    let mut at = 20;
    for point in 0..points {
        for channel in 0..output_channels {
            let value = if point & (1 << channel.min(input_channels - 1)) != 0 {
                u16::MAX
            } else {
                0
            };
            result[at..at + 2].copy_from_slice(&value.to_be_bytes());
            at += 2;
        }
    }
    result
}

fn stage_fixture(
    reverse: bool,
    input_channels: usize,
    output_channels: usize,
    has_a: bool,
    has_clut: bool,
    has_m: bool,
    has_matrix: bool,
) -> Vec<u8> {
    let mut tag = vec![0; 32];
    tag[0..4].copy_from_slice(if reverse { b"mBA " } else { b"mAB " });
    tag[8] = input_channels as u8;
    tag[9] = output_channels as u8;
    let a_count = if reverse {
        output_channels
    } else {
        input_channels
    };
    let b_count = if reverse {
        input_channels
    } else {
        output_channels
    };
    let b_offset = append_aligned(&mut tag, &curve_set(b_count));
    u32_at(&mut tag, 12, b_offset);
    if has_matrix {
        let matrix_offset = append_aligned(&mut tag, &matrix_stage());
        u32_at(&mut tag, 16, matrix_offset);
    }
    if has_m {
        let m_offset = append_aligned(&mut tag, &curve_set(3));
        u32_at(&mut tag, 20, m_offset);
    }
    if has_clut {
        let clut_offset = append_aligned(&mut tag, &clut_stage(input_channels, output_channels));
        u32_at(&mut tag, 24, clut_offset);
    }
    if has_a {
        let a_offset = append_aligned(&mut tag, &curve_set(a_count));
        u32_at(&mut tag, 28, a_offset);
    }
    tag
}

fn rgb_to_gray_profile() -> (Vec<u8>, Vec<u8>) {
    (
        rgb_mft_profile(),
        profile(*b"GRAY", vec![(*b"B2A0", gray_mba())]),
    )
}

#[test]
fn structural_parse_defers_inverse_only_validation() {
    let bytes = forward_only_non_monotonic_matrix_profile();
    assert!(Profile::new(&bytes).is_err());
    let profile = Profile::parse(&bytes).unwrap();
    let compiled = profile
        .compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::RelativeColorimetric,
            TransformLimits::default(),
        )
        .unwrap();
    let mut pcs = [0.0; 3];
    compiled
        .transform_f32(&[0.25, 0.5, 0.75], &mut pcs)
        .unwrap();
    assert!(pcs.iter().all(|value| value.is_finite()));
    assert!(profile
        .compile(
            TransformDirection::PcsToDevice,
            RenderingIntent::RelativeColorimetric,
            TransformLimits::default(),
        )
        .is_err());
}

#[test]
fn mft_clut_uses_first_axis_slowest_and_mba_supports_three_to_one() {
    let rgb = Profile::new(&rgb_mft_profile()).unwrap();
    assert_eq!(rgb.color_space(), ColorSpace::Rgb);
    let matrix = Profile::new(&matrix_rgb_profile()).unwrap();
    // Exercise the A2B LUT only. Using the same LUT in both directions could
    // hide an axis-order error by applying the permutation twice.
    let identity = Transform::new(&rgb, &matrix, TransformOptions::default()).unwrap();
    let mut out = [0.0; 3];
    identity.transform_f32(&[1.0, 0.0, 0.0], &mut out).unwrap();
    assert!(
        (out[0] - 1.0).abs() < 1e-6,
        "first axis was not slowest: {out:?}"
    );

    let (rgb_bytes, gray_bytes) = rgb_to_gray_profile();
    let rgb = Profile::new(&rgb_bytes).unwrap();
    let gray = Profile::new(&gray_bytes).unwrap();
    let transform = Transform::new(&rgb, &gray, TransformOptions::default()).unwrap();
    let mut gray_out = [0.0];
    transform
        .transform_f32(&[1.0, 0.0, 0.0], &mut gray_out)
        .unwrap();
    assert!(
        (gray_out[0] - 1.0).abs() < 1e-6,
        "mBA 3-to-1 failed: {gray_out:?}"
    );
}

#[test]
fn malformed_lut_channels_stages_and_nonfinite_input_are_rejected() {
    let mut bad_channels = identity_mft2();
    bad_channels[8] = 2;
    let bad = Profile::new(&profile(*b"RGB ", vec![(*b"A2B0", bad_channels)])).unwrap();
    let err = Transform::new(&bad, &bad, TransformOptions::default()).unwrap_err();
    assert!(matches!(err, TransformError::UnsupportedProfileFeature(_)));

    let mut no_stages = vec![0; 32];
    no_stages[0..4].copy_from_slice(b"mAB ");
    no_stages[8] = 3;
    no_stages[9] = 3;
    let empty = Profile::new(&profile(*b"RGB ", vec![(*b"A2B0", no_stages)])).unwrap();
    let err = Transform::new(&empty, &empty, TransformOptions::default()).unwrap_err();
    assert!(matches!(err, TransformError::InvalidProfile(_)));

    let rgb = Profile::new(&rgb_mft_profile()).unwrap();
    let identity = Transform::new(&rgb, &rgb, TransformOptions::default()).unwrap();
    let mut out = [0.0; 3];
    assert!(matches!(
        identity.transform_f32(&[f32::NAN, 0.0, 0.0], &mut out),
        Err(TransformError::NonFiniteInput)
    ));
}

#[test]
fn absolute_direction_zero_fallback_still_requires_media_white() {
    let rgb = Profile::new(&rgb_mft_profile()).unwrap();
    let mut options = TransformOptions::default();
    options.rendering_intent = RenderingIntent::AbsoluteColorimetric;
    assert!(matches!(
        Transform::new(&rgb, &rgb, options),
        Err(TransformError::UnsupportedProfileFeature(_))
    ));
}

#[test]
fn intent_selection_uses_requested_suffix_and_direction_zero_fallback() {
    let source = Profile::new(&profile(
        *b"RGB ",
        vec![
            (*b"A2B0", mft2_constant([4096, 4096, 4096])),
            (*b"A2B1", mft2_constant([16384, 16384, 16384])),
            (*b"A2B2", mft2_constant([20000, 20000, 20000])),
            (*b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        ],
    ))
    .unwrap();
    let destination = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    for (intent, expected) in [
        (RenderingIntent::Perceptual, 4096.0 / 32768.0),
        (RenderingIntent::RelativeColorimetric, 16384.0 / 32768.0),
        (RenderingIntent::Saturation, 20000.0 / 32768.0),
    ] {
        let transform = Transform::new(
            &source,
            &destination,
            TransformOptions {
                rendering_intent: intent,
                clamp: false,
                ..TransformOptions::default()
            },
        )
        .unwrap();
        let mut output = [0.0; 3];
        transform
            .transform_f32(&[0.2, 0.4, 0.8], &mut output)
            .unwrap();
        let expected_output = [expected / 0.9642, expected, expected / 0.8249];
        assert!(
            output
                .into_iter()
                .zip(expected_output)
                .all(|(value, expected)| (value - expected).abs() < 0.002),
            "intent {intent:?}: expected {expected}, got {output:?}"
        );
    }

    let malformed_relative = Profile::new(&profile(
        *b"RGB ",
        vec![
            (*b"A2B0", identity_mft2()),
            (*b"A2B1", vec![0; 7]),
            (*b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        ],
    ))
    .unwrap();
    assert!(matches!(
        Transform::new(
            &malformed_relative,
            &destination,
            TransformOptions::default(),
        ),
        Err(TransformError::InvalidProfile(_))
    ));

    let saturation_fallback = Profile::new(&profile(
        *b"RGB ",
        vec![
            (*b"A2B0", identity_mft2()),
            (*b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        ],
    ))
    .unwrap();
    assert!(Transform::new(
        &saturation_fallback,
        &destination,
        TransformOptions {
            rendering_intent: RenderingIntent::Saturation,
            ..TransformOptions::default()
        },
    )
    .is_ok());

    let absolute_fallback = Profile::new(&profile(
        *b"RGB ",
        vec![
            (*b"A2B0", identity_mft2()),
            (*b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        ],
    ))
    .unwrap();
    assert!(Transform::new(
        &absolute_fallback,
        &destination,
        TransformOptions {
            rendering_intent: RenderingIntent::AbsoluteColorimetric,
            ..TransformOptions::default()
        },
    )
    .is_ok());
}

#[test]
fn absolute_lut_to_matrix_uses_one_media_white_bridge() {
    let source_white = [0.7714, 0.8, 0.6599];
    let source = Profile::new(&profile(
        *b"RGB ",
        vec![
            (*b"A2B1", mft2_d50_white()),
            (
                *b"wtpt",
                xyz(source_white[0], source_white[1], source_white[2]),
            ),
        ],
    ))
    .unwrap();
    let destination = Profile::new(&matrix_rgb_profile_with_white([0.9642, 1.0, 0.8249])).unwrap();
    let transform = Transform::new(
        &source,
        &destination,
        TransformOptions {
            rendering_intent: RenderingIntent::AbsoluteColorimetric,
            clamp: false,
            ..TransformOptions::default()
        },
    )
    .unwrap();
    let mut output = [0.0; 3];
    transform
        .transform_f32(&[0.2, 0.4, 0.8], &mut output)
        .unwrap();
    for (actual, expected) in output.into_iter().zip([0.8, 0.8, 0.8]) {
        assert!(
            (actual - expected).abs() < 0.003,
            "absolute bridge: {output:?}"
        );
    }
}

fn matrix_absolute_profile(white: [f32; 3]) -> Vec<u8> {
    matrix_rgb_profile_with_white(white)
}

fn lut_absolute_profile(white: [f32; 3], a_to_b: bool) -> Vec<u8> {
    profile(
        *b"RGB ",
        vec![
            (
                if a_to_b { *b"A2B1" } else { *b"B2A1" },
                if a_to_b {
                    mft2_d50_white()
                } else {
                    identity_mft2()
                },
            ),
            (*b"wtpt", xyz(white[0], white[1], white[2])),
        ],
    )
}

#[test]
fn absolute_bridge_covers_all_matrix_and_lut_pairings() {
    let source_white = [0.7714, 0.8, 0.6599];
    let destination_white = [0.9642, 1.0, 0.8249];
    let source_matrix = Profile::new(&matrix_absolute_profile(source_white)).unwrap();
    let source_lut = Profile::new(&lut_absolute_profile(source_white, true)).unwrap();
    let destination_matrix = Profile::new(&matrix_absolute_profile(destination_white)).unwrap();
    let destination_lut = Profile::new(&lut_absolute_profile(destination_white, false)).unwrap();

    for (source, matrix_input) in [(&source_matrix, true), (&source_lut, false)] {
        for (destination, matrix_output) in [(&destination_matrix, true), (&destination_lut, false)]
        {
            let transform = Transform::new(
                source,
                destination,
                TransformOptions {
                    rendering_intent: RenderingIntent::AbsoluteColorimetric,
                    clamp: false,
                    ..TransformOptions::default()
                },
            )
            .unwrap();
            let mut output = [0.0; 3];
            transform
                .transform_f32(
                    if matrix_input {
                        &[1.0, 1.0, 1.0]
                    } else {
                        &[0.2, 0.4, 0.8]
                    },
                    &mut output,
                )
                .unwrap();
            let expected = if matrix_output {
                [0.8, 0.8, 0.8]
            } else {
                let encoded = 32768.0 / 65535.0;
                [
                    0.8 * 0.9642 * encoded,
                    0.8 * encoded,
                    0.8 * 0.8249 * encoded,
                ]
            };
            for (actual, expected) in output.into_iter().zip(expected) {
                assert!(
                    (actual - expected).abs() < 0.003,
                    "pairing output: {output:?}"
                );
            }
        }
    }
}

#[test]
fn gray_xyz_and_lab_matrix_routes_have_distinct_analytic_pcs() {
    let xyz_profile = Profile::new(&gray_matrix_profile(*b"XYZ ")).unwrap();
    let lab_profile = Profile::new(&gray_matrix_profile(*b"Lab ")).unwrap();
    for (profile, lab) in [(&xyz_profile, false), (&lab_profile, true)] {
        let forward = profile
            .compile(
                TransformDirection::DeviceToPcs,
                RenderingIntent::RelativeColorimetric,
                TransformLimits::default(),
            )
            .unwrap();
        let reverse = profile
            .compile(
                TransformDirection::PcsToDevice,
                RenderingIntent::RelativeColorimetric,
                TransformLimits::default(),
            )
            .unwrap();
        for gray in [0.0_f32, 0.08, 0.5, 1.0] {
            let l = gray * 100.0;
            let luminance = if !lab {
                gray
            } else if l > 8.0 {
                ((l + 16.0) / 116.0).powi(3)
            } else {
                l * 27.0 / 24389.0
            };
            let expected = [0.9642 * luminance, luminance, 0.8249 * luminance];
            let mut pcs = [0.0; 3];
            forward.transform_f32(&[gray], &mut pcs).unwrap();
            for (actual, expected) in pcs.into_iter().zip(expected) {
                assert!((actual - expected).abs() < 1e-5, "gray={gray}: {pcs:?}");
            }
            let mut result = [0.0];
            reverse.transform_f32(&pcs, &mut result).unwrap();
            assert!((result[0] - gray).abs() < 1e-5, "gray={gray}: {result:?}");
        }
    }
}

#[test]
fn lut_to_gray_lab_uses_physical_xyz_for_both_source_pcs() {
    let xyz_source = Profile::new(&profile_with_pcs(
        *b"RGB ",
        *b"XYZ ",
        vec![(*b"A2B1", mft2_d50_white())],
    ))
    .unwrap();
    let lab_source = Profile::new(&profile_with_pcs(
        *b"RGB ",
        *b"Lab ",
        vec![(*b"A2B1", mft2_lab_white())],
    ))
    .unwrap();
    let destination = Profile::new(&gray_matrix_profile(*b"Lab ")).unwrap();
    for source in [&xyz_source, &lab_source] {
        let transform = Transform::new(source, &destination, TransformOptions::default()).unwrap();
        let mut output = [0.0];
        transform
            .transform_f32(&[0.2, 0.4, 0.8], &mut output)
            .unwrap();
        assert!(
            (output[0] - 1.0).abs() < 0.002,
            "Gray Lab output: {output:?}"
        );
    }
}

#[test]
fn rgb_to_gray_lab_uses_luminance_even_for_chromatic_xyz() {
    let source = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let destination = Profile::new(&gray_matrix_profile(*b"Lab ")).unwrap();
    let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
    let mut output = [0.0];
    transform
        .transform_f32(&[0.125, 0.25, 0.375], &mut output)
        .unwrap();
    let expected = (116.0 * 0.25_f32.cbrt() - 16.0) / 100.0;
    assert!(
        (output[0] - expected).abs() < 1e-5,
        "Gray Lab output: {output:?}"
    );
}

#[test]
fn rgb_lab_matrix_route_remains_explicitly_unsupported() {
    let mut bytes = matrix_rgb_profile_with_white([0.9642, 1.0, 0.8249]);
    bytes[20..24].copy_from_slice(b"Lab ");
    let profile = Profile::parse(&bytes).unwrap();
    assert!(matches!(
        profile.compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::RelativeColorimetric,
            TransformLimits::default(),
        ),
        Err(TransformError::UnsupportedProfileFeature(_))
    ));
}

#[test]
fn absolute_lut_tags_without_media_white_are_rejected() {
    let profile = Profile::new(&profile(
        *b"RGB ",
        vec![(*b"A2B3", identity_mft2()), (*b"B2A3", identity_mft2())],
    ))
    .unwrap();
    let options = TransformOptions {
        rendering_intent: RenderingIntent::AbsoluteColorimetric,
        ..TransformOptions::default()
    };
    assert!(matches!(
        Transform::new(&profile, &profile, options),
        Err(TransformError::UnsupportedProfileFeature(_))
    ));
}

#[test]
#[ignore = "requires the official sRGB v4 profile; set ICC_V4_MAB_TEST_PROFILE"]
fn official_v4_mab_fixture_compiles_without_nonfinite_output() {
    let path = std::env::var_os("ICC_V4_MAB_TEST_PROFILE")
        .map(std::path::PathBuf::from)
        .expect("ICC_V4_MAB_TEST_PROFILE must name the official sRGB v4 ICC fixture");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "official sRGB v4 ICC fixture is required at {} (or ICC_V4_MAB_TEST_PROFILE): {error}",
            path.display()
        )
    });
    let profile = Profile::new(&bytes).unwrap();
    for intent in [
        RenderingIntent::Perceptual,
        RenderingIntent::RelativeColorimetric,
    ] {
        let options = TransformOptions {
            rendering_intent: intent,
            ..TransformOptions::default()
        };
        let transform = Transform::new(&profile, &profile, options).unwrap();
        let mut output = [0.0; 3];
        transform
            .transform_f32(&[0.2, 0.4, 0.8], &mut output)
            .unwrap();
        assert!(output.iter().all(|value| value.is_finite()));
    }
}

#[test]
fn mft2_xyz_uses_legacy_32768_pcs_encoding() {
    let source = Profile::new(&profile(*b"RGB ", vec![(*b"A2B0", mft2_d50_white())])).unwrap();
    let destination = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let options = TransformOptions {
        clamp: false,
        ..TransformOptions::default()
    };
    let transform = Transform::new(&source, &destination, options).unwrap();
    let mut output = [0.0; 3];
    transform
        .transform_f32(&[0.2, 0.4, 0.8], &mut output)
        .unwrap();
    for value in output {
        assert!(
            (value - 1.0).abs() < 0.002,
            "bad legacy XYZ scale: {output:?}"
        );
    }
}

#[test]
fn mft2_xyz_one_way_preserves_non_saturated_channels() {
    let source = Profile::new(&profile(*b"RGB ", vec![(*b"A2B0", mft2_xyz_vector())])).unwrap();
    let destination = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
    let mut output = [0.0; 3];
    transform
        .transform_f32(&[0.2, 0.4, 0.8], &mut output)
        .unwrap();
    let expected = [
        (10000.0 / 32768.0) / 0.9642,
        18000.0 / 32768.0,
        (24000.0 / 32768.0) / 0.8249,
    ];
    for (actual, expected) in output.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.002,
            "PCS channel changed: {output:?}"
        );
    }
}

#[test]
fn legacy_mft2_and_modern_mab_lab_encode_d50_white() {
    let destination = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let old = Profile::new(&profile_with_pcs(
        *b"RGB ",
        *b"Lab ",
        vec![(*b"A2B0", mft2_lab_white())],
    ))
    .unwrap();
    let modern = Profile::new(&profile_with_pcs(
        *b"RGB ",
        *b"Lab ",
        vec![(*b"A2B0", mab_lab_white())],
    ))
    .unwrap();
    for source in [old, modern] {
        let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
        let mut output = [0.0; 3];
        transform
            .transform_f32(&[0.2, 0.4, 0.8], &mut output)
            .unwrap();
        for value in output {
            assert!(
                (value - 1.0).abs() < 0.01,
                "Lab white conversion failed: {output:?}"
            );
        }
    }
}

#[test]
fn optional_mab_stage_combinations_and_direction_are_checked() {
    let destination = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    for lut in [mab_b_only(), mab_b_matrix_m()] {
        let source = Profile::new(&profile(*b"RGB ", vec![(*b"A2B0", lut)])).unwrap();
        let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
        let mut output = [0.0; 3];
        transform
            .transform_f32(&[0.2, 0.4, 0.8], &mut output)
            .unwrap();
        assert!(output.iter().all(|value| value.is_finite()));
    }
    let wrong_direction = Profile::new(&profile(*b"RGB ", vec![(*b"A2B0", gray_mba())])).unwrap();
    assert!(matches!(
        Transform::new(&wrong_direction, &destination, TransformOptions::default()),
        Err(TransformError::InvalidProfile(_))
    ));
}

#[test]
fn compiled_lut_rejects_out_of_domain_intermediate_matrix_values() {
    let a_to_b = Profile::new(&profile(
        *b"RGB ",
        vec![(*b"A2B0", mab_b_matrix_m_with_scale(2.0))],
    ))
    .unwrap();
    let compiled_a_to_b = a_to_b
        .compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::Perceptual,
            TransformLimits::default(),
        )
        .unwrap();
    let mut pcs = [0.2, 0.3, 0.4];
    let before_pcs = pcs;
    assert!(compiled_a_to_b
        .transform_f32(&[0.75, 0.5, 0.25], &mut pcs)
        .is_err());
    assert_eq!(pcs, before_pcs, "strict mAB failure must not write output");

    let b_to_a = Profile::new(&profile(
        *b"RGB ",
        vec![(*b"B2A0", mba_b_matrix_m_with_scale(2.0))],
    ))
    .unwrap();
    let compiled_b_to_a = b_to_a
        .compile(
            TransformDirection::PcsToDevice,
            RenderingIntent::Perceptual,
            TransformLimits::default(),
        )
        .unwrap();
    let mut device = [0.6, 0.7, 0.8];
    let before_device = device;
    assert!(compiled_b_to_a
        .transform_f32(&[1.5, 1.0, 0.5], &mut device)
        .is_err());
    assert_eq!(
        device, before_device,
        "strict mBA failure must not write output"
    );
}

#[test]
fn matrix_colorimetric_route_requires_and_uses_media_white() {
    let profile = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    for intent in [
        RenderingIntent::Perceptual,
        RenderingIntent::RelativeColorimetric,
        RenderingIntent::Saturation,
        RenderingIntent::AbsoluteColorimetric,
    ] {
        let options = TransformOptions {
            rendering_intent: intent,
            ..TransformOptions::default()
        };
        let transform = Transform::new(&profile, &profile, options).unwrap();
        let mut output = [0.0; 3];
        transform
            .transform_f32(&[0.2, 0.4, 0.8], &mut output)
            .unwrap();
        for (actual, expected) in output.into_iter().zip([0.2, 0.4, 0.8]) {
            assert!(
                (actual - expected).abs() < 0.002,
                "intent route changed: {output:?}"
            );
        }
    }
}

#[test]
fn modern_lab_output_adapter_uses_neutral_midpoint() {
    let source = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let destination = Profile::new(&profile_with_pcs(
        *b"RGB ",
        *b"Lab ",
        vec![(*b"B2A0", mab_b2a_identity())],
    ))
    .unwrap();
    let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
    let mut output = [0.0; 3];
    transform
        .transform_f32(&[1.0, 1.0, 1.0], &mut output)
        .unwrap();
    assert!((output[0] - 1.0).abs() < 0.002);
    assert!(
        (output[1] - 0.5).abs() < 0.002,
        "modern Lab midpoint: {output:?}"
    );
    assert!(
        (output[2] - 0.5).abs() < 0.002,
        "modern Lab midpoint: {output:?}"
    );
}

#[test]
fn legacy_lab_non_neutral_vector_uses_500_200_physical_formula() {
    let source = Profile::new(&profile_with_pcs(
        *b"RGB ",
        *b"Lab ",
        vec![(*b"A2B0", mft2_lab_non_neutral())],
    ))
    .unwrap();
    let destination = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
    let mut actual = [0.0; 3];
    transform
        .transform_f32(&[0.2, 0.4, 0.8], &mut actual)
        .unwrap();
    let fy = (50.0 + 16.0) / 116.0;
    let fx = fy + 40.0 / 500.0;
    let fz = fy - -30.0 / 200.0;
    let cubic = |value: f32| {
        let value3 = value * value * value;
        if value3 > 216.0 / 24389.0 {
            value3
        } else {
            (116.0 * value - 16.0) / (24389.0 / 27.0)
        }
    };
    let expected = [cubic(fx), cubic(fy), cubic(fz)];
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.002,
            "non-neutral Lab mismatch: {actual} vs {expected}"
        );
    }
}

#[test]
fn default_clamp_clips_negative_input_before_gamma() {
    let source = Profile::new(&matrix_profile_with(
        512,
        [[0.9642, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.8249]],
    ))
    .unwrap();
    let transform = Transform::new(&source, &source, TransformOptions::default()).unwrap();
    let output = transform.transform_f32_vec(&[-0.5, 0.5, 0.5]).unwrap();
    assert_eq!(
        output[0], 0.0,
        "negative input must be clipped before gamma"
    );
}

#[test]
fn default_clamp_accepts_negative_fractional_gamma_input() {
    let source = Profile::new(&matrix_profile_with(
        563,
        [[0.9642, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.8249]],
    ))
    .unwrap();
    let transform = Transform::new(&source, &source, TransformOptions::default()).unwrap();
    let output = transform.transform_f32_vec(&[-0.1, 0.5, 0.5]).unwrap();
    assert!(output.iter().all(|value| value.is_finite()));
}

#[test]
fn default_clamp_clips_negative_destination_linear_before_inverse_gamma() {
    let source = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let destination = Profile::new(&matrix_profile_with(
        512,
        [[0.7, 0.2642, 0.0], [0.3, 0.7, 0.0], [0.0, 0.0, 0.8249]],
    ))
    .unwrap();
    let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
    assert!(transform.transform_f32_vec(&[1.0, 0.0, 0.0]).is_ok());
}

#[test]
fn strict_device_domain_rejects_without_modifying_output() {
    let profile = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let transform = Transform::new(
        &profile,
        &profile,
        TransformOptions {
            clamp: false,
            ..TransformOptions::default()
        },
    )
    .unwrap();
    let mut output = [0.25, 0.5, 0.75];
    let before = output;
    assert!(transform
        .transform_f32(&[-0.5, 0.5, 0.5], &mut output)
        .is_err());
    assert_eq!(output, before, "rejected input must not write output");
}

#[test]
fn clamped_device_domain_explicitly_clips_before_processing() {
    let profile = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let transform = Transform::new(&profile, &profile, TransformOptions::default()).unwrap();
    let mut output = [f32::NAN; 3];
    transform
        .transform_f32(&[-0.5, 0.5, 0.5], &mut output)
        .unwrap();
    assert_eq!(output[0], 0.0);
}

#[test]
fn strict_inverse_domain_rejects_unreachable_linear_value() {
    let source = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let destination = Profile::new(&matrix_profile_with(
        256,
        [[0.7, 0.2642, 0.0], [0.3, 0.7, 0.0], [0.0, 0.0, 0.8249]],
    ))
    .unwrap();
    let transform = Transform::new(
        &source,
        &destination,
        TransformOptions {
            clamp: false,
            ..TransformOptions::default()
        },
    )
    .unwrap();
    let mut output = [0.1, 0.2, 0.3];
    let before = output;
    assert!(transform
        .transform_f32(&[1.0, 0.0, 0.0], &mut output)
        .is_err());
    assert_eq!(output, before, "unreachable inverse must not write output");
}

#[test]
fn strict_lut_domain_rejects_out_of_range_before_lut_evaluation() {
    let source_bytes = rgb_mft_profile();
    let source = Profile::new(&source_bytes).unwrap();
    let destination = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let strict = Transform::new(
        &source,
        &destination,
        TransformOptions {
            clamp: false,
            ..TransformOptions::default()
        },
    )
    .unwrap();
    let mut rejected = [0.4, 0.5, 0.6];
    let before = rejected;
    assert!(strict
        .transform_f32(&[1.25, 0.5, 0.5], &mut rejected)
        .is_err());
    assert_eq!(rejected, before, "strict LUT rejection must be atomic");

    let clamped = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
    let mut accepted = [0.0; 3];
    clamped
        .transform_f32(&[1.25, 0.5, 0.5], &mut accepted)
        .unwrap();
    assert!(accepted.iter().all(|value| value.is_finite()));
}

#[test]
fn non_finite_device_samples_are_rejected_even_when_clamped() {
    let profile = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let transform = Transform::new(&profile, &profile, TransformOptions::default()).unwrap();
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut output = [0.25, 0.5, 0.75];
        let before = output;
        assert!(transform
            .transform_f32(&[value, 0.5, 0.5], &mut output)
            .is_err());
        assert_eq!(output, before);
    }
}

#[test]
fn full_mba_gray_clut_uses_declared_output_channels() {
    let source = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let destination = Profile::new(&profile(*b"GRAY", vec![(*b"B2A0", gray_full_mba())])).unwrap();
    let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
    assert_eq!(transform.output_channels(), 1);
    let output = transform.transform_f32_vec(&[1.0, 0.0, 0.0]).unwrap();
    assert!(
        (output[0] - 1.0).abs() < 1e-6,
        "full mBA CLUT output: {output:?}"
    );
}

#[test]
fn unequal_channels_without_clut_are_rejected() {
    let mut tag = vec![0; 80];
    tag[0..4].copy_from_slice(b"mBA ");
    tag[8] = 3;
    tag[9] = 1;
    u32_at(&mut tag, 12, 32); // B curves only
    for at in [32, 48, 64] {
        tag[at..at + 16].copy_from_slice(&padded_gamma_curve());
    }
    let source = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let destination = Profile::new(&profile(*b"GRAY", vec![(*b"B2A0", tag)])).unwrap();
    assert!(Transform::new(&source, &destination, TransformOptions::default()).is_err());
}

#[test]
fn mft_table_entries_respect_curve_entry_limit_before_allocation() {
    let raw = profile(*b"RGB ", vec![(*b"A2B0", identity_mft2())]);
    let mut limits = icc_profile::ParseLimits::default();
    limits.max_curve_entries = 1;
    let source = Profile::from_bytes_with_limits(&raw, limits).unwrap();
    let destination = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    assert!(Transform::new(&source, &destination, TransformOptions::default()).is_err());
}

#[test]
fn mab_curve_set_entries_respect_transform_limit_before_materialization() {
    let raw = profile(*b"RGB ", vec![(*b"A2B0", mab_b_matrix_m())]);
    let parsed = Profile::parse(&raw).unwrap();
    let limits = TransformLimits::builder()
        .max_curve_entries(2)
        .build()
        .unwrap();
    assert!(matches!(
        parsed.compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::RelativeColorimetric,
            limits,
        ),
        Err(TransformError::ResourceLimit(_))
    ));
}

#[test]
fn reported_lut_memory_is_the_compile_limit_boundary() {
    for (raw, direction) in [
        (
            profile(*b"RGB ", vec![(*b"A2B0", identity_mft2())]),
            TransformDirection::DeviceToPcs,
        ),
        (
            profile(*b"RGB ", vec![(*b"A2B0", mab_lab_white())]),
            TransformDirection::DeviceToPcs,
        ),
    ] {
        let parsed = Profile::parse(&raw).unwrap();
        let compiled = parsed
            .compile(
                direction,
                RenderingIntent::RelativeColorimetric,
                TransformLimits::default(),
            )
            .unwrap();
        let exact = compiled.memory_usage().unwrap().compiled_bytes();
        let exact_limits = TransformLimits::builder()
            .max_compiled_bytes(exact)
            .build()
            .unwrap();
        assert!(parsed
            .compile(
                direction,
                RenderingIntent::RelativeColorimetric,
                exact_limits
            )
            .is_ok());
        let under_limits = TransformLimits::builder()
            .max_compiled_bytes(exact - 1)
            .build()
            .unwrap();
        assert!(matches!(
            parsed.compile(
                direction,
                RenderingIntent::RelativeColorimetric,
                under_limits
            ),
            Err(TransformError::ResourceLimit(_))
        ));
    }
}

#[test]
fn one_shot_lut_parsing_defers_unused_matrix_validation() {
    let malformed = vec![(*b"rXYZ", vec![0u8])];
    let source = profile(
        *b"RGB ",
        [vec![(*b"A2B0", identity_mft2())], malformed.clone()].concat(),
    );
    let destination = profile(
        *b"RGB ",
        [vec![(*b"B2A0", identity_mft2())], malformed].concat(),
    );
    let result = Transform::from_bytes_with_limits(
        &source,
        &destination,
        TransformOptions::default(),
        icc_profile::ParseLimits::default(),
        TransformLimits::default(),
    );
    assert!(result.is_ok());
}

#[test]
fn reported_pair_memory_is_exact_for_mft_mab_and_mba_routes() {
    for (source_tag, source_lut, destination_tag, destination_lut) in [
        (*b"A2B0", identity_mft2(), *b"B2A0", identity_mft2()),
        (*b"A2B0", mab_lab_white(), *b"B2A0", mab_b2a_identity()),
    ] {
        let source = Profile::new(&profile(*b"RGB ", vec![(source_tag, source_lut)])).unwrap();
        let destination =
            Profile::new(&profile(*b"RGB ", vec![(destination_tag, destination_lut)])).unwrap();
        let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
        let exact = transform.memory_usage().unwrap().compiled_bytes();
        let exact_limits = TransformLimits::builder()
            .max_compiled_bytes(exact)
            .build()
            .unwrap();
        assert!(Transform::new_with_limits(
            &source,
            &destination,
            TransformOptions::default(),
            exact_limits,
        )
        .is_ok());
        let under_limits = TransformLimits::builder()
            .max_compiled_bytes(exact - 1)
            .build()
            .unwrap();
        assert!(matches!(
            Transform::new_with_limits(
                &source,
                &destination,
                TransformOptions::default(),
                under_limits,
            ),
            Err(TransformError::ResourceLimit(_))
        ));
    }
}

#[test]
fn mab_and_mba_stage_presence_matrix_is_checked() {
    let matrix = d50_matrix_rgb_profile();
    let input = Profile::new(&matrix).unwrap();
    for reverse in [false, true] {
        for mask in 0..16 {
            let has_a = mask & 1 != 0;
            let has_clut = mask & 2 != 0;
            let has_m = mask & 4 != 0;
            let has_matrix = mask & 8 != 0;
            let tag = stage_fixture(reverse, 3, 3, has_a, has_clut, has_m, has_matrix);
            let result = if reverse {
                let output = Profile::new(&profile(*b"RGB ", vec![(*b"B2A0", tag)])).unwrap();
                Transform::new(&input, &output, TransformOptions::default())
            } else {
                let source = Profile::new(&profile(*b"RGB ", vec![(*b"A2B0", tag)])).unwrap();
                Transform::new(&source, &input, TransformOptions::default())
            };
            let legal = has_a == has_clut && has_m == has_matrix;
            assert_eq!(
                result.is_ok(),
                legal,
                "unexpected {} stage result for mask {mask:04b}",
                if reverse { "mBA" } else { "mAB" }
            );
        }
    }
}

#[test]
fn gray_channel_changes_require_a_clut_pair() {
    let rgb = Profile::new(&d50_matrix_rgb_profile()).unwrap();
    let gray_a2b = Profile::new(&profile(
        *b"GRAY",
        vec![(
            *b"A2B0",
            stage_fixture(false, 1, 3, true, true, false, false),
        )],
    ))
    .unwrap();
    let gray_to_rgb = Transform::new(&gray_a2b, &rgb, TransformOptions::default()).unwrap();
    assert_eq!(gray_to_rgb.input_channels(), 1);
    assert_eq!(gray_to_rgb.output_channels(), 3);

    let rgb_b2a = Profile::new(&profile(
        *b"GRAY",
        vec![(
            *b"B2A0",
            stage_fixture(true, 3, 1, true, true, false, false),
        )],
    ))
    .unwrap();
    let rgb_to_gray = Transform::new(&rgb, &rgb_b2a, TransformOptions::default()).unwrap();
    assert_eq!(rgb_to_gray.input_channels(), 3);
    assert_eq!(rgb_to_gray.output_channels(), 1);

    for (reverse, input_channels, output_channels, color_space, tag_name) in [
        (false, 1, 3, *b"GRAY", *b"A2B0"),
        (true, 3, 1, *b"GRAY", *b"B2A0"),
    ] {
        let tag = stage_fixture(
            reverse,
            input_channels,
            output_channels,
            false,
            false,
            false,
            false,
        );
        let profile = Profile::new(&profile(color_space, vec![(tag_name, tag)])).unwrap();
        let result = if reverse {
            Transform::new(&rgb, &profile, TransformOptions::default())
        } else {
            Transform::new(&profile, &rgb, TransformOptions::default())
        };
        assert!(
            result.is_err(),
            "channel-changing LUT without CLUT was accepted"
        );
    }
}
