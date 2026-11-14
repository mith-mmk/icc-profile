//! Repository-owned matrix BPC fixtures and independent scalar oracles.
use icc_profile::{
    AlgorithmProfile, ClutInterpolation3D, Profile, RenderingIntent, Transform, TransformLimits,
    TransformOptions,
};
const WHITE: [f64; 3] = [63190.0 / 65536.0, 1.0, 54061.0 / 65536.0];
const PCS_WHITE: [f64; 3] = [0.9642, 1.0, 0.8249];
fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_be_bytes());
}
fn xyz(v: [f64; 3]) -> Vec<u8> {
    let mut b = vec![0; 20];
    b[..4].copy_from_slice(b"XYZ ");
    for (i, value) in v.iter().enumerate() {
        put(&mut b, 8 + i * 4, (value * 65536.0).round() as i32 as u32);
    }
    b
}
fn curve(v: &[u16]) -> Vec<u8> {
    let mut b = vec![0; 12];
    b[..4].copy_from_slice(b"curv");
    put(&mut b, 8, v.len() as u32);
    for x in v {
        b.extend_from_slice(&x.to_be_bytes());
    }
    b
}
fn fixture(version: u8, values: &[u16], extra: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
    let mut tags = vec![];
    for (i, (color, trc)) in [
        (*b"rXYZ", *b"rTRC"),
        (*b"gXYZ", *b"gTRC"),
        (*b"bXYZ", *b"bTRC"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut col = [0.0; 3];
        col[i] = WHITE[i];
        tags.push((color, xyz(col)));
        tags.push((trc, curve(values)));
    }
    tags.extend(extra);
    let mut b = vec![0; 132 + 12 * tags.len()];
    b[8] = version;
    b[12..16].copy_from_slice(b"mntr");
    b[16..20].copy_from_slice(b"RGB ");
    b[20..24].copy_from_slice(b"XYZ ");
    b[36..40].copy_from_slice(b"acsp");
    put(&mut b, 68, 63190);
    put(&mut b, 72, 65536);
    put(&mut b, 76, 54061);
    put(&mut b, 128, tags.len() as u32);
    for (i, (sig, data)) in tags.into_iter().enumerate() {
        while !b.len().is_multiple_of(4) {
            b.push(0);
        }
        let at = b.len();
        put(&mut b, 132 + i * 12, u32::from_be_bytes(sig));
        put(&mut b, 136 + i * 12, at as u32);
        put(&mut b, 140 + i * 12, data.len() as u32);
        b.extend(data);
    }
    let len = b.len();
    put(&mut b, 0, len as u32);
    b
}
fn options(clamp: bool) -> TransformOptions {
    TransformOptions {
        black_point_compensation: true,
        clamp,
        ..Default::default()
    }
}
fn make(src: &[u16], dst: &[u16], clamp: bool) -> Transform {
    Transform::new(
        &Profile::parse(&fixture(4, src, vec![])).unwrap(),
        &Profile::parse(&fixture(4, dst, vec![])).unwrap(),
        options(clamp),
    )
    .unwrap()
}
fn lstar(y: f64) -> f64 {
    if y > 216.0 / 24389.0 {
        116.0 * y.cbrt() - 16.0
    } else {
        24389.0 / 27.0 * y
    }
}
fn luminance(l: f64) -> f64 {
    if l > 8.0 {
        ((l + 16.0) / 116.0).powi(3)
    } else {
        l * 27.0 / 24389.0
    }
}
fn coefficients(src: &[u16], dst: &[u16]) -> (f64, [f64; 3]) {
    let black = |v: &[u16]| {
        luminance(lstar(*v.first().unwrap().min(v.last().unwrap()) as f64 / 65535.0).min(50.0))
    };
    let s = (1.0 - black(dst)) / (1.0 - black(src));
    (s, PCS_WHITE.map(|v| (1.0 - s) * v))
}
fn oracle(input: [f32; 3], src: [u16; 2], dst: [u16; 2], clamp: bool) -> [f32; 3] {
    let (s, o) = coefficients(&src, &dst);
    std::array::from_fn(|i| {
        let x = if clamp {
            input[i].clamp(0.0, 1.0)
        } else {
            input[i]
        };
        let linear = (src[0] as f64 + (src[1] as f64 - src[0] as f64) * x as f64) / 65535.0;
        let value = ((s * WHITE[i] * linear + o[i]) / WHITE[i] * 65535.0 - dst[0] as f64)
            / (dst[1] as f64 - dst[0] as f64);
        (if clamp { value.clamp(0.0, 1.0) } else { value }) as f32
    })
}
fn close(a: &[f32], b: &[f32]) {
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 5e-6, "{a:?} != {b:?}");
    }
}
#[test]
fn different_blacks_compress_expand_and_reverse_with_independent_oracle() {
    for source_version in [2, 4] {
        for destination_version in [2, 4] {
            for (src, dst) in [
                ([0, 65535], [6000, 65535]),
                ([6000, 65535], [0, 65535]),
                ([1000, 65535], [6000, 65535]),
                ([6000, 65535], [1000, 65535]),
                ([65535, 1000], [6000, 65535]),
                ([1000, 65535], [65535, 6000]),
                ([30000, 65535], [6000, 65535]),
                ([30000, 65535], [40000, 65535]),
            ] {
                let source = Profile::parse(&fixture(source_version, &src, vec![])).unwrap();
                let destination =
                    Profile::parse(&fixture(destination_version, &dst, vec![])).unwrap();
                let t = Transform::new(&source, &destination, options(true)).unwrap();
                for input in [[0.0; 3], [0.01, 0.02, 0.03], [0.2, 0.4, 0.7], [1.0; 3]] {
                    let mut out = [0.0; 3];
                    t.transform_f32_atomic(&input, &mut out).unwrap();
                    close(&out, &oracle(input, src, dst, true));
                }
            }
        }
    }
    let t = make(&[3000, 65535], &[3000, 65535], false);
    let input = [0.2, 0.4, 0.7];
    let mut output = [0.0; 3];
    t.transform_f32_atomic(&input, &mut output).unwrap();
    close(&output, &input);
}
#[test]
fn matrix_bpc_strict_workers_algorithms_and_atomic_boundaries() {
    let src = [1000, 65535];
    let dst = [6000, 65535];
    let t = make(&src, &dst, false);
    let clone = t.clone();
    let mut old_worker = t.worker();
    let tri = t.with_algorithm_profile(
        AlgorithmProfile::default().with_clut_3d(ClutInterpolation3D::Trilinear),
    );
    for n in [0, 1, 47, 48, 49, 63, 64, 65, 95, 96, 97, 129] {
        let input: Vec<_> = (0..n * 3).map(|i| [0.2, 0.4, 0.7][i % 3]).collect();
        let mut output = vec![0.0; n * 3];
        let mut other = output.clone();
        old_worker
            .transform_f32_atomic(&input, &mut output)
            .unwrap();
        tri.worker()
            .transform_f32_atomic(&input, &mut other)
            .unwrap();
        assert_eq!(output, other);
        clone.transform_f32_atomic(&input, &mut other).unwrap();
        assert_eq!(output, other);
        for p in output.as_chunks::<3>().0.iter() {
            close(p, &oracle([0.2, 0.4, 0.7], src, dst, false));
        }
        for max in [255.0f32, 65535.0] {
            let codes: Vec<_> = input.iter().map(|v| (v * max).round() as u16).collect();
            let mut out = vec![0u16; n * 3];
            if max == 255.0 {
                let mut small = vec![0; n * 3];
                tri.worker()
                    .transform_u8_atomic(
                        &codes.iter().map(|v| *v as u8).collect::<Vec<_>>(),
                        &mut small,
                    )
                    .unwrap();
                for (a, b) in out.iter_mut().zip(small) {
                    *a = b as u16;
                }
            } else {
                tri.worker().transform_u16_atomic(&codes, &mut out).unwrap();
            }
            if n > 0 {
                let expected = oracle(
                    [codes[0], codes[1], codes[2]].map(|v| v as f32 / max),
                    src,
                    dst,
                    false,
                )
                .map(|v| (v * max).round() as i32);
                for p in out.as_chunks::<3>().0.iter() {
                    for i in 0..3 {
                        assert!((p[i] as i32 - expected[i]).abs() <= 1);
                    }
                }
            }
        }
    }
    for bad in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        let mut input = vec![0.5; 129 * 3];
        *input.last_mut().unwrap() = bad;
        let mut out = vec![-0.0f32; 129 * 3];
        assert!(tri.worker().transform_f32_atomic(&input, &mut out).is_err());
        assert!(out.iter().all(|v| v.to_bits() == (-0.0f32).to_bits()));
    }
    for len in [2, 4] {
        let mut out = vec![-0.0f32; len];
        assert!(tri
            .transform_f32_atomic(&[0.2, 0.4, 0.7], &mut out)
            .is_err());
        assert!(out.iter().all(|v| v.to_bits() == (-0.0f32).to_bits()));
    }
}
#[test]
fn matrix_bpc_budget_and_selected_forward_route_fail_closed() {
    let s = Profile::parse(&fixture(4, &[1000, 65535], vec![])).unwrap();
    let d = Profile::parse(&fixture(4, &[6000, 65535], vec![])).unwrap();
    let on = Transform::new(&s, &d, options(false)).unwrap();
    let off = Transform::new(
        &s,
        &d,
        TransformOptions {
            clamp: false,
            ..Default::default()
        },
    )
    .unwrap();
    let charge = on.memory_usage().unwrap().compiled_bytes();
    assert!(charge > off.memory_usage().unwrap().compiled_bytes());
    Transform::new_with_limits(
        &s,
        &d,
        options(false),
        TransformLimits::builder()
            .max_compiled_bytes(charge)
            .build()
            .unwrap(),
    )
    .unwrap();
    assert!(Transform::new_with_limits(
        &s,
        &d,
        options(false),
        TransformLimits::builder()
            .max_compiled_bytes(charge - 1)
            .build()
            .unwrap()
    )
    .is_err());
    let off_charge = off.memory_usage().unwrap().compiled_bytes();
    Transform::new_with_limits(
        &s,
        &d,
        TransformOptions {
            clamp: false,
            ..Default::default()
        },
        TransformLimits::builder()
            .max_compiled_bytes(off_charge)
            .build()
            .unwrap(),
    )
    .unwrap();
    let mut lut = vec![0; 68];
    lut[..4].copy_from_slice(b"mAB ");
    lut[8] = 3;
    lut[9] = 3;
    put(&mut lut, 12, 32);
    for at in [32, 44, 56] {
        lut[at..at + 4].copy_from_slice(b"curv");
    }
    for sig in [*b"A2B0", *b"A2B1"] {
        for tag in [lut.clone(), b"bad!".to_vec()] {
            let dest = Profile::parse(&fixture(4, &[6000, 65535], vec![(sig, tag)])).unwrap();
            assert!(Transform::new(&s, &dest, options(false)).is_err());
        }
    }
    for intent in [
        RenderingIntent::Perceptual,
        RenderingIntent::Saturation,
        RenderingIntent::AbsoluteColorimetric,
    ] {
        assert!(Transform::new(
            &s,
            &d,
            TransformOptions {
                rendering_intent: intent,
                ..options(false)
            }
        )
        .is_err());
    }
    for side in [false, true] {
        for (at, value) in [(68, 63189), (72, 65535), (76, 54062)] {
            let mut bad = fixture(4, &[1000, 65535], vec![]);
            put(&mut bad, at, value);
            let bad = Profile::parse(&bad).unwrap();
            assert!(if side {
                Transform::new(&bad, &d, options(false))
            } else {
                Transform::new(&s, &bad, options(false))
            }
            .is_err());
        }
        for space in [*b"GRAY", *b"CMYK"] {
            let mut bad = fixture(4, &[1000, 65535], vec![]);
            bad[16..20].copy_from_slice(&space);
            let bad = Profile::parse(&bad).unwrap();
            assert!(if side {
                Transform::new(&bad, &d, options(false))
            } else {
                Transform::new(&s, &bad, options(false))
            }
            .is_err());
        }
    }
    let lut_source = Profile::parse(&fixture(4, &[1000, 65535], vec![(*b"A2B1", lut)])).unwrap();
    assert!(Transform::new(&lut_source, &d, options(false)).is_err());
}
#[test]
fn matrix_bpc_diagnostics_ties_caps_and_metadata_are_explicit() {
    use icc_profile::BpcMethod;
    for (src, dst, endpoint) in [
        (vec![1000, 65535], vec![6000, 65535], [0.0; 3]),
        (vec![65535, 1000], vec![6000, 65535], [1.0; 3]),
        (vec![4000, 8000, 4000], vec![6000, 65535], [0.0; 3]),
        (vec![30000, 65535], vec![40000, 65535], [0.0; 3]),
    ] {
        let t = make(&src, &dst, true);
        let info = t.bpc_diagnostics().unwrap();
        assert_eq!(info.method(), BpcMethod::MatrixEndpointsLstar50);
        assert_eq!(info.source().endpoint(), endpoint);
        for (estimate, values) in [(info.source(), &src), (info.destination(), &dst)] {
            let y = *values.first().unwrap().min(values.last().unwrap()) as f64 / 65535.0;
            let l = lstar(y);
            assert!((estimate.raw_lstar() - l).abs() < 2e-5);
            assert!((estimate.capped_lstar() - l.min(50.0)).abs() < 2e-5);
            assert_eq!(estimate.was_capped(), l > 50.0);
            assert!((estimate.effective_y() - luminance(l.min(50.0))).abs() < 1e-7);
            for (i, white) in WHITE.iter().enumerate() {
                assert!((estimate.endpoint_xyz()[i] - white * y).abs() < 1e-7);
            }
        }
        let (scale, offset) = coefficients(&src, &dst);
        assert!((info.scale() - scale).abs() < 1e-7);
        for i in 0..3 {
            assert!((info.offset()[i] - offset[i]).abs() < 1e-7);
            // Neutralized source black and PCS white are the fixed endpoint
            // constraints; chromatic device black need not be mapped exactly.
            assert!(
                (info.scale() * PCS_WHITE[i] * info.source().effective_y() + info.offset()[i]
                    - PCS_WHITE[i] * info.destination().effective_y())
                .abs()
                    < 1e-12
            );
            assert!((info.scale() * PCS_WHITE[i] + info.offset()[i] - PCS_WHITE[i]).abs() < 1e-12);
        }
    }
    let source = Profile::parse(&fixture(4, &[1000, 65535], vec![])).unwrap();
    let plain = Profile::parse(&fixture(4, &[6000, 65535], vec![])).unwrap();
    let mut chad = vec![0; 44];
    chad[..4].copy_from_slice(b"sf32");
    for i in 0..3 {
        put(&mut chad, 8 + 16 * i, 98304);
    }
    let decorated = Profile::parse(&fixture(
        4,
        &[6000, 65535],
        vec![
            (*b"wtpt", xyz([0.75, 0.875, 0.625])),
            (*b"bkpt", xyz([0.1, 0.1, 0.1])),
            (*b"chad", chad),
        ],
    ))
    .unwrap();
    let a = Transform::new(&source, &plain, options(false)).unwrap();
    let b = Transform::new(&source, &decorated, options(false)).unwrap();
    let mut x = [0.0; 3];
    let mut y = [0.0; 3];
    a.transform_f32_atomic(&[0.2, 0.4, 0.7], &mut x).unwrap();
    b.transform_f32_atomic(&[0.2, 0.4, 0.7], &mut y).unwrap();
    assert_eq!(x, y);
    assert_eq!(
        a.bpc_diagnostics().unwrap().scale(),
        b.bpc_diagnostics().unwrap().scale()
    );
    assert!(Transform::new(&source, &plain, TransformOptions::default())
        .unwrap()
        .bpc_diagnostics()
        .is_none());
}

