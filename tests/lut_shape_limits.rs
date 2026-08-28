//! Structural LUT-shape regressions.
//!
//! The selected matrix stage must be range-checked before the materializer
//! allocates any of the A/B/M curve sets.  Both ICC directions are covered.

use icc_profile::{
    ParseLimits, Profile, RenderingIntent, TransformDirection, TransformError, TransformLimits,
};

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn profile_with_tag(signature: [u8; 4], tag: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 144];
    bytes[8..12].copy_from_slice(&[4, 0, 0, 0]);
    bytes[12..16].copy_from_slice(b"mntr");
    bytes[16..20].copy_from_slice(b"RGB ");
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    put_u32(&mut bytes, 64, 1);
    put_u32(&mut bytes, 128, 1);
    let offset = bytes.len();
    bytes.extend_from_slice(tag);
    bytes[132..136].copy_from_slice(&signature);
    put_u32(&mut bytes, 136, offset as u32);
    put_u32(&mut bytes, 140, tag.len() as u32);
    let length = bytes.len() as u32;
    put_u32(&mut bytes, 0, length);
    bytes
}

fn identity_curve() -> [u8; 12] {
    let mut curve = [0; 12];
    curve[..4].copy_from_slice(b"curv");
    curve
}

fn truncated_matrix_lut(signature: &[u8; 4]) -> Vec<u8> {
    // B curves occupy [32, 68), M curves [68, 104).  The matrix offset is
    // exactly at the end, so its 48-byte payload is truncated without making
    // either the stage pair or the curve ranges malformed.
    let mut tag = vec![0; 104];
    tag[..4].copy_from_slice(signature);
    tag[8] = 3;
    tag[9] = 3;
    put_u32(&mut tag, 12, 32); // B curves
    put_u32(&mut tag, 16, 104); // selected matrix, truncated
    put_u32(&mut tag, 20, 68); // M curves
    for (index, curve) in [32usize, 44, 56, 68, 80, 92]
        .into_iter()
        .zip([identity_curve(); 6])
    {
        tag[index..index + curve.len()].copy_from_slice(&curve);
    }
    tag
}

fn assert_rejected_before_materialization(tag_signature: [u8; 4], direction: TransformDirection) {
    let profile_bytes = profile_with_tag(
        if direction == TransformDirection::DeviceToPcs {
            *b"A2B0"
        } else {
            *b"B2A0"
        },
        &truncated_matrix_lut(&tag_signature),
    );
    let profile = Profile::parse(&profile_bytes).unwrap();
    let error = profile
        .compile(
            direction,
            RenderingIntent::Perceptual,
            TransformLimits::default(),
        )
        .unwrap_err();
    assert!(
        matches!(error, TransformError::InvalidProfile(_)),
        "unexpected error: {error:?}"
    );
}

#[test]
fn truncated_mab_matrix_is_rejected_before_curve_materialization() {
    assert_rejected_before_materialization(*b"mAB ", TransformDirection::DeviceToPcs);
}

#[test]
fn truncated_mba_matrix_is_rejected_before_curve_materialization() {
    assert_rejected_before_materialization(*b"mBA ", TransformDirection::PcsToDevice);
}

fn sampled_curve(entries: u32) -> Vec<u8> {
    let mut curve = vec![0; 12 + entries as usize * 2];
    curve[..4].copy_from_slice(b"curv");
    put_u32(&mut curve, 8, entries);
    for index in 0..entries {
        let value = (index * 65535 / (entries - 1)) as u16;
        curve[12 + index as usize * 2..14 + index as usize * 2]
            .copy_from_slice(&value.to_be_bytes());
    }
    curve
}

fn rgb_mab_with_sampled_curves(signature: [u8; 4]) -> Vec<u8> {
    let curve = sampled_curve(32);
    let mut tag = vec![0; 32 + curve.len() * 3];
    tag[..4].copy_from_slice(&signature);
    tag[8] = 3;
    tag[9] = 3;
    put_u32(&mut tag, 12, 32);
    for index in 0..3 {
        let start = 32 + index * curve.len();
        tag[start..start + curve.len()].copy_from_slice(&curve);
    }
    tag
}

#[test]
fn profile_parse_curve_limit_applies_to_selected_mab_and_mba_sets() {
    for (signature, direction, tag_name) in [
        (*b"mAB ", TransformDirection::DeviceToPcs, *b"A2B0"),
        (*b"mBA ", TransformDirection::PcsToDevice, *b"B2A0"),
    ] {
        let bytes = profile_with_tag(tag_name, &rgb_mab_with_sampled_curves(signature));
        let mut limited_parse = ParseLimits::default();
        limited_parse.max_curve_entries = 16;
        let limited = Profile::parse_with_limits(&bytes, limited_parse)
            .unwrap()
            .compile(
                direction,
                RenderingIntent::Perceptual,
                TransformLimits::default(),
            );
        assert!(
            matches!(limited, Err(TransformError::ResourceLimit(_))),
            "32-entry curve bypassed profile parse limit for {signature:?}"
        );

        let mut exact_parse = ParseLimits::default();
        exact_parse.max_curve_entries = 32;
        let exact = Profile::parse_with_limits(&bytes, exact_parse)
            .unwrap()
            .compile(
                direction,
                RenderingIntent::Perceptual,
                TransformLimits::default(),
            );
        assert!(
            exact.is_ok(),
            "exact curve limit rejected {signature:?}: {exact:?}"
        );
    }
}

