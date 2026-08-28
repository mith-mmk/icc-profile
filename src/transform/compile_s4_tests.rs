use super::super::compile_budget::CompileBudget;
use super::super::error::TransformError;
use super::super::limits::TransformLimits;
use super::super::lut::LutTransform;
use super::super::profile::MatrixProfile;
use super::super::profile::{Pcs, Profile, RenderingIntent};
use super::super::route_plan::{admit_pair, plan_route_with_policy, OwnerPolicy};
use super::{materialize_admitted_pair, Transform, TransformDirection};
use std::cell::Cell;

use crate::allocation_probe;

const RELATIVE: RenderingIntent = RenderingIntent::RelativeColorimetric;

fn plan_route(
    profile: &Profile,
    direction: TransformDirection,
    intent: RenderingIntent,
    limits: TransformLimits,
) -> Result<super::super::route_plan::RoutePlan<'_>, TransformError> {
    plan_route_with_policy(
        profile,
        direction,
        intent,
        limits,
        OwnerPolicy::TransformStage,
    )
}

#[derive(Clone, Copy)]
enum PairKind {
    MatrixMatrix,
    MatrixLut,
    LutMatrix,
    LutLut,
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn raw_profile(space: &[u8; 4], tags: Vec<(&[u8; 4], Vec<u8>)>) -> Vec<u8> {
    let mut bytes = vec![0; 132 + 12 * tags.len()];
    bytes[8..12].copy_from_slice(&[4, 0, 0, 0]);
    bytes[12..16].copy_from_slice(b"mntr");
    bytes[16..20].copy_from_slice(space);
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    put_u32(&mut bytes, 64, 1);
    put_u32(&mut bytes, 128, tags.len() as u32);
    for (index, (signature, data)) in tags.into_iter().enumerate() {
        let offset = (bytes.len() + 3) & !3;
        bytes.resize(offset + data.len(), 0);
        bytes[offset..].copy_from_slice(&data);
        bytes[132 + index * 12..136 + index * 12].copy_from_slice(signature);
        put_u32(&mut bytes, 136 + index * 12, offset as u32);
        put_u32(&mut bytes, 140 + index * 12, data.len() as u32);
    }
    let length = bytes.len() as u32;
    put_u32(&mut bytes, 0, length);
    bytes
}

fn sampled_curve() -> Vec<u8> {
    sampled_curve_with_entries(2)
}

fn sampled_curve_with_entries(entries: usize) -> Vec<u8> {
    let mut curve = vec![0; 12 + entries * 2];
    curve[..4].copy_from_slice(b"curv");
    put_u32(&mut curve, 8, entries as u32);
    for index in 0..entries {
        let value = (u32::from(u16::MAX) * index as u32 / (entries - 1) as u32) as u16;
        curve[12 + index * 2..14 + index * 2].copy_from_slice(&value.to_be_bytes());
    }
    curve
}

fn gray_matrix_profile(curve_entries: usize) -> Profile {
    Profile::parse(&raw_profile(
        b"GRAY",
        vec![(b"kTRC", sampled_curve_with_entries(curve_entries))],
    ))
    .unwrap()
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

fn matrix_profile(gray: bool) -> Profile {
    let curve = sampled_curve();
    let tags = if gray {
        vec![(b"kTRC", curve)]
    } else {
        vec![
            (b"rXYZ", xyz_tag([0.9642, 0.0, 0.0])),
            (b"gXYZ", xyz_tag([0.0, 1.0, 0.0])),
            (b"bXYZ", xyz_tag([0.0, 0.0, 0.8249])),
            (b"rTRC", curve.clone()),
            (b"gTRC", curve.clone()),
            (b"bTRC", curve),
        ]
    };
    Profile::parse(&raw_profile(if gray { b"GRAY" } else { b"RGB " }, tags)).unwrap()
}

fn mft2() -> Vec<u8> {
    let mut tag = vec![0; 52 + (3 * 2 + 8 * 3 + 3 * 2) * 2];
    tag[..4].copy_from_slice(b"mft2");
    tag[8] = 3;
    tag[9] = 3;
    tag[10] = 2;
    for index in 0..9 {
        put_u32(
            &mut tag,
            12 + index * 4,
            if index % 4 == 0 { 0x0001_0000 } else { 0 },
        );
    }
    tag[48..50].copy_from_slice(&2u16.to_be_bytes());
    tag[50..52].copy_from_slice(&2u16.to_be_bytes());
    let mut offset = 52;
    for _ in 0..3 {
        tag[offset..offset + 2].copy_from_slice(&0u16.to_be_bytes());
        tag[offset + 2..offset + 4].copy_from_slice(&u16::MAX.to_be_bytes());
        offset += 4;
    }
    for _ in 0..24 {
        tag[offset..offset + 2].copy_from_slice(&0u16.to_be_bytes());
        offset += 2;
    }
    for _ in 0..3 {
        tag[offset..offset + 2].copy_from_slice(&0u16.to_be_bytes());
        tag[offset + 2..offset + 4].copy_from_slice(&u16::MAX.to_be_bytes());
        offset += 4;
    }
    tag
}

fn para4_curve() -> Vec<u8> {
    let mut curve = vec![0; 40];
    curve[..4].copy_from_slice(b"para");
    curve[8..10].copy_from_slice(&4u16.to_be_bytes());
    for (index, value) in [1.0_f32, 1.0, 0.0, 0.0, 0.5, 0.0, 1.0]
        .into_iter()
        .enumerate()
    {
        let fixed = (value * 65536.0).round() as i32;
        curve[12 + index * 4..16 + index * 4].copy_from_slice(&fixed.to_be_bytes());
    }
    curve
}

fn identity_matrix() -> Vec<u8> {
    let mut matrix = vec![0; 48];
    for index in 0..3 {
        put_u32(&mut matrix, index * 16, 0x0001_0000);
    }
    matrix
}

fn clut_2x2x2() -> Vec<u8> {
    let mut clut = vec![0; 20 + 8 * 3 * 2];
    clut[..3].copy_from_slice(&[2, 2, 2]);
    clut[16] = 2;
    clut
}

fn all_stage_lut(signature: [u8; 4], gap: usize) -> Vec<u8> {
    let curve = para4_curve();
    let mut tag = vec![0; 32 + gap];
    tag[..4].copy_from_slice(&signature);
    tag[8] = 3;
    tag[9] = 3;
    let append = |tag: &mut Vec<u8>, bytes: &[u8]| -> u32 {
        let offset = tag.len();
        tag.extend_from_slice(bytes);
        offset as u32
    };
    let mut curves = Vec::with_capacity(3 * curve.len());
    for _ in 0..3 {
        curves.extend_from_slice(&curve);
    }
    let b = append(&mut tag, &curves);
    let matrix = append(&mut tag, &identity_matrix());
    let m = append(&mut tag, &curves);
    let clut = append(&mut tag, &clut_2x2x2());
    let a = append(&mut tag, &curves);
    put_u32(&mut tag, 12, b);
    put_u32(&mut tag, 16, matrix);
    put_u32(&mut tag, 20, m);
    put_u32(&mut tag, 24, clut);
    put_u32(&mut tag, 28, a);
    tag
}

fn lut_profile(signature: &[u8; 4]) -> Profile {
    Profile::parse(&raw_profile(b"RGB ", vec![(signature, mft2())])).unwrap()
}

fn pair_profiles(kind: PairKind) -> (Profile, Profile) {
    match kind {
        PairKind::MatrixMatrix => (matrix_profile(true), matrix_profile(true)),
        PairKind::MatrixLut => (matrix_profile(true), lut_profile(b"B2A1")),
        PairKind::LutMatrix => (lut_profile(b"A2B1"), matrix_profile(true)),
        PairKind::LutLut => (lut_profile(b"A2B1"), lut_profile(b"B2A1")),
    }
}

fn limits(bytes: usize, curves: usize, clut: usize) -> TransformLimits {
    TransformLimits::builder()
        .max_compiled_bytes(bytes)
        .max_curve_entries(curves)
        .max_clut_entries(clut)
        .build()
        .unwrap()
}

fn pair_cost(kind: PairKind) -> (usize, usize, usize) {
    let (input, output) = pair_profiles(kind);
    let planning_limits = limits(usize::MAX, 1 << 20, 1 << 24);
    let input_plan = plan_route(
        &input,
        TransformDirection::DeviceToPcs,
        RELATIVE,
        planning_limits,
    )
    .unwrap();
    let output_plan = plan_route(
        &output,
        TransformDirection::PcsToDevice,
        RELATIVE,
        planning_limits,
    )
    .unwrap();
    let input = input_plan.inventory().unwrap();
    let output = output_plan.inventory().unwrap();
    let expected_input = fixed_route_cost(!matches!(
        kind,
        PairKind::MatrixLut | PairKind::MatrixMatrix
    ));
    let expected_output = fixed_route_cost(!matches!(
        kind,
        PairKind::MatrixMatrix | PairKind::LutMatrix
    ));
    assert_eq!(input, expected_input);
    assert_eq!(output, expected_output);
    (
        expected_input.0.checked_add(expected_output.0).unwrap(),
        expected_input.1.checked_add(expected_output.1).unwrap(),
        expected_input.2.checked_add(expected_output.2).unwrap(),
    )
}

fn fixed_route_cost(lut: bool) -> (usize, usize, usize) {
    if lut {
        let decoded_samples = (3 * 2 + 8 * 3 + 3 * 2) * size_of::<f32>();
        let owner = 6 * size_of::<Vec<f32>>() + 3 * size_of::<usize>();
        (size_of::<LutTransform>() + decoded_samples + owner, 12, 24)
    } else {
        (
            size_of::<MatrixProfile>()
                + size_of::<super::super::curve::Curve>()
                + 2 * size_of::<f32>(),
            2,
            0,
        )
    }
}

#[test]
fn two_direction_admission_has_exact_and_one_under_boundaries_for_all_pairs() {
    for kind in [
        PairKind::MatrixMatrix,
        PairKind::MatrixLut,
        PairKind::LutMatrix,
        PairKind::LutLut,
    ] {
        let (bytes, curves, clut) = pair_cost(kind);
        let (input, output) = pair_profiles(kind);
        let exact_limits = limits(bytes, curves, clut.max(1));
        let input_plan = plan_route(
            &input,
            TransformDirection::DeviceToPcs,
            RELATIVE,
            exact_limits,
        )
        .unwrap();
        let output_plan = plan_route(
            &output,
            TransformDirection::PcsToDevice,
            RELATIVE,
            exact_limits,
        )
        .unwrap();
        let mut exact_budget = CompileBudget::new(exact_limits);
        assert!(admit_pair(&input_plan, &output_plan, &mut exact_budget).is_ok());
        assert_eq!(exact_budget.test_state().2, bytes);
        let transform = Transform::new_with_limits(
            &input,
            &output,
            super::super::profile::TransformOptions::default(),
            exact_limits,
        )
        .unwrap();
        assert!(transform.input_channels() > 0 && transform.output_channels() > 0);

        let under_limits = limits(bytes - 1, curves, clut.max(1));
        let input_plan = plan_route(
            &input,
            TransformDirection::DeviceToPcs,
            RELATIVE,
            under_limits,
        )
        .unwrap();
        let output_plan = plan_route(
            &output,
            TransformDirection::PcsToDevice,
            RELATIVE,
            under_limits,
        )
        .unwrap();
        let mut under_budget = CompileBudget::new(under_limits);
        let checkpoint = under_budget.checkpoint();
        assert!(matches!(
            admit_pair(&input_plan, &output_plan, &mut under_budget),
            Err(TransformError::ResourceLimit(_))
        ));
        assert_eq!(under_budget.checkpoint(), checkpoint);

        if curves > 2 {
            let curve_limited = limits(bytes, curves - 1, clut.max(1));
            let input_plan = plan_route(
                &input,
                TransformDirection::DeviceToPcs,
                RELATIVE,
                curve_limited,
            )
            .unwrap();
            let output_plan = plan_route(
                &output,
                TransformDirection::PcsToDevice,
                RELATIVE,
                curve_limited,
            )
            .unwrap();
            let mut curve_budget = CompileBudget::new(curve_limited);
            assert!(matches!(
                admit_pair(&input_plan, &output_plan, &mut curve_budget),
                Err(TransformError::ResourceLimit(_))
            ));
        }
        if clut > 0 && matches!(kind, PairKind::LutLut) {
            let clut_limited = limits(bytes, curves, clut - 1);
            let input_plan = plan_route(
                &input,
                TransformDirection::DeviceToPcs,
                RELATIVE,
                clut_limited,
            )
            .unwrap();
            let output_plan = plan_route(
                &output,
                TransformDirection::PcsToDevice,
                RELATIVE,
                clut_limited,
            )
            .unwrap();
            let mut clut_budget = CompileBudget::new(clut_limited);
            assert!(matches!(
                admit_pair(&input_plan, &output_plan, &mut clut_budget),
                Err(TransformError::ResourceLimit(_))
            ));
        }
    }
}

#[test]
fn pair_admission_counts_clut_entries_once_and_keeps_matrix_zero() {
    let (bytes, curves, clut) = pair_cost(PairKind::LutLut);
    assert!(bytes > 0 && curves > 0 && clut > 0);
    let (matrix_bytes, matrix_curves, matrix_clut) = pair_cost(PairKind::MatrixMatrix);
    assert!(matrix_bytes > 0 && matrix_curves > 0);
    assert_eq!(matrix_clut, 0);
}

#[test]
fn pair_plans_do_not_depend_on_eager_profile_matrix_cache() {
    let profile = matrix_profile(true);
    let route = plan_route(
        &profile,
        TransformDirection::DeviceToPcs,
        RELATIVE,
        limits(1024, 16, 1),
    );
    assert!(route.is_ok());
    assert_eq!(profile.pcs(), Pcs::Xyz);
}

#[test]
fn pair_pending_candidate_failure_is_atomic_and_retryable() {
    struct Candidate<'a> {
        bytes: Vec<u8>,
        dropped: &'a Cell<bool>,
    }

    impl Drop for Candidate<'_> {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    let (bytes, curves, clut) = pair_cost(PairKind::LutLut);
    let limits = limits(bytes, curves, clut);
    let (input, output) = pair_profiles(PairKind::LutLut);
    let input_plan = plan_route(&input, TransformDirection::DeviceToPcs, RELATIVE, limits).unwrap();
    let output_plan =
        plan_route(&output, TransformDirection::PcsToDevice, RELATIVE, limits).unwrap();
    let mut budget = CompileBudget::new(limits);
    admit_pair(&input_plan, &output_plan, &mut budget).unwrap();
    let checkpoint = budget.checkpoint();
    let dropped = Cell::new(false);
    let result = budget.try_candidate(
        8,
        "temporary pair owner",
        || {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(16)
                .map_err(|_| TransformError::ResourceLimit("temporary pair owner"))?;
            bytes.resize(16, 0xa5);
            Ok(Candidate {
                bytes,
                dropped: &dropped,
            })
        },
        |candidate| Ok(candidate.bytes.capacity()),
    );
    assert!(result.is_err());
    assert!(dropped.get());
    assert_eq!(budget.checkpoint(), checkpoint);
    assert!(Transform::new_with_limits(
        &input,
        &output,
        super::super::profile::TransformOptions::default(),
        limits,
    )
    .is_ok());
}

#[test]
fn same_admitted_pair_drops_partial_output_and_retries_without_readmission() {
    struct Candidate<'a> {
        bytes: Vec<u8>,
        source: *const u8,
        dropped: &'a Cell<usize>,
    }