fn tag_offset(bytes: &[u8], signature: [u8; 4]) -> usize {
    let count = u32::from_be_bytes(bytes[128..132].try_into().unwrap()) as usize;
    for i in 0..count {
        let at = 132 + i * 12;
        if bytes[at..at + 4] == signature {
            return u32::from_be_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        }
    }
    panic!("missing fixture tag");
}

#[test]
fn valid_late_samples_outside_corrected_inverse_domain_preserve_atomic_output() {
    // Destination black exceeds the estimator's L*=50 cap. Valid dark RGB
    // therefore maps below the destination TRC domain with strict clipping.
    let t = make(&[1000, 65535], &[30000, 65535], false);
    let mut input = vec![0.8; 129 * 3];
    let mut ordinary = vec![0.0; 129 * 3];
    t.transform_f32_atomic(&input, &mut ordinary).unwrap();
    input[128 * 3..].fill(0.0);
    for worker in [false, true] {
        let mut out: Vec<f32> = (0..129 * 3)
            .map(|i| {
                if i % 2 == 0 {
                    -0.0
                } else {
                    f32::from_bits(0x3f123456)
                }
            })
            .collect();
        let before: Vec<_> = out.iter().map(|v| v.to_bits()).collect();
        let result = if worker {
            t.worker().transform_f32_atomic(&input, &mut out)
        } else {
            t.transform_f32_atomic(&input, &mut out)
        };
        assert!(result.is_err());
        assert_eq!(before, out.iter().map(|v| v.to_bits()).collect::<Vec<_>>());
    }
    let clamped = make(&[1000, 65535], &[30000, 65535], true);
    clamped.transform_f32_atomic(&input, &mut ordinary).unwrap();
    assert!(ordinary[128 * 3..].iter().all(|v| *v == 0.0));
}

