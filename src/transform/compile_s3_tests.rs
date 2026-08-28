use std::mem::size_of;

use super::super::compile_budget::CompileBudget;
use super::super::error::TransformError;
use super::super::limits::{ParseLimits, TransformLimits};
use super::super::lut::LutTransform;
use super::super::lut_plan::plan_lut;
use super::super::profile::{Pcs, Profile, RenderingIntent};
use super::{CompiledDirection, TransformDirection};

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
    put_u32(&mut bytes, 132, u32::from_be_bytes(signature));
    put_u32(&mut bytes, 136, offset as u32);
    put_u32(&mut bytes, 140, tag.len() as u32);
    let length = bytes.len() as u32;
    put_u32(&mut bytes, 0, length);
    bytes
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

fn lut_limits(max_compiled_bytes: usize) -> TransformLimits {
    TransformLimits::builder()
        .max_compiled_bytes(max_compiled_bytes)
        .max_curve_entries(4096)
        .max_clut_entries(4096)
        .build()
        .expect("valid transform limits")
}

fn all_stage_owned_bytes() -> usize {
    9 * std::mem::size_of::<super::super::curve::Curve>()
        + 9 * 7 * size_of::<f32>()
        + 3 * size_of::<usize>()
        + 24 * size_of::<f32>()
        + size_of::<LutTransform>()
        + size_of::<CompiledDirection>()
}

#[test]
fn private_compatibility_lut_entrypoints_remain_available_to_harnesses() {
    let data = all_stage_lut(*b"mAB ", 0);
    assert!(
        LutTransform::parse(&data, ParseLimits::default(), Pcs::Xyz, true).is_ok(),
        "test-only compatibility parser must remain callable"
    );
    assert!(
        super::super::lut_plan::check_encoded_limits(&data, TransformLimits::default()).is_ok(),
        "test-only encoded-limit wrapper must remain callable"
    );
    let plan = plan_lut(
        &data,
        (3, 3),
        Pcs::Xyz,
        true,
        TransformLimits::default(),
        ParseLimits::default(),
    )
    .expect("compatibility plan");
    assert!(
        plan.materialize(ParseLimits::default()).is_ok(),
        "test-only materialization wrapper must remain callable"
    );
}

#[test]
fn encoded_gap_is_not_counted_as_compiled_storage_in_either_lut_direction() {
    let expected_owned = all_stage_owned_bytes();
    for (tag_name, direction, tag_signature) in [
        (*b"A2B0", TransformDirection::DeviceToPcs, *b"mAB "),
        (*b"B2A0", TransformDirection::PcsToDevice, *b"mBA "),
    ] {
        let tag = all_stage_lut(tag_signature, 8192);
        assert!(tag.len() > expected_owned);
        let profile = Profile::parse(&profile_with_tag(tag_name, &tag)).expect("profile parse");
        let exact = profile.compile(
            direction,
            RenderingIntent::Perceptual,
            lut_limits(expected_owned),
        );
        assert!(exact.is_ok(), "{direction:?} gap LUT rejected: {exact:?}");
        let under = profile.compile(
            direction,
            RenderingIntent::Perceptual,
            lut_limits(expected_owned - 1),
        );
        assert!(
            matches!(under, Err(TransformError::ResourceLimit(_))),
            "{direction:?} one-under unexpectedly accepted: {under:?}"
        );
    }
}

fn mft2_grid16() -> Vec<u8> {
    let mut tag = vec![0; 52 + (3 * 2 + 16usize.pow(3) * 3 + 3 * 2) * 2];
    tag[..4].copy_from_slice(b"mft2");
    tag[8] = 3;
    tag[9] = 3;
    tag[10] = 16;
    for index in 0..9 {
        put_u32(
            &mut tag,
            12 + index * 4,
            if index % 4 == 0 { 0x0001_0000 } else { 0 },
        );
    }
    tag[48..50].copy_from_slice(&2u16.to_be_bytes());
    tag[50..52].copy_from_slice(&2u16.to_be_bytes());
    let mut at = 52;
    for _ in 0..3 {
        tag[at..at + 2].copy_from_slice(&0u16.to_be_bytes());
        tag[at + 2..at + 4].copy_from_slice(&u16::MAX.to_be_bytes());
        at += 4;
    }
    for _ in 0..(16usize.pow(3) * 3) {
        tag[at..at + 2].copy_from_slice(&0u16.to_be_bytes());
        at += 2;
    }
    for _ in 0..3 {
        tag[at..at + 2].copy_from_slice(&0u16.to_be_bytes());
        tag[at + 2..at + 4].copy_from_slice(&u16::MAX.to_be_bytes());
        at += 4;
    }
    tag
}