    impl Drop for Candidate<'_> {
        fn drop(&mut self) {
            self.dropped.set(self.dropped.get() + 1);
        }
    }

    fn make_candidate<'a>(
        budget: &mut CompileBudget,
        source: &'a [u8],
        dropped: &'a Cell<usize>,
        capacity: usize,
    ) -> Result<Candidate<'a>, TransformError> {
        budget.try_candidate(
            8,
            "pair candidate",
            || {
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(capacity)
                    .map_err(|_| TransformError::ResourceLimit("pair candidate"))?;
                bytes.extend_from_slice(source);
                Ok(Candidate {
                    bytes,
                    source: source.as_ptr(),
                    dropped,
                })
            },
            |candidate: &Candidate<'_>| Ok(candidate.bytes.capacity()),
        )
    }

    let limits = limits(16, 16, 1);
    let mut budget = CompileBudget::new(limits);
    budget.admit_storage(16, "admitted pair payload").unwrap();
    let admitted = budget.checkpoint();
    let source = [0x3c_u8; 8];
    let dropped = Cell::new(0);
    let failed = materialize_admitted_pair(
        &mut budget,
        admitted,
        |budget| make_candidate(budget, &source, &dropped, 8),
        |budget| make_candidate(budget, &source, &dropped, 16),
        |_| Ok(()),
        |_| Ok(()),
    );
    assert!(failed.is_err());
    assert_eq!(dropped.get(), 2);
    assert_eq!(budget.checkpoint(), admitted);

    let (input, output) = materialize_admitted_pair(
        &mut budget,
        admitted,
        |budget| make_candidate(budget, &source, &dropped, 8),
        |budget| make_candidate(budget, &source, &dropped, 8),
        |_| Ok(()),
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(input.source, source.as_ptr());
    assert_eq!(output.source, source.as_ptr());
    assert_eq!(input.bytes.as_slice(), source);
    assert_eq!(output.bytes.as_slice(), source);
    assert_eq!(budget.test_state().2, 0);
    assert_eq!(budget.test_state().3, 16);
}

