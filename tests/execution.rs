use icc_profile::{ExecutionLimits, Profile, RenderingIntent, Transform, TransformError};

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn gray_profile() -> Profile {
    gray_profile_with_samples(&[256])
}

fn gray_profile_with_samples(samples: &[u16]) -> Profile {
    let tag_size = 12 + samples.len() * 2;
    let mut bytes = vec![0; 144 + tag_size];
    bytes[8..12].copy_from_slice(&[4, 0, 0, 0]);
    bytes[12..16].copy_from_slice(b"mntr");
    bytes[16..20].copy_from_slice(b"GRAY");
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    put_u32(&mut bytes, 64, 1);
    put_u32(&mut bytes, 128, 1);
    bytes[132..136].copy_from_slice(b"kTRC");
    put_u32(&mut bytes, 136, 144);
    put_u32(&mut bytes, 140, tag_size as u32);
    bytes[144..148].copy_from_slice(b"curv");
    put_u32(&mut bytes, 152, samples.len() as u32);
    for (index, sample) in samples.iter().enumerate() {
        let offset = 156 + index * 2;
        bytes[offset..offset + 2].copy_from_slice(&sample.to_be_bytes());
    }
    let length = bytes.len() as u32;
    put_u32(&mut bytes, 0, length);
    Profile::parse(&bytes).unwrap()
}

fn gray_transform() -> Transform {
    let profile = gray_profile();
    Transform::new(
        &profile,
        &profile,
        icc_profile::TransformOptions {
            rendering_intent: RenderingIntent::RelativeColorimetric,
            ..icc_profile::TransformOptions::default()
        },
    )
    .unwrap()
}

fn sampled_gray_transform() -> Transform {
    let source = gray_profile();
    let destination = gray_profile_with_samples(&[16384, 49151]);
    Transform::new(
        &source,
        &destination,
        icc_profile::TransformOptions {
            rendering_intent: RenderingIntent::RelativeColorimetric,
            clamp: false,
            ..icc_profile::TransformOptions::default()
        },
    )
    .unwrap()
}

#[test]
fn f32_vec_limits_are_checked_before_output_reservation() {
    let transform = gray_transform();
    let limits = ExecutionLimits::builder()
        .max_output_bytes(4)
        .build()
        .unwrap();
    let result = transform.transform_f32_vec_with_limits(&[0.25, 0.75], limits);
    assert!(matches!(result, Err(TransformError::ResourceLimit(_))));
}

#[test]
fn f32_vec_output_bound_covers_exact_under_empty_and_default() {
    let transform = gray_transform();
    let exact = transform
        .transform_f32_vec_with_limits(
            &[0.25, 0.75],
            ExecutionLimits::builder()
                .max_output_bytes(8)
                .build()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(exact.len(), 2);

    let under = transform.transform_f32_vec_with_limits(
        &[0.25, 0.75],
        ExecutionLimits::builder()
            .max_output_bytes(7)
            .build()
            .unwrap(),
    );
    assert!(matches!(under, Err(TransformError::ResourceLimit(_))));

    let empty = transform
        .transform_f32_vec_with_limits(
            &[],
            ExecutionLimits::builder()
                .max_output_bytes(0)
                .build()
                .unwrap(),
        )
        .unwrap();
    assert!(empty.is_empty());
    assert_eq!(
        ExecutionLimits::default().max_output_bytes(),
        64 * 1024 * 1024
    );
}

#[test]
fn integer_direct_and_worker_paths_match_borrowed_f32_core() {
    let transform = gray_transform();
    let input8: Vec<u8> = (0..257).map(|value| (value % 256) as u8).collect();
    let input16: Vec<u16> = (0..257)
        .map(|value| ((value * 257) % 65536) as u16)
        .collect();
    let normalized8: Vec<f32> = input8
        .iter()
        .map(|value| f32::from(*value) / 255.0)
        .collect();
    let normalized16: Vec<f32> = input16
        .iter()
        .map(|value| f32::from(*value) / 65535.0)
        .collect();
    let mut expected8 = vec![0.0; input8.len()];
    let mut expected16 = vec![0.0; input16.len()];
    transform
        .transform_f32(&normalized8, &mut expected8)
        .unwrap();
    transform
        .transform_f32(&normalized16, &mut expected16)
        .unwrap();
    let expected8: Vec<u8> = expected8
        .into_iter()
        .map(|value| (value * 255.0).round() as u8)
        .collect();
    let expected16: Vec<u16> = expected16
        .into_iter()
        .map(|value| (value * 65535.0).round() as u16)
        .collect();

    let mut actual8 = vec![0; input8.len()];
    let mut actual16 = vec![0; input16.len()];
    transform.transform_u8(&input8, &mut actual8).unwrap();
    transform.transform_u16(&input16, &mut actual16).unwrap();
    assert_eq!(actual8, expected8);
    assert_eq!(actual16, expected16);

    let mut worker = transform.worker();
    actual8.fill(0);
    actual16.fill(0);
    worker.transform_u8(&input8, &mut actual8).unwrap();
    worker.transform_u16(&input16, &mut actual16).unwrap();
    assert_eq!(actual8, expected8);
    assert_eq!(actual16, expected16);
}

#[test]
fn integer_length_errors_leave_outputs_unchanged() {
    let transform = gray_transform();
    let mut output8 = [77u8; 2];
    let mut output16 = [1234u16; 2];
    assert!(matches!(
        transform.transform_u8(&[0, 1, 2], &mut output8),
        Err(TransformError::InvalidBufferLength { .. })
    ));
    assert_eq!(output8, [77; 2]);
    assert!(matches!(
        transform.transform_u16(&[0, 1, 2], &mut output16),
        Err(TransformError::InvalidBufferLength { .. })
    ));
    assert_eq!(output16, [1234; 2]);
}

#[test]
fn integer_late_inverse_error_preserves_the_entire_output() {
    let transform = sampled_gray_transform();
    let good_input = vec![32768u16; 4097];
    let mut good_output = vec![0xCAFEu16; good_input.len()];
    transform
        .transform_u16(&good_input, &mut good_output)
        .unwrap();
    assert!(good_output.iter().all(|value| *value != 0xCAFE));
    assert!(good_output
        .iter()
        .all(|value| (*value as i32 - 32768).abs() <= 2));

    let mut input = good_input;
    *input.last_mut().unwrap() = u16::MAX;
    let mut direct = vec![0xCAFEu16; input.len()];
    assert!(matches!(
        transform.transform_u16(&input, &mut direct),
        Err(TransformError::MalformedProfile(_))
    ));
    assert!(direct.iter().all(|value| *value == 0xCAFE));

    let mut worker_output = vec![0xCAFEu16; input.len()];
    let mut worker = transform.worker();
    assert!(matches!(
        worker.transform_u16(&input, &mut worker_output),
        Err(TransformError::MalformedProfile(_))
    ));
    assert!(worker_output.iter().all(|value| *value == 0xCAFE));
}