#[test]
fn chromatic_endpoint_black_uses_only_y_for_neutral_compensation() {
    let mut bytes = fixture(4, &[1000, 65535], vec![]);
    for (tag, black) in [(*b"rTRC", 12000u16), (*b"bTRC", 20000u16)] {
        let at = tag_offset(&bytes, tag);
        bytes[at + 12..at + 14].copy_from_slice(&black.to_be_bytes());
    }
    let source = Profile::parse(&bytes).unwrap();
    let destination = Profile::parse(&fixture(4, &[6000, 65535], vec![])).unwrap();
    let t = Transform::new(&source, &destination, options(false)).unwrap();
    let info = t.bpc_diagnostics().unwrap();
    let black = [12000.0, 1000.0, 20000.0].map(|v| v / 65535.0);
    for i in 0..3 {
        assert!((info.source().endpoint_xyz()[i] - WHITE[i] * black[i]).abs() < 1e-7);
    }
    let (scale, offset) = coefficients(&[1000, 65535], &[6000, 65535]);
    assert!((info.scale() - scale).abs() < 1e-7);
    for (i, value) in offset.iter().enumerate() {
        assert!((info.offset()[i] - value).abs() < 1e-7);
    }
    let input = [0.2f32, 0.4, 0.7];
    let expected: [f32; 3] = std::array::from_fn(|i| {
        let linear = black[i] + (1.0 - black[i]) * input[i] as f64;
        (((scale * WHITE[i] * linear + offset[i]) / WHITE[i] - 6000.0 / 65535.0)
            / (1.0 - 6000.0 / 65535.0)) as f32
    });
    let mut actual = [0.0; 3];
    t.transform_f32_atomic(&input, &mut actual).unwrap();
    close(&actual, &expected);
    assert!(
        (info.source().endpoint_xyz()[0] - PCS_WHITE[0] * info.source().effective_y()).abs() > 0.1
    );
}