#[test]
fn real_mab_pair_retries_the_same_admission_after_candidate_capacity_rejection() {
    struct Candidate<'a> {
        bytes: Vec<u8>,
        dropped: &'a Cell<bool>,
    }

    impl Drop for Candidate<'_> {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    let input_data = all_stage_lut(*b"mAB ", 0);
    let output_data = all_stage_lut(*b"mBA ", 0);
    let input = Profile::parse(&raw_profile(b"RGB ", vec![(b"A2B1", input_data)])).unwrap();
    let output = Profile::parse(&raw_profile(b"RGB ", vec![(b"B2A1", output_data)])).unwrap();
    let source_input = input.tag(u32::from_be_bytes(*b"A2B1")).unwrap();
    let source_output = output.tag(u32::from_be_bytes(*b"B2A1")).unwrap();
    let input_pointer = source_input.as_ptr();
    let output_pointer = source_output.as_ptr();
    let input_snapshot = source_input.to_vec();
    let output_snapshot = source_output.to_vec();
    let planning_limits = limits(200_000, 4096, 4096);
    let input_plan = plan_route(
        &input,
        TransformDirection::DeviceToPcs,
        RELATIVE,
        planning_limits,
    )
    .unwrap();
    let output_plan = plan_route(
        &output,
        TransformDirection::PcsToDevice,
        RELATIVE,
        planning_limits,
    )
    .unwrap();
    let input_cost = input_plan.inventory().unwrap();
    let output_cost = output_plan.inventory().unwrap();
    let fixed_direction_bytes = size_of::<LutTransform>()
        .checked_add(
            9usize
                .checked_mul(size_of::<super::super::curve::Curve>())
                .unwrap(),
        )
        .and_then(|bytes| bytes.checked_add(9usize.checked_mul(7 * 4).unwrap()))
        .and_then(|bytes| bytes.checked_add(3usize.checked_mul(size_of::<usize>()).unwrap()))
        .and_then(|bytes| bytes.checked_add(24usize.checked_mul(4).unwrap()))
        .unwrap();
    let fixed_pair_bytes = 2usize.checked_mul(fixed_direction_bytes).unwrap();
    assert_eq!(
        input_cost.0.checked_add(output_cost.0).unwrap(),
        fixed_pair_bytes,
        "route inventory must match the fixed LUT owner formula"
    );
    let pair_bytes = fixed_pair_bytes;
    let pair_curves = input_cost.1.checked_add(output_cost.1).unwrap();
    let pair_clut = input_cost.2.checked_add(output_cost.2).unwrap();
    let exact_limits = limits(pair_bytes, pair_curves, pair_clut);
    let input_plan = plan_route(
        &input,
        TransformDirection::DeviceToPcs,
        RELATIVE,
        exact_limits,
    )
    .unwrap();
    let output_plan = plan_route(
        &output,
        TransformDirection::PcsToDevice,
        RELATIVE,
        exact_limits,
    )
    .unwrap();
    let mut budget = CompileBudget::new(exact_limits);
    admit_pair(&input_plan, &output_plan, &mut budget).unwrap();
    let checkpoint = budget.checkpoint();
    let dropped = Cell::new(false);
    let (candidate, live, peak) = allocation_probe::live_scope(|| {
        budget.try_candidate(
            28,
            "MAB parameter owner",
            || {
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(32)
                    .map_err(|_| TransformError::ResourceLimit("MAB parameter owner"))?;
                bytes.resize(32, 0xa5);
                Ok(Candidate {
                    bytes,
                    dropped: &dropped,
                })
            },
            |candidate| Ok(candidate.bytes.capacity()),
        )
    });
    assert!(matches!(
        candidate,
        Err(TransformError::ResourceLimit("compiled transform bytes"))
    ));
    assert!(dropped.get());
    assert_eq!(live, 0, "failed candidate storage must be deallocated");
    assert_eq!(peak, 32, "candidate must allocate its actual 32-byte owner");
    assert_eq!(budget.checkpoint(), checkpoint);
    let pair = super::materialize_pair(
        input_plan,
        output_plan,
        &mut budget,
        input.limits(),
        output.limits(),
    )
    .unwrap();
    assert_eq!(budget.test_state().2, 0);
    assert_eq!(budget.test_state().3, pair_bytes);
    assert_eq!(pair.input.input_channels, 3);
    assert_eq!(pair.output.output_channels, 3);
    assert_eq!(
        input.tag(u32::from_be_bytes(*b"A2B1")).unwrap().as_ptr(),
        input_pointer
    );
    assert_eq!(
        output.tag(u32::from_be_bytes(*b"B2A1")).unwrap().as_ptr(),
        output_pointer
    );
    assert_eq!(
        input.tag(u32::from_be_bytes(*b"A2B1")).unwrap(),
        input_snapshot
    );
    assert_eq!(
        output.tag(u32::from_be_bytes(*b"B2A1")).unwrap(),
        output_snapshot
    );
}

