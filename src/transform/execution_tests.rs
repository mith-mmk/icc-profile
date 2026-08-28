use super::compile::Transform;
use super::error::TransformError;
use super::execution::{checked_output_allocation, try_new_f32_output, try_new_f32_output_with};
use super::limits::ExecutionLimits;
use super::profile::{Profile, RenderingIntent, TransformOptions};

fn limits(bytes: usize) -> ExecutionLimits {
    ExecutionLimits::builder()
        .max_output_bytes(bytes)
        .build()
        .unwrap()
}

#[test]
fn scalar_layout_rejects_overflow_and_invalid_lengths_before_allocation() {
    for (input_len, input_channels, output_channels, message) in [
        (usize::MAX, 1, 3, "output length"),
        (usize::MAX / 4 + 1, 1, 1, "transform output bytes"),
        (
            isize::MAX as usize / 4 + 1,
            1,
            1,
            "transform output addressable size",
        ),
    ] {
        let (result, _, requests) = crate::allocation_probe::watch(1, || {
            checked_output_allocation(
                input_len,
                input_channels,
                output_channels,
                limits(usize::MAX),
            )
        });
        assert!(matches!(
            result,
            Err(TransformError::ResourceLimit(actual)) if actual == message
        ));
        assert_eq!(requests, 0);
    }
    let (result, _, requests) =
        crate::allocation_probe::watch(1, || checked_output_allocation(1, 3, 3, limits(0)));
    assert!(matches!(
        result,
        Err(TransformError::InvalidBufferLength {
            expected: 3,
            actual: 1
        })
    ));
    assert_eq!(requests, 0);
}

#[test]
fn overcapacity_candidate_is_dropped_and_same_plan_retries() {
    let plan = checked_output_allocation(2, 1, 1, limits(8)).unwrap();
    let ((result, hits, _), live, peak) = crate::allocation_probe::live_scope(|| {
        crate::allocation_probe::watch(12, || {
            try_new_f32_output_with(&plan, limits(8), || Ok(Vec::with_capacity(3)))
        })
    });
    assert!(matches!(
        result,
        Err(TransformError::ResourceLimit("transform output capacity"))
    ));
    assert_eq!(hits, 1);
    assert_eq!(live, 0);
    assert_eq!(peak, 12);

    let retry = try_new_f32_output(&plan, limits(8)).unwrap();
    assert_eq!(retry.as_slice(), [0.0, 0.0]);
}

#[test]
fn real_reservation_failure_is_typed_and_retryable() {
    let plan = checked_output_allocation(2, 1, 1, limits(8)).unwrap();
    let ((result, hits, _), live, peak) = crate::allocation_probe::live_scope(|| {
        crate::allocation_probe::watch(8, || {
            crate::allocation_probe::fail_once(8, || try_new_f32_output(&plan, limits(8)))
        })
    });
    assert!(matches!(
        result,
        Err(TransformError::ResourceLimit("transform output allocation"))
    ));
    assert_eq!(hits, 1);
    assert_eq!(live, 0);
    assert_eq!(peak, 0);

    let retry = try_new_f32_output(&plan, limits(8)).unwrap();
    assert_eq!(retry.len(), 2);
}

#[test]
fn candidate_must_be_empty_and_have_capacity_before_resize() {
    let plan = checked_output_allocation(2, 1, 1, limits(8)).unwrap();

    let ((insufficient, _, total), live, _) = crate::allocation_probe::live_scope(|| {
        crate::allocation_probe::watch(1, || {
            try_new_f32_output_with(&plan, limits(8), || Ok(Vec::new()))
        })
    });
    assert!(matches!(
        insufficient,
        Err(TransformError::ResourceLimit("transform output candidate"))
    ));
    assert_eq!(total, 0);
    assert_eq!(live, 0);

    let ((nonempty, _, total), live, _) = crate::allocation_probe::live_scope(|| {
        let candidate = vec![0.25, 0.75];
        crate::allocation_probe::watch(1, || {
            try_new_f32_output_with(&plan, limits(8), || Ok(candidate))
        })
    });
    assert!(matches!(
        nonempty,
        Err(TransformError::ResourceLimit("transform output candidate"))
    ));
    assert_eq!(total, 0);
    assert_eq!(live, 0);
}

#[test]
fn output_bounds_cover_exact_under_empty_and_default_cases() {
    let plan = checked_output_allocation(2, 1, 1, limits(8)).unwrap();
    let exact = try_new_f32_output(&plan, limits(8)).unwrap();
    assert_eq!(exact.len(), 2);

    let ((under, _, total), live, _) = crate::allocation_probe::live_scope(|| {
        crate::allocation_probe::watch(8, || try_new_f32_output(&plan, limits(7)))
    });
    assert!(matches!(
        under,
        Err(TransformError::ResourceLimit("transform output limit"))
    ));
    assert_eq!(total, 0);
    assert_eq!(live, 0);

    let empty_plan = checked_output_allocation(0, 1, 1, limits(0)).unwrap();
    let empty = try_new_f32_output(&empty_plan, limits(0)).unwrap();
    assert!(empty.is_empty());
    assert_eq!(
        ExecutionLimits::default().max_output_bytes(),
        64 * 1024 * 1024
    );
}

fn identity_gray_transform() -> Transform {
    let mut bytes = vec![0; 160];
    bytes[8..12].copy_from_slice(&[4, 0, 0, 0]);
    bytes[12..16].copy_from_slice(b"mntr");
    bytes[16..20].copy_from_slice(b"GRAY");
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    bytes[64..68].copy_from_slice(&1u32.to_be_bytes());
    bytes[128..132].copy_from_slice(&1u32.to_be_bytes());
    bytes[132..136].copy_from_slice(b"kTRC");
    bytes[136..140].copy_from_slice(&144u32.to_be_bytes());
    bytes[140..144].copy_from_slice(&16u32.to_be_bytes());
    bytes[144..148].copy_from_slice(b"curv");
    bytes[152..156].copy_from_slice(&1u32.to_be_bytes());
    bytes[156..158].copy_from_slice(&256u16.to_be_bytes());
    let profile_len = bytes.len() as u32;
    bytes[0..4].copy_from_slice(&profile_len.to_be_bytes());
    let profile = Profile::parse(&bytes).unwrap();
    Transform::new(
        &profile,
        &profile,
        TransformOptions {
            rendering_intent: RenderingIntent::RelativeColorimetric,
            ..TransformOptions::default()
        },
    )
    .unwrap()
}

#[test]
fn worker_cold_and_warm_execution_use_no_image_sized_allocations() {
    let transform = identity_gray_transform();
    let input = [0.25, 0.75];
    let mut output = [0.0; 2];
    let (mut worker, _, creation_total) = crate::allocation_probe::watch(1, || transform.worker());
    assert_eq!(creation_total, 0);

    let (cold_result, _, cold_total) =
        crate::allocation_probe::watch(1, || worker.transform_f32(&input, &mut output));
    cold_result.unwrap();
    assert_eq!(cold_total, 0);
    assert_eq!(output, input);
    let (warm_result, _, warm_total) =
        crate::allocation_probe::watch(1, || worker.transform_f32(&input, &mut output));
    warm_result.unwrap();
    assert_eq!(warm_total, 0);
    assert_eq!(output, input);
}
