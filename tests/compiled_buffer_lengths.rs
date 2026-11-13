//! Buffer diagnostics for compiled stages, using self-contained v4 LUT profiles.
use icc_profile::{
    CompiledProfile, Profile, RenderingIntent, TransformDirection, TransformError, TransformLimits,
};

fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
}

fn stage(device_channels: usize, direction: TransformDirection) -> CompiledProfile {
    let reverse = direction == TransformDirection::PcsToDevice;
    let (inputs, outputs) = if reverse {
        (3, device_channels)
    } else {
        (device_channels, 3)
    };
    let mut tag = vec![0; 52 + (1 << inputs) * outputs * 2];
    tag[..4].copy_from_slice(if reverse { b"mBA " } else { b"mAB " });
    tag[8] = inputs as u8;
    tag[9] = outputs as u8;
    put_u32(&mut tag, 24, 32);
    tag[32..32 + inputs].fill(2);
    tag[48] = 2;
    // A constant-zero CLUT and identity A/B curves provide known valid stages.
    for (offset_field, channels) in [(12, 3), (28, device_channels)] {
        let offset = tag.len();
        put_u32(&mut tag, offset_field, offset as u32);
        for _ in 0..channels {
            let mut curve = [0; 12];
            curve[..4].copy_from_slice(b"curv");
            tag.extend_from_slice(&curve);
        }
    }
    let mut bytes = vec![0; 144];
    bytes[8] = 4;
    bytes[12..16].copy_from_slice(if device_channels == 4 {
        b"prtr"
    } else {
        b"mntr"
    });
    bytes[16..20].copy_from_slice(if device_channels == 4 {
        b"CMYK"
    } else {
        b"RGB "
    });
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    put_u32(&mut bytes, 128, 1);
    bytes[132..136].copy_from_slice(if reverse { b"B2A1" } else { b"A2B1" });
    put_u32(&mut bytes, 136, 144);
    put_u32(&mut bytes, 140, tag.len() as u32);
    bytes.extend(tag);
    let size = bytes.len();
    put_u32(&mut bytes, 0, size as u32);
    let compiled = Profile::parse(&bytes)
        .unwrap()
        .compile(
            direction,
            RenderingIntent::RelativeColorimetric,
            TransformLimits::default(),
        )
        .unwrap();
    assert_eq!(
        (compiled.input_channels(), compiled.output_channels()),
        (inputs, outputs)
    );
    compiled
}

fn stages() -> Vec<CompiledProfile> {
    vec![
        stage(3, TransformDirection::DeviceToPcs),
        stage(3, TransformDirection::PcsToDevice),
        stage(4, TransformDirection::DeviceToPcs),
        stage(4, TransformDirection::PcsToDevice),
    ]
}

fn assert_length_error(
    stage: &CompiledProfile,
    input_len: usize,
    output_len: usize,
    expected: usize,
    actual: usize,
) {
    let input = vec![0.25; input_len];
    // Include a NaN payload and negative zero to check preservation bit-for-bit.
    let mut output: Vec<f32> = (0..output_len)
        .map(|i| match i % 3 {
            0 => f32::from_bits(0x7fc0_0123),
            1 => -0.0,
            _ => 42.0,
        })
        .collect();
    let before: Vec<_> = output.iter().map(|value| value.to_bits()).collect();
    let error = stage.transform_f32(&input, &mut output).unwrap_err();
    assert!(
        matches!(error, TransformError::InvalidBufferLength { expected: e, actual: a } if e == expected && a == actual),
        "{error:?}, expected {expected}, actual {actual}"
    );
    assert_eq!(
        output
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        before
    );
}

#[test]
fn short_and_long_inputs_report_input_length() {
    for stage in stages() {
        let n = stage.input_channels();
        for actual in [0, n - 1, n + 1] {
            assert_length_error(&stage, actual, stage.output_channels(), n, actual);
        }
    }
}

#[test]
fn short_and_long_outputs_report_output_length() {
    for stage in stages() {
        let n = stage.output_channels();
        for actual in [0, n - 1, n + 1] {
            assert_length_error(&stage, stage.input_channels(), actual, n, actual);
        }
    }
}

#[test]
fn invalid_input_length_takes_precedence_over_invalid_output_length() {
    for stage in stages() {
        let n = stage.input_channels();
        let m = stage.output_channels();
        for input_len in [0, n - 1, n + 1] {
            for output_len in [0, m - 1, m + 1] {
                assert_length_error(&stage, input_len, output_len, n, input_len);
            }
        }
    }
}

#[test]
fn valid_lengths_evaluate_the_compiled_stage() {
    for stage in stages() {
        let input = vec![0.25; stage.input_channels()];
        let mut output = vec![42.0; stage.output_channels()];
        stage.transform_f32(&input, &mut output).unwrap();
        assert_eq!(output, vec![0.0; stage.output_channels()]);
    }
}