#[test]
fn destination_allocation_failure_drops_completed_input_and_restores_admission() {
    let input = gray_matrix_profile(2);
    let output = gray_matrix_profile(3);
    let tag = u32::from_be_bytes(*b"kTRC");
    let input_tag = input.tag(tag).unwrap();
    let output_tag = output.tag(tag).unwrap();
    let input_pointer = input_tag.as_ptr();
    let output_pointer = output_tag.as_ptr();
    let input_snapshot = input_tag.to_vec();
    let output_snapshot = output_tag.to_vec();
    let limits = limits(4096, 16, 1);

    let ((result, live, peak), input_hits, _) = allocation_probe::watch(8, || {
        allocation_probe::live_scope(|| {
            allocation_probe::fail_once(12, || {
                Transform::new_with_limits(
                    &input,
                    &output,
                    super::super::profile::TransformOptions::default(),
                    limits,
                )
            })
        })
    });

    assert!(matches!(result, Err(TransformError::ResourceLimit(_))));
    assert_eq!(input_hits, 1, "the 8-byte input curve owner must be built");
    assert_eq!(
        allocation_probe::failure_hits(),
        1,
        "the 12-byte output curve owner allocation must be rejected"
    );
    assert!(
        peak > 12,
        "the real input owner must exist before output failure"
    );
    assert_eq!(
        live, 0,
        "all completed input and partial output owners are dropped"
    );
    assert_eq!(input.tag(tag).unwrap().as_ptr(), input_pointer);
    assert_eq!(output.tag(tag).unwrap().as_ptr(), output_pointer);
    assert_eq!(input.tag(tag).unwrap(), input_snapshot);
    assert_eq!(output.tag(tag).unwrap(), output_snapshot);

    let retry = Transform::new_with_limits(
        &input,
        &output,
        super::super::profile::TransformOptions::default(),
        limits,
    );
    assert!(retry.is_ok(), "the same profiles must retry after rollback");
}
