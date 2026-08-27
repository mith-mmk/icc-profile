use icc_profile::{ColorSpace, ParseLimits, Profile, Transform, TransformError, TransformOptions};

fn put_u32(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_be_bytes());
}
fn put_i32(data: &mut [u8], at: usize, value: i32) {
    put_u32(data, at, value as u32);
}

fn synthetic_rgb_profile() -> Vec<u8> {
    let mut profile = vec![0u8; 132 + 6 * 12];
    put_u32(&mut profile, 16, u32::from_be_bytes(*b"RGB "));
    put_u32(&mut profile, 20, u32::from_be_bytes(*b"XYZ "));
    put_u32(&mut profile, 36, u32::from_be_bytes(*b"acsp"));
    put_u32(&mut profile, 64, 0);
    put_u32(&mut profile, 128, 6);
    let tags: [([u8; 4], Vec<u8>); 6] = [
        (*b"rXYZ", xyz(0.4124, 0.2126, 0.0193)),
        (*b"gXYZ", xyz(0.3576, 0.7152, 0.1192)),
        (*b"bXYZ", xyz(0.1805, 0.0722, 0.9505)),
        (*b"rTRC", gamma(2.2)),
        (*b"gTRC", gamma(2.2)),
        (*b"bTRC", gamma(2.2)),
    ];
    let mut offset = profile.len();
    for (index, (signature, tag)) in tags.into_iter().enumerate() {
        offset = (offset + 3) & !3;
        profile.resize(offset + tag.len(), 0);
        profile[offset..offset + tag.len()].copy_from_slice(&tag);
        let entry = 132 + index * 12;
        put_u32(&mut profile, entry, u32::from_be_bytes(signature));
        put_u32(&mut profile, entry + 4, offset as u32);
        put_u32(&mut profile, entry + 8, tag.len() as u32);
        offset += tag.len();
    }
    let profile_len = profile.len() as u32;
    put_u32(&mut profile, 0, profile_len);
    profile
}

fn xyz(x: f32, y: f32, z: f32) -> Vec<u8> {
    let mut tag = vec![0u8; 20];
    put_u32(&mut tag, 0, u32::from_be_bytes(*b"XYZ "));
    put_i32(&mut tag, 8, (x * 65536.0).round() as i32);
    put_i32(&mut tag, 12, (y * 65536.0).round() as i32);
    put_i32(&mut tag, 16, (z * 65536.0).round() as i32);
    tag
}

fn gamma(value: f32) -> Vec<u8> {
    let mut tag = vec![0u8; 16];
    put_u32(&mut tag, 0, u32::from_be_bytes(*b"curv"));
    put_u32(&mut tag, 8, 1);
    let encoded = (value * 256.0).round().clamp(0.0, 65535.0) as u16;
    tag[12..14].copy_from_slice(&encoded.to_be_bytes());
    tag
}

fn sampled(values: &[u16]) -> Vec<u8> {
    let mut tag = vec![0u8; 12 + values.len() * 2];
    put_u32(&mut tag, 0, u32::from_be_bytes(*b"curv"));
    put_u32(&mut tag, 8, values.len() as u32);
    for (index, value) in values.iter().enumerate() {
        tag[12 + index * 2..14 + index * 2].copy_from_slice(&value.to_be_bytes());
    }
    tag
}

fn para(function: u16, values: &[f32]) -> Vec<u8> {
    let mut tag = vec![0u8; 12 + values.len() * 4];
    put_u32(&mut tag, 0, u32::from_be_bytes(*b"para"));
    tag[8..10].copy_from_slice(&function.to_be_bytes());
    for (index, value) in values.iter().enumerate() {
        put_i32(&mut tag, 12 + index * 4, (value * 65536.0).round() as i32);
    }
    tag
}

fn replace_last_trc(mut profile: Vec<u8>, tag: &[u8]) -> Vec<u8> {
    let entry = 132 + 5 * 12;
    let offset = u32::from_be_bytes(profile[entry + 4..entry + 8].try_into().unwrap()) as usize;
    profile.resize(offset + tag.len(), 0);
    profile[offset..offset + tag.len()].copy_from_slice(tag);
    put_u32(&mut profile, entry + 8, tag.len() as u32);
    let profile_len = profile.len() as u32;
    put_u32(&mut profile, 0, profile_len);
    profile
}

fn synthetic_gray_profile() -> Vec<u8> {
    let tag = gamma(2.2);
    let mut profile = vec![0u8; 132 + 12 + tag.len()];
    put_u32(&mut profile, 16, u32::from_be_bytes(*b"GRAY"));
    put_u32(&mut profile, 20, u32::from_be_bytes(*b"XYZ "));
    put_u32(&mut profile, 36, u32::from_be_bytes(*b"acsp"));
    put_u32(&mut profile, 64, 1);
    put_u32(&mut profile, 128, 1);
    let offset = 144usize;
    put_u32(&mut profile, 132, u32::from_be_bytes(*b"kTRC"));
    put_u32(&mut profile, 136, offset as u32);
    put_u32(&mut profile, 140, tag.len() as u32);
    profile[offset..offset + tag.len()].copy_from_slice(&tag);
    let profile_len = profile.len() as u32;
    put_u32(&mut profile, 0, profile_len);
    profile
}