fn mft_lut(wide: bool, grid: u8) -> Vec<u8> {
    let entries = if wide { 2usize } else { 256usize };
    let sample_width = if wide { 2usize } else { 1usize };
    let header = if wide { 52usize } else { 48usize };
    let clut_samples = usize::from(grid).pow(3) * 3;
    let table_samples = 3 * entries * 2;
    let mut tag = vec![0; header + (clut_samples + table_samples) * sample_width];
    tag[..4].copy_from_slice(if wide { b"mft2" } else { b"mft1" });
    tag[8] = 3;
    tag[9] = 3;
    tag[10] = grid;
    for index in 0..9 {
        put_u32(&mut tag, 12 + index * 4, 0x0001_0000);
    }
    if wide {
        tag[48..50].copy_from_slice(&2u16.to_be_bytes());
        tag[50..52].copy_from_slice(&2u16.to_be_bytes());
    }
    let mut at = header;
    for _ in 0..3 {
        if wide {
            tag[at..at + 2].copy_from_slice(&0u16.to_be_bytes());
            tag[at + 2..at + 4].copy_from_slice(&u16::MAX.to_be_bytes());
            at += 4;
        } else {
            tag[at] = 0;
            tag[at + 1] = u8::MAX;
            at += 2;
        }
    }
    for _ in 0..clut_samples {
        if wide {
            tag[at..at + 2].copy_from_slice(&0u16.to_be_bytes());
            at += 2;
        } else {
            tag[at] = 0;
            at += 1;
        }
    }
    for _ in 0..3 {
        if wide {
            tag[at..at + 2].copy_from_slice(&0u16.to_be_bytes());
            tag[at + 2..at + 4].copy_from_slice(&u16::MAX.to_be_bytes());
            at += 4;
        } else {
            tag[at] = 0;
            tag[at + 1] = u8::MAX;
            at += 2;
        }
    }
    tag
}

#[test]
fn mft_degenerate_grid_is_rejected_before_materialization_in_both_directions() {
    for wide in [false, true] {
        for (tag_name, direction) in [
            (*b"A2B0", TransformDirection::DeviceToPcs),
            (*b"B2A0", TransformDirection::PcsToDevice),
        ] {
            for grid in [0, 1] {
                let malformed = profile_with_tag(tag_name, &mft_lut(wide, grid));
                let profile = Profile::parse(&malformed).unwrap();
                let error = profile
                    .compile(
                        direction,
                        RenderingIntent::Perceptual,
                        TransformLimits::default(),
                    )
                    .unwrap_err();
                assert!(
                    matches!(error, TransformError::InvalidProfile(_)),
                    "mft{} grid {grid} {direction:?}: {error:?}",
                    if wide { 2 } else { 1 }
                );
            }
            let valid = profile_with_tag(tag_name, &mft_lut(wide, 2));
            assert!(
                Profile::parse(&valid)
                    .unwrap()
                    .compile(
                        direction,
                        RenderingIntent::Perceptual,
                        TransformLimits::default(),
                    )
                    .is_ok(),
                "mft{} grid 2 positive control failed for {direction:?}",
                if wide { 2 } else { 1 }
            );
        }
    }
}

#[test]
fn mft_owner_storage_is_counted_at_exact_and_one_under_limits() {
    let bytes = profile_with_tag(*b"A2B0", &mft_lut(true, 16));
    let profile = Profile::parse(&bytes).unwrap();
    let payload_bytes = 49_200usize;
    let owner_bytes = payload_bytes
        .checked_add(6 * std::mem::size_of::<Vec<f32>>())
        .and_then(|bytes| bytes.checked_add(3 * std::mem::size_of::<usize>()))
        .unwrap();
    // The public integration test cannot name the private Arc-owned headers;
    // discover their exact contribution through the monotonic admission
    // boundary. The independent private S3 test checks the same boundary
    // directly with size_of::<LutTransform>() + size_of::<CompiledDirection>().
    let mut low = owner_bytes;
    let mut high = owner_bytes.checked_add(4096).unwrap();
    while low < high {
        let mid = low + (high - low) / 2;
        let limit = TransformLimits::builder()
            .max_compiled_bytes(mid)
            .build()
            .unwrap();
        if profile
            .compile(
                TransformDirection::DeviceToPcs,
                RenderingIntent::Perceptual,
                limit,
            )
            .is_ok()
        {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    let exact = TransformLimits::builder()
        .max_compiled_bytes(low)
        .build()
        .unwrap();
    assert!(profile
        .compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::Perceptual,
            exact,
        )
        .is_ok());
    let one_under = TransformLimits::builder()
        .max_compiled_bytes(low - 1)
        .build()
        .unwrap();
    assert!(matches!(
        profile.compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::Perceptual,
            one_under,
        ),
        Err(TransformError::ResourceLimit(_))
    ));
}