#[test]
fn lut_admission_includes_private_headers_and_retry_keeps_pending_state() {
    use std::cell::Cell;

    struct Candidate<'a> {
        bytes: Vec<u8>,
        dropped: &'a Cell<bool>,
    }

    impl Drop for Candidate<'_> {
        fn drop(&mut self) {
            assert!(self.bytes.iter().all(|value| *value == 0xa5));
            self.dropped.set(true);
        }
    }

    let data = all_stage_lut(*b"mAB ", 0);
    let source_pointer = data.as_ptr();
    let exact = all_stage_owned_bytes();
    let limits = lut_limits(exact + 8);
    let exact_plan = plan_lut(
        &data,
        (3, 3),
        super::super::profile::Pcs::Xyz,
        true,
        limits,
        ParseLimits::default(),
    )
    .expect("exact plan");

    let mut budget = CompileBudget::new(limits);
    budget
        .admit_storage(8, "preexisting LUT owner")
        .expect("preexisting admission");
    let mut old = budget
        .try_new_vec::<u8>(8, 8, "preexisting LUT owner")
        .expect("preexisting owner");
    old.extend_from_slice(&[0x33; 8]);
    let old_pointer = old.as_ptr();
    exact_plan
        .admit(
            &mut budget,
            size_of::<LutTransform>()
                .checked_add(size_of::<CompiledDirection>())
                .expect("header size"),
        )
        .expect("exact admission");
    let checkpoint = budget.checkpoint();
    let dropped = Cell::new(false);
    let candidate = budget.try_candidate(
        28,
        "LUT parameter owner",
        || {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(32)
                .map_err(|_| TransformError::ResourceLimit("LUT parameter owner"))?;
            bytes.resize(32, 0xa5);
            Ok(Candidate {
                bytes,
                dropped: &dropped,
            })
        },
        |value| Ok(value.bytes.capacity()),
    );
    assert!(candidate.is_err(), "over-capacity candidate was accepted");
    assert!(dropped.get(), "rejected candidate was not dropped");
    assert_eq!(budget.checkpoint(), checkpoint);
    assert_eq!(old.as_ptr(), old_pointer);
    assert_eq!(old, [0x33; 8]);
    assert_eq!(data.as_ptr(), source_pointer);
    let materialized = exact_plan.materialize_with_budget(
        &mut budget,
        ParseLimits::default(),
        size_of::<LutTransform>()
            .checked_add(size_of::<CompiledDirection>())
            .expect("header size"),
    );
    assert!(
        materialized.is_ok(),
        "retry after candidate rollback failed: {materialized:?}"
    );
}

#[test]
fn private_header_sizes_are_part_of_the_fixed_owner_boundary() {
    let headers = size_of::<LutTransform>()
        .checked_add(size_of::<CompiledDirection>())
        .expect("header size");
    let expected = 49_200usize
        .checked_add(6 * size_of::<Vec<f32>>())
        .and_then(|bytes| bytes.checked_add(3 * size_of::<usize>()))
        .and_then(|bytes| bytes.checked_add(headers))
        .expect("owner size");
    assert!(expected > headers);

    let profile =
        Profile::parse(&profile_with_tag(*b"A2B0", &mft2_grid16())).expect("mft2 profile");
    let exact_limits = TransformLimits::builder()
        .max_compiled_bytes(expected)
        .max_curve_entries(4096)
        .max_clut_entries(20_000)
        .build()
        .expect("valid exact limits");
    assert!(profile
        .compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::Perceptual,
            exact_limits,
        )
        .is_ok());
    let under_limits = TransformLimits::builder()
        .max_compiled_bytes(expected - 1)
        .max_curve_entries(4096)
        .max_clut_entries(20_000)
        .build()
        .expect("valid one-under limits");
    assert!(matches!(
        profile.compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::Perceptual,
            under_limits,
        ),
        Err(TransformError::ResourceLimit(_))
    ));
}
