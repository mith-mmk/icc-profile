//! Original synthetic destination fixtures and independent analytic contracts.
use icc_profile::{
    AlgorithmProfile, ClutInterpolation3D, Profile, RenderingIntent, Transform, TransformLimits,
    TransformOptions,
};
#[allow(dead_code)]
const W: [f64; 3] = [0.9642, 1.0, 0.8249];
const MW: [f64; 3] = [63190.0 / 65536.0, 1.0, 54061.0 / 65536.0];
fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_be_bytes());
}
fn curve(sampled: bool) -> Vec<u8> {
    let mut v = vec![0; 12];
    v[..4].copy_from_slice(b"curv");
    if sampled {
        put(&mut v, 8, 2);
        v.extend([0, 0, 255, 255]);
    }
    v
}
fn profile(space: [u8; 4], pcs: [u8; 4], tags: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
    let mut b = vec![0; 132 + 12 * tags.len()];
    b[8] = 4;
    b[12..16].copy_from_slice(if space == *b"CMYK" { b"prtr" } else { b"mntr" });
    b[16..20].copy_from_slice(&space);
    b[20..24].copy_from_slice(&pcs);
    b[36..40].copy_from_slice(b"acsp");
    for (at, v) in [
        (68, 63190),
        (72, 65536),
        (76, 54061),
        (128, tags.len() as u32),
    ] {
        put(&mut b, at, v);
    }
    for (i, (sig, data)) in tags.into_iter().enumerate() {
        while !b.len().is_multiple_of(4) {
            b.push(0);
        }
        let off = b.len();
        put(&mut b, 132 + i * 12, u32::from_be_bytes(sig));
        put(&mut b, 136 + i * 12, off as u32);
        put(&mut b, 140 + i * 12, data.len() as u32);
        b.extend(data);
    }
    let len = b.len();
    put(&mut b, 0, len as u32);
    b
}
fn forward_codes(p: [f64; 4], lab: bool) -> [f64; 3] {
    let [c, m, y, k] = p;
    if lab {
        [
            10000.0 + 8000.0 * c + 12000.0 * k,
            32896.0 + 1500.0 * m,
            32896.0 - 1200.0 * y,
        ]
    } else {
        [
            1000.0 + 3000.0 * c + 2000.0 * k,
            1200.0 + 4000.0 * m + 3000.0 * k,
            900.0 + 2500.0 * y + 1500.0 * k,
        ]
    }
}
fn forward(lab: bool) -> Vec<u8> {
    let mut t = vec![0; 52];
    t[..4].copy_from_slice(b"mAB ");
    t[8] = 4;
    t[9] = 3;
    put(&mut t, 24, 32);
    t[32..36].fill(2);
    t[48] = 2;
    for c in [0.0, 1.0] {
        for m in [0.0, 1.0] {
            for y in [0.0, 1.0] {
                for k in [0.0, 1.0] {
                    for v in forward_codes([c, m, y, k], lab) {
                        t.extend_from_slice(&(v as u16).to_be_bytes());
                    }
                }
            }
        }
    }
    let off = t.len();
    put(&mut t, 12, off as u32);
    for _ in 0..3 {
        t.extend(curve(false));
    }
    let off = t.len();
    put(&mut t, 28, off as u32);
    for _ in 0..4 {
        t.extend(curve(false));
    }
    t
}
fn reverse() -> Vec<u8> {
    let mut t = vec![0; 52];
    t[..4].copy_from_slice(b"mBA ");
    t[8] = 3;
    t[9] = 4;
    put(&mut t, 24, 32);
    t[32..35].fill(2);
    t[48] = 2;
    for x in [0u16, 1] {
        for y in [0u16, 1] {
            for z in [0u16, 1] {
                for v in [
                    5000 + 10000 * x,
                    7000 + 12000 * y,
                    9000 + 8000 * z,
                    4000 + 20000 * y * z,
                ] {
                    t.extend_from_slice(&v.to_be_bytes());
                }
            }
        }
    }
    let off = t.len();
    put(&mut t, 12, off as u32);
    for _ in 0..3 {
        t.extend(curve(false));
    }
    let off = t.len();
    put(&mut t, 28, off as u32);
    for i in 0..4 {
        t.extend(curve(i == 3));
    }
    t
}
#[allow(dead_code)]
fn source(lab: bool) -> Vec<u8> {
    profile(
        *b"CMYK",
        if lab { *b"Lab " } else { *b"XYZ " },
        vec![(*b"A2B1", forward(lab)), (*b"B2A0", reverse())],
    )
}
fn destination() -> Vec<u8> {
    let mut tags = vec![];
    for (i, (xyz, trc)) in [
        (*b"rXYZ", *b"rTRC"),
        (*b"gXYZ", *b"gTRC"),
        (*b"bXYZ", *b"bTRC"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut t = vec![0; 20];
        t[..4].copy_from_slice(b"XYZ ");
        put(&mut t, 8 + i * 4, (MW[i] * 65536.0) as u32);
        tags.push((xyz, t));
        let mut c = curve(true);
        c[12..14].copy_from_slice(&2000u16.to_be_bytes());
        tags.push((trc, c));
    }
    profile(*b"RGB ", *b"XYZ ", tags)
}
fn options(clamp: bool) -> TransformOptions {
    TransformOptions {
        black_point_compensation: true,
        clamp,
        ..Default::default()
    }
}

fn printer() -> Vec<u8> {
    // Relative round trip is Lout = 20 + .8 Lin: not midrange straight.
    // Its normalized shadow ramp is Lin/100, whose fitted positive root is zero.
    let mut f = forward(true);
    for vertex in 0..16 {
        let k = vertex % 2;
        let codes = [if k == 0 { 13107u16 } else { 65535u16 }, 32896, 32896];
        for (c, v) in codes.iter().enumerate() {
            f[52 + vertex * 6 + c * 2..54 + vertex * 6 + c * 2].copy_from_slice(&v.to_be_bytes());
        }
    }
    let mut r = reverse();
    for vertex in 0..8 {
        let l = vertex / 4;
        for c in 0..4 {
            let v = if c == 3 && l == 1 { 65535u16 } else { 0u16 };
            r[52 + vertex * 8 + c * 2..54 + vertex * 8 + c * 2].copy_from_slice(&v.to_be_bytes());
        }
    }
    profile(
        *b"CMYK",
        *b"Lab ",
        vec![(*b"A2B1", f), (*b"B2A1", r.clone()), (*b"B2A0", r)],
    )
}
fn make() -> Transform {
    Transform::new(
        &Profile::parse(&destination()).unwrap(),
        &Profile::parse(&printer()).unwrap(),
        options(true),
    )
    .unwrap()
}
#[test]
fn quadratic_destination_and_four_channel_pipeline_match_f64() {
    for interpolation in [
        ClutInterpolation3D::Trilinear,
        ClutInterpolation3D::Tetrahedral,
    ] {
        let t =
            make().with_algorithm_profile(AlgorithmProfile::default().with_clut_3d(interpolation));
        let d = t.cmyk_output_bpc_diagnostics().unwrap();
        assert_eq!(d.method(), icc_profile::CmykOutputBpcMethod::QuadraticFit);
        assert!((d.initial_lstar() - 20.0).abs() < 1e-4);
        assert!(d.destination_lstar() < 1e-3);
        assert!(d.shadow_points() > 3);
        assert!(t.bpc_diagnostics().is_none());
        assert!(t.cmyk_bpc_diagnostics().is_none());
        for v in [0.0, 0.01, 0.2, 0.5, 1.0] {
            let mut out = [0.0; 4];
            t.transform_f32_atomic(&[v; 3], &mut out).unwrap();
            // Analytic matrix source black cancellation leaves Y=v, then Lab L*.
            let y = v as f64;
            let l = if y > 216.0 / 24389.0 {
                116.0 * y.cbrt() - 16.0
            } else {
                24389.0 / 27.0 * y
            };
            assert!(out[..3].iter().all(|x| *x == 0.0));
            assert!((out[3] as f64 - l / 100.0).abs() < 2e-5);
        }
    }
}
#[test]
fn chunks_workers_integers_and_atomic_late_failures() {
    let t = make();
    let input = vec![0.25; 3 * 137];
    let mut expected = vec![0.0; 4 * 137];
    t.transform_f32(&input, &mut expected).unwrap();
    let mut w = t.worker();
    let mut out = vec![0.0; expected.len()];
    for _ in 0..3 {
        w.transform_f32_atomic(&input, &mut out).unwrap();
        assert_eq!(out, expected);
    }
    let mut bad = input;
    bad[3 * 136] = f32::NAN;
    out.fill(0.731);
    assert!(w.transform_f32_atomic(&bad, &mut out).is_err());
    assert!(out.iter().all(|v| *v == 0.731));
    let mut bytes = vec![0; 4 * 137];
    t.transform_u8(&vec![64; 3 * 137], &mut bytes).unwrap();
    let mut words = vec![0; 4 * 137];
    w.transform_u16(&vec![16384; 3 * 137], &mut words).unwrap();
    assert!(bytes.chunks_exact(4).all(|p| p[3] > 0 && p[..3] == [0; 3]));
    assert!(words.chunks_exact(4).all(|p| p[3] > 0 && p[..3] == [0; 3]));
}
#[test]
fn admission_boundaries_and_route_rejection() {
    let src = Profile::parse(&destination()).unwrap();
    let dst = Profile::parse(&printer()).unwrap();
    let t = make();
    let m = t.memory_usage().unwrap();
    let charge = m.build_peak_bytes() - m.profile_bytes();
    assert!(m.build_peak_bytes() > m.resident_bytes() + 2048);
    for (limit, ok) in [(charge, true), (charge - 1, false)] {
        let limits = TransformLimits::builder()
            .max_compiled_bytes(limit)
            .build()
            .unwrap();
        assert_eq!(
            Transform::new_with_limits(&src, &dst, options(true), limits).is_ok(),
            ok
        );
    }
    for tag in [b"A2B1", b"B2A0", b"B2A1"] {
        let mut b = printer();
        for i in 0..3 {
            if &b[132 + i * 12..136 + i * 12] == tag {
                b[132 + i * 12..136 + i * 12].copy_from_slice(b"zzzz");
            }
        }
        assert!(Transform::new(&src, &Profile::parse(&b).unwrap(), options(true)).is_err());
    }
    for intent in [
        RenderingIntent::AbsoluteColorimetric,
        RenderingIntent::Perceptual,
        RenderingIntent::Saturation,
    ] {
        let mut o = options(true);
        o.rendering_intent = intent;
        assert!(Transform::new(&src, &dst, o).is_err());
    }
}
#[test]
#[ignore = "exports original fixtures for public-API comparisons"]
fn export_output_bpc_fixtures() {
    std::fs::write(
        ".test-cms-compare/bpc-output-original-rgb.icc",
        destination(),
    )
    .unwrap();
    std::fs::write(".test-cms-compare/bpc-output-original-cmyk.icc", printer()).unwrap();
}

#[test]
fn interpolation_estimates_are_immutable_and_auxiliary_entries_are_admitted() {
    let src = Profile::parse(&destination()).unwrap();
    let bytes = profile(
        *b"CMYK",
        *b"Lab ",
        vec![
            (*b"A2B1", forward(true)),
            (*b"B2A0", reverse()),
            (*b"B2A1", reverse()),
        ],
    );
    let dst = Profile::parse(&bytes).unwrap();
    let t = Transform::new(&src, &dst, options(true)).unwrap();
    let original = t.cmyk_output_bpc_diagnostics().unwrap();
    let tri = t.clone().with_algorithm_profile(
        AlgorithmProfile::default().with_clut_3d(ClutInterpolation3D::Trilinear),
    );
    let switched = tri.cmyk_output_bpc_diagnostics().unwrap();
    assert_ne!(
        original.initial_device_black()[3],
        switched.initial_device_black()[3]
    );
    assert_ne!(original.initial_lstar(), switched.initial_lstar());
    assert_eq!(t.cmyk_output_bpc_diagnostics().unwrap(), original);
    let worker = t.worker();
    assert_eq!(worker.algorithm_profile(), t.algorithm_profile());
    for (entries, ok) in [(112, true), (111, false)] {
        let limits = TransformLimits::builder()
            .max_clut_entries(entries)
            .build()
            .unwrap();
        assert_eq!(
            Transform::new_with_limits(&src, &dst, options(true), limits).is_ok(),
            ok
        );
    }
    for (entries, ok) in [(29, true), (28, false)] {
        let limits = TransformLimits::builder()
            .max_curve_entries(entries)
            .build()
            .unwrap();
        assert_eq!(
            Transform::new_with_limits(&src, &dst, options(true), limits).is_ok(),
            ok
        );
    }
}

#[test]
fn strict_derived_pcs_failure_is_atomic_and_clamping_succeeds() {
    let mut source = destination();
    let count = u32::from_be_bytes(source[128..132].try_into().unwrap()) as usize;
    for i in 0..count {
        if &source[132 + i * 12..136 + i * 12] == b"gXYZ" {
            let offset =
                u32::from_be_bytes(source[136 + i * 12..140 + i * 12].try_into().unwrap()) as usize;
            put(&mut source, offset + 12, 131072);
        }
    }
    let src = Profile::parse(&source).unwrap();
    let dst = Profile::parse(&printer()).unwrap();
    let strict = Transform::new(&src, &dst, options(false)).unwrap();
    let clamped = Transform::new(&src, &dst, options(true)).unwrap();
    let mut input = vec![0.25; 3 * 137];
    input[3 * 136..].fill(1.0);
    let mut good = [0.0; 4];
    strict.transform_f32_atomic(&input[..3], &mut good).unwrap();
    let mut output = vec![0.731; 4 * 137];
    assert!(strict.transform_f32_atomic(&input, &mut output).is_err());
    assert!(output.iter().all(|v| *v == 0.731));
    let mut worker = strict.worker();
    for _ in 0..2 {
        assert!(worker.transform_f32_atomic(&input, &mut output).is_err());
        assert!(output.iter().all(|v| *v == 0.731));
    }
    let mut words = vec![16384; 3 * 137];
    words[3 * 136..].fill(65535);
    let mut integer_output = vec![12345; 4 * 137];
    assert!(worker.transform_u16(&words, &mut integer_output).is_err());
    assert!(integer_output.iter().all(|v| *v == 12345));
    clamped.transform_f32_atomic(&input, &mut output).unwrap();
    assert!(output
        .iter()
        .all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
}
