//! Matrix/TRC compile planning regressions.
//!
//! These checks exercise the bounded planning seam without depending on an
//! allocator implementation.  The external review harness additionally
//! verifies that rejection happens before decoded table allocation.

use icc_profile::{
    Profile, RenderingIntent, Transform, TransformDirection, TransformError, TransformLimits,
    TransformOptions,
};

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn sampled_curve() -> Vec<u8> {
    let mut curve = vec![0; 12 + 1024 * 2];
    curve[..4].copy_from_slice(b"curv");
    put_u32(&mut curve, 8, 1024);
    for index in 0..1024u32 {
        let value = (index * 65535 / 1023) as u16;
        curve[12 + index as usize * 2..14 + index as usize * 2]
            .copy_from_slice(&value.to_be_bytes());
    }
    curve
}

fn gray_profile() -> Vec<u8> {
    let curve = sampled_curve();
    let offset = 144usize;
    let mut profile = vec![0; offset + curve.len()];
    profile[8..12].copy_from_slice(&[4, 0, 0, 0]);
    profile[12..16].copy_from_slice(b"mntr");
    profile[16..20].copy_from_slice(b"GRAY");
    profile[20..24].copy_from_slice(b"XYZ ");
    profile[36..40].copy_from_slice(b"acsp");
    put_u32(&mut profile, 64, 1);
    put_u32(&mut profile, 128, 1);
    profile[132..136].copy_from_slice(b"kTRC");
    put_u32(&mut profile, 136, offset as u32);
    put_u32(&mut profile, 140, curve.len() as u32);
    profile[offset..].copy_from_slice(&curve);
    let profile_len = profile.len() as u32;
    put_u32(&mut profile, 0, profile_len);
    profile
}

fn limited(bytes: usize, entries: usize) -> TransformLimits {
    TransformLimits::builder()
        .max_compiled_bytes(bytes)
        .max_curve_entries(entries)
        .build()
        .unwrap()
}

#[test]
fn matrix_compile_rejects_sampled_curve_before_materialization() {
    let profile = Profile::parse(&gray_profile()).unwrap();
    let result = profile.compile(
        TransformDirection::DeviceToPcs,
        RenderingIntent::RelativeColorimetric,
        limited(128, 2),
    );
    assert!(matches!(result, Err(TransformError::ResourceLimit(_))));
}

#[test]
fn matrix_compile_bytes_and_entries_limits_are_independent() {
    let profile = Profile::parse(&gray_profile()).unwrap();
    for (bytes, entries) in [(128, 1 << 20), (65536, 2)] {
        let result = profile.compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::RelativeColorimetric,
            limited(bytes, entries),
        );
        assert!(
            matches!(result, Err(TransformError::ResourceLimit(_))),
            "limits admitted ({bytes}, {entries})"
        );
    }
}

#[test]
fn transform_uses_matrix_plan_for_eager_profiles() {
    let profile = Profile::from_bytes(&gray_profile()).unwrap();
    let result = Transform::new_with_limits(
        &profile,
        &profile,
        TransformOptions::default(),
        limited(128, 2),
    );
    assert!(matches!(result, Err(TransformError::ResourceLimit(_))));
}

#[test]
fn matrix_entry_limit_accepts_exact_count_and_rejects_one_under() {
    let profile = Profile::parse(&gray_profile()).unwrap();
    let accepted = profile.compile(
        TransformDirection::DeviceToPcs,
        RenderingIntent::RelativeColorimetric,
        limited(1 << 20, 1024),
    );
    assert!(accepted.is_ok());
    let rejected = profile.compile(
        TransformDirection::DeviceToPcs,
        RenderingIntent::RelativeColorimetric,
        limited(1 << 20, 1023),
    );
    assert!(matches!(rejected, Err(TransformError::ResourceLimit(_))));
}