#[test]
fn profile_rejects_truncated_and_huge_inputs() {
    assert!(matches!(
        Profile::new(&[0; 128]),
        Err(TransformError::InvalidProfile(_))
    ));
    let mut limits = ParseLimits::default();
    limits.max_profile_size = 8;
    assert!(matches!(
        Profile::from_bytes_with_limits(&synthetic_rgb_profile(), limits),
        Err(TransformError::ResourceLimit(_))
    ));
}

#[test]
fn matrix_trc_transform_is_thread_shareable_and_quantized() {
    let bytes = synthetic_rgb_profile();
    let profile = Profile::new(&bytes).unwrap();
    assert_eq!(profile.color_space(), ColorSpace::Rgb);
    let transform = Transform::new(&profile, &profile, TransformOptions::default()).unwrap();
    let input = [0u8, 32, 128, 255, 64, 200];
    let mut output = [0u8; 6];
    transform.transform_u8(&input, &mut output).unwrap();
    for (a, b) in input.iter().zip(output) {
        assert!((*a as i16 - b as i16).abs() <= 1, "{a} -> {b}");
    }
    let transform = std::sync::Arc::new(transform);
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let t = transform.clone();
            std::thread::spawn(move || t.transform_f32_vec(&[0.1, 0.5, 0.9]).unwrap())
        })
        .collect();
    for handle in handles {
        assert_eq!(handle.join().unwrap().len(), 3);
    }
}

#[test]
fn gray_transform_has_one_channel_and_rgb_to_gray_is_safe() {
    let rgb = Profile::new(&synthetic_rgb_profile()).unwrap();
    let gray = Profile::new(&synthetic_gray_profile()).unwrap();
    let same = Transform::new(&gray, &gray, TransformOptions::default()).unwrap();
    let mut gray_out = [0.0];
    same.transform_f32(&[0.5], &mut gray_out).unwrap();
    assert!((gray_out[0] - 0.5).abs() < 0.01);
    let to_gray = Transform::new(&rgb, &gray, TransformOptions::default()).unwrap();
    let mut out = [0.0];
    to_gray.transform_f32(&[1.0, 1.0, 1.0], &mut out).unwrap();
    assert_eq!(out.len(), 1);
    assert!(out[0] > 0.9);
}

#[test]
fn unsupported_intent_and_bpc_are_explicit_errors() {
    let bytes = synthetic_rgb_profile();
    let profile = Profile::new(&bytes).unwrap();
    let mut options = TransformOptions::default();
    options.rendering_intent = icc_profile::RenderingIntent::Perceptual;
    assert!(matches!(
        Transform::new(&profile, &profile, options),
        Err(TransformError::UnsupportedProfileFeature(_))
    ));
    options = TransformOptions::default();
    options.black_point_compensation = true;
    assert!(matches!(
        Transform::new(&profile, &profile, options),
        Err(TransformError::UnsupportedProfileFeature(_))
    ));
}

#[test]
fn xclr_signatures_accept_only_icc_hex_channel_counts() {
    for lead in [b'2', b'9', b'A', b'F'] {
        let mut bytes = synthetic_rgb_profile();
        put_u32(&mut bytes, 16, u32::from_be_bytes([lead, b'C', b'L', b'R']));
        let profile = Profile::new(&bytes).unwrap();
        assert_eq!(
            profile.color_space(),
            ColorSpace::NColor(if lead <= b'9' {
                lead - b'0'
            } else {
                lead - b'A' + 10
            })
        );
    }
    for lead in [b'1', b':', b'@', b'G'] {
        let mut bytes = synthetic_rgb_profile();
        put_u32(&mut bytes, 16, u32::from_be_bytes([lead, b'C', b'L', b'R']));
        assert!(matches!(
            Profile::new(&bytes),
            Err(TransformError::UnsupportedProfileFeature("color space"))
        ));
    }
}

#[test]
fn constant_sampled_trc_is_rejected() {
    let mut bytes = synthetic_rgb_profile();
    let offset = u32::from_be_bytes(
        bytes[132 + 3 * 12 + 4..132 + 3 * 12 + 8]
            .try_into()
            .unwrap(),
    ) as usize;
    let mut curve = vec![0u8; 16];
    put_u32(&mut curve, 0, u32::from_be_bytes(*b"curv"));
    put_u32(&mut curve, 8, 2);
    curve[12..14].copy_from_slice(&1000u16.to_be_bytes());
    curve[14..16].copy_from_slice(&1000u16.to_be_bytes());
    bytes[offset..offset + curve.len()].copy_from_slice(&curve);
    assert!(matches!(
        Profile::new(&bytes),
        Err(TransformError::InvalidProfile(_))
    ));
}