#[test]
fn failed_postmaterialization_estimator_does_not_poison_repeated_builds() {
    let good = Profile::parse(&fixture(4, &[1000, 65535], vec![])).unwrap();
    let destination = Profile::parse(&fixture(4, &[6000, 65535], vec![])).unwrap();
    let mut bytes = fixture(4, &[1000, 65535], vec![]);
    let at = tag_offset(&bytes, *b"gXYZ");
    put(&mut bytes, at + 12, (-65536i32) as u32);
    let invalid = Profile::parse(&bytes).unwrap();
    // Structurally valid, invertible matrix: ordinary compilation succeeds;
    // the BPC estimator rejects its negative endpoint luminance only later.
    Transform::new(&invalid, &destination, TransformOptions::default()).unwrap();
    let good_transform = Transform::new(&good, &destination, options(false)).unwrap();
    let limits = TransformLimits::builder()
        .max_compiled_bytes(good_transform.memory_usage().unwrap().compiled_bytes())
        .build()
        .unwrap();
    for _ in 0..4 {
        let result = Transform::new_with_limits(&invalid, &destination, options(false), limits);
        assert!(matches!(
            result,
            Err(icc_profile::TransformError::InvalidProfile(
                "invalid BPC matrix endpoint XYZ"
            ))
        ));
        let retry =
            Transform::new_with_limits(&good, &destination, options(false), limits).unwrap();
        let mut result = [0.0; 3];
        retry
            .transform_f32_atomic(&[0.2, 0.4, 0.7], &mut result)
            .unwrap();
        close(
            &result,
            &oracle([0.2, 0.4, 0.7], [1000, 65535], [6000, 65535], false),
        );
    }
}