#[test]
fn non_monotonic_sampled_trc_is_rejected() {
    let mut bytes = synthetic_rgb_profile();
    let entry = 132 + 5 * 12;
    let offset = u32::from_be_bytes(bytes[entry + 4..entry + 8].try_into().unwrap()) as usize;
    bytes.resize(bytes.len() + 2, 0);
    let mut curve = vec![0u8; 18];
    put_u32(&mut curve, 0, u32::from_be_bytes(*b"curv"));
    put_u32(&mut curve, 8, 3);
    curve[12..14].copy_from_slice(&0u16.to_be_bytes());
    curve[14..16].copy_from_slice(&65535u16.to_be_bytes());
    curve[16..18].copy_from_slice(&1000u16.to_be_bytes());
    bytes[offset..offset + curve.len()].copy_from_slice(&curve);
    put_u32(&mut bytes, entry + 8, curve.len() as u32);
    let profile_len = bytes.len() as u32;
    put_u32(&mut bytes, 0, profile_len);
    assert!(matches!(
        Profile::new(&bytes),
        Err(TransformError::InvalidProfile(_))
    ));
}

#[test]
fn sampled_curve_plateau_uses_annex_f1_endpoints() {
    let bytes = replace_last_trc(synthetic_rgb_profile(), &sampled(&[0, 0, 65535]));
    let profile = Profile::new(&bytes).unwrap();
    let transform = Transform::new(&profile, &profile, TransformOptions::default()).unwrap();
    let mut output = [0.0; 3];
    transform
        .transform_f32(&[0.0, 0.0, 0.0], &mut output)
        .unwrap();
    assert!(
        (output[2] - 0.5).abs() < 1e-5,
        "internal plateau: {}",
        output[2]
    );

    let bytes = replace_last_trc(synthetic_rgb_profile(), &sampled(&[0, 65535, 65535]));
    let profile = Profile::new(&bytes).unwrap();
    let transform = Transform::new(&profile, &profile, TransformOptions::default()).unwrap();
    transform
        .transform_f32(&[0.0, 0.0, 1.0], &mut output)
        .unwrap();
    assert!(
        (output[2] - 0.5).abs() < 1e-5,
        "endpoint plateau: {}",
        output[2]
    );
}

#[test]
fn parametric_inverse_handles_plateau_decrease_and_rejects_bad_shapes() {
    let bytes = replace_last_trc(synthetic_rgb_profile(), &para(1, &[1.0, 1.0, -0.5]));
    let profile = Profile::new(&bytes).unwrap();
    let transform = Transform::new(&profile, &profile, TransformOptions::default()).unwrap();
    let mut output = [0.0; 3];
    transform
        .transform_f32(&[0.0, 0.0, 0.0], &mut output)
        .unwrap();
    assert!(
        (output[2] - 0.5).abs() < 1e-4,
        "parametric plateau: {}",
        output[2]
    );

    let bytes = replace_last_trc(
        synthetic_rgb_profile(),
        &para(4, &[1.0, -1.0, 1.0, -1.0, 0.0, 0.0, 0.0]),
    );
    let profile = Profile::new(&bytes).unwrap();
    let transform = Transform::new(&profile, &profile, TransformOptions::default()).unwrap();
    transform
        .transform_f32(&[0.0, 0.0, 0.25], &mut output)
        .unwrap();
    assert!(
        (output[2] - 0.25).abs() < 1e-4,
        "decreasing parametric curve: {}",
        output[2]
    );

    let bad = replace_last_trc(
        synthetic_rgb_profile(),
        &para(3, &[1.0, -1.0, 1.0, 1.0, 0.5]),
    );
    assert!(matches!(
        Profile::new(&bad),
        Err(TransformError::UnsupportedProfileFeature(_))
    ));
    let constant = replace_last_trc(
        synthetic_rgb_profile(),
        &para(3, &[1.0, 0.0, 1.0, 0.0, 0.0]),
    );
    assert!(matches!(
        Profile::new(&constant),
        Err(TransformError::UnsupportedProfileFeature(_))
    ));

    let micro_non_monotonic = replace_last_trc(
        synthetic_rgb_profile(),
        &para(4, &[1.0, 1.0, 0.0, -1.0, 1.0 / 65536.0, 0.0, 1.0 / 65536.0]),
    );
    assert!(matches!(
        Profile::new(&micro_non_monotonic),
        Err(TransformError::UnsupportedProfileFeature(_))
    ));
}
