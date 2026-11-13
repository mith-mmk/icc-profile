//! Independent fixtures for the bounded v4 CMYK-source BPC estimator.
use icc_profile::{
    AlgorithmProfile, ClutInterpolation3D, Profile, RenderingIntent, Transform, TransformLimits,
    TransformOptions,
};
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
fn local_black(lab: bool, tri: bool) -> [f64; 4] {
    let v = if lab { 128.0 / 255.0 } else { 0.0 };
    let cross = if tri { v * v } else { v };
    [
        5000.0,
        7000.0 + 12000.0 * v,
        9000.0 + 8000.0 * v,
        4000.0 + 20000.0 * cross,
    ]
    .map(|v| v / 65535.0)
}
fn to_xyz(p: [f64; 4], lab: bool) -> [f64; 3] {
    let codes = forward_codes(p, lab);
    if !lab {
        return codes.map(|v| v / 32768.0);
    }
    let [l, a, b] = [
        codes[0] / 65535.0 * 100.0,
        codes[1] / 65535.0 * 255.0 - 128.0,
        codes[2] / 65535.0 * 255.0 - 128.0,
    ];
    let fy = (l + 16.0) / 116.0;
    let inv = |v: f64| {
        if v > 6.0 / 29.0 {
            v.powi(3)
        } else {
            3.0 * (6.0f64 / 29.0).powi(2) * (v - 4.0 / 29.0)
        }
    };
    [
        W[0] * inv(fy + a / 500.0),
        inv(fy),
        W[2] * inv(fy - b / 200.0),
    ]
}
fn oracle(p: [f64; 4], lab: bool, tri: bool, clamp: bool) -> [f32; 3] {
    let y = to_xyz(local_black(lab, tri), lab)[1].min(((50.0f64 + 16.0) / 116.0).powi(3));
    let s = (1.0 - 2000.0 / 65535.0) / (1.0 - y);
    let xyz = to_xyz(p, lab);
    std::array::from_fn(|i| {
        let v =
            ((s * xyz[i] + (1.0 - s) * W[i]) / MW[i] - 2000.0 / 65535.0) / (1.0 - 2000.0 / 65535.0);
        (if clamp { v.clamp(0.0, 1.0) } else { v }) as f32
    })
}
fn build(lab: bool, tri: bool, clamp: bool) -> Transform {
    Transform::new(
        &Profile::parse(&source(lab)).unwrap(),
        &Profile::parse(&destination()).unwrap(),
        options(clamp),
    )
    .unwrap()
    .with_algorithm_profile(AlgorithmProfile::default().with_clut_3d(if tri {
        ClutInterpolation3D::Trilinear
    } else {
        ClutInterpolation3D::Tetrahedral
    }))
}
fn close(a: &[f32], b: &[f32]) {
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 8e-6, "{a:?} != {b:?}");
    }
}
#[test]
fn cmyk_bpc_xyz_lab_and_k_match_independent_oracle() {
    for lab in [false, true] {
        for tri in [false, true] {
            let t = build(lab, tri, true);
            for p in [
                [0.0; 4],
                [0.8; 4],
                [0.2, 0.4, 0.6, 0.8],
                [0.8, 0.6, 0.4, 0.2],
                [1.0; 4],
            ] {
                let mut out = [0.0; 3];
                t.transform_f32_atomic(&p, &mut out).unwrap();
                close(&out, &oracle(p.map(|v| v as f64), lab, tri, true));
            }
        }
    }
    let a = build(true, false, true);
    let b = a.clone().with_algorithm_profile(
        AlgorithmProfile::default().with_clut_3d(ClutInterpolation3D::Trilinear),
    );
    let mut x = [0.0; 3];
    let mut y = x;
    a.transform_f32_atomic(&[0.8; 4], &mut x).unwrap();
    b.transform_f32_atomic(&[0.8; 4], &mut y).unwrap();
    assert!((x[1] - y[1]).abs() > 0.001);
}
#[test]
fn cmyk_bpc_workers_atomicity_and_algorithm_clones() {
    for lab in [false, true] {
        let t = build(lab, false, false);
        let mut old = t.worker();
        let tri = t.clone().with_algorithm_profile(
            AlgorithmProfile::default().with_clut_3d(ClutInterpolation3D::Trilinear),
        );
        for (transform, is_tri) in [(&t, false), (&tri, true)] {
            for n in [0, 1, 47, 48, 49, 95, 96, 97, 129] {
                let input = vec![0.8; n * 4];
                let mut out = vec![0.0; n * 3];
                transform
                    .worker()
                    .transform_f32_atomic(&input, &mut out)
                    .unwrap();
                for p in out.as_chunks::<3>().0.iter() {
                    close(p, &oracle([0.8; 4], lab, is_tri, false));
                }
                let mut integer = vec![0u16; n * 3];
                transform
                    .worker()
                    .transform_u16_atomic(&vec![52428; n * 4], &mut integer)
                    .unwrap();
                let e = oracle([0.8; 4], lab, is_tri, false).map(|v| (v * 65535.0).round() as i32);
                for p in integer.as_chunks::<3>().0.iter() {
                    for i in 0..3 {
                        assert!((p[i] as i32 - e[i]).abs() <= 1);
                    }
                }
            }
            for bad in [f32::NAN, 1.1, -0.1, 0.0] {
                let mut input = vec![0.8; 129 * 4];
                if bad == 0.0 {
                    input[128 * 4..].fill(0.0);
                } else {
                    *input.last_mut().unwrap() = bad;
                }
                let mut out = vec![-0.0f32; 129 * 3];
                assert!(transform
                    .worker()
                    .transform_f32_atomic(&input, &mut out)
                    .is_err());
                assert!(out.iter().all(|v| v.to_bits() == (-0.0f32).to_bits()));
            }
        }
        let mut output = [0.0; 3];
        old.transform_f32_atomic(&[0.8; 4], &mut output).unwrap();
        close(&output, &oracle([0.8; 4], lab, false, false));
    }
}
#[test]
fn cmyk_bpc_auxiliary_limits_and_scope_fail_closed() {
    let s = Profile::parse(&source(true)).unwrap();
    let d = Profile::parse(&destination()).unwrap();
    let t = Transform::new(&s, &d, options(true)).unwrap();
    let memory = t.memory_usage().unwrap();
    assert!(memory.build_peak_bytes() > memory.resident_bytes());
    assert_eq!(
        memory.resident_bytes(),
        memory.profile_bytes() + memory.compiled_bytes()
    );
    let doubled = memory.checked_add(memory).unwrap();
    assert_eq!(doubled.build_peak_bytes(), memory.build_peak_bytes() * 2);
    assert_eq!(doubled.resident_bytes(), memory.resident_bytes() * 2);
    assert_eq!(doubled.compiled_bytes(), memory.compiled_bytes() * 2);
    assert_eq!(doubled.profile_bytes(), memory.profile_bytes() * 2);
    let bytes = memory.build_peak_bytes() - memory.profile_bytes();
    for (limit, ok) in [(bytes, true), (bytes - 1, false)] {
        assert_eq!(
            Transform::new_with_limits(
                &s,
                &d,
                options(true),
                TransformLimits::builder()
                    .max_compiled_bytes(limit)
                    .build()
                    .unwrap()
            )
            .is_ok(),
            ok
        );
    }
    for (entries, ok) in [(80, true), (79, false)] {
        assert_eq!(
            Transform::new_with_limits(
                &s,
                &d,
                options(true),
                TransformLimits::builder()
                    .max_clut_entries(entries)
                    .build()
                    .unwrap()
            )
            .is_ok(),
            ok
        );
    }
    for (entries, ok) in [(21, true), (20, false)] {
        assert_eq!(
            Transform::new_with_limits(
                &s,
                &d,
                options(true),
                TransformLimits::builder()
                    .max_curve_entries(entries)
                    .build()
                    .unwrap()
            )
            .is_ok(),
            ok
        );
    }
    for tags in [
        vec![(*b"A2B1", forward(true))],
        vec![(*b"A2B0", forward(true)), (*b"B2A0", reverse())],
        vec![(*b"A2B1", forward(true)), (*b"B2A1", reverse())],
        vec![(*b"A2B1", forward(true)), (*b"B2A0", b"bad!".to_vec())],
    ] {
        let p = Profile::parse(&profile(*b"CMYK", *b"Lab ", tags)).unwrap();
        assert!(Transform::new(&p, &d, options(true)).is_err());
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
                ..options(true)
            }
        )
        .is_err());
    }
    for (at, value) in [(8, 2u8), (12, b's'), (68, 1)] {
        let mut b = source(true);
        b[at] = value;
        let p = Profile::parse(&b).unwrap();
        assert!(Transform::new(&p, &d, options(true)).is_err());
    }
}

#[test]
fn cmyk_bpc_diagnostics_preserve_four_channels_and_current_algorithm() {
    for lab in [false, true] {
        for tri in [false, true] {
            let t = build(lab, tri, true);
            assert!(t.bpc_diagnostics().is_none());
            let info = t.cmyk_bpc_diagnostics().unwrap();
            let device = local_black(lab, tri);
            close(&info.device_black(), &device.map(|v| v as f32));
            let xyz = to_xyz(device, lab);
            for (i, value) in xyz.iter().enumerate() {
                assert!((info.source_xyz()[i] - value).abs() < 2e-6);
            }
            assert_eq!(info.auxiliary_route().selected_tag(), Some(*b"B2A0"));
            assert!(!info.auxiliary_route().used_fallback());
            assert_eq!(
                info.interpolation(),
                if tri {
                    ClutInterpolation3D::Trilinear
                } else {
                    ClutInterpolation3D::Tetrahedral
                }
            );
            let capped_y = xyz[1].min(((50.0f64 + 16.0) / 116.0).powi(3));
            let scale = (1.0 - 2000.0 / 65535.0) / (1.0 - capped_y);
            assert!((info.effective_y() - capped_y).abs() < 2e-6);
            assert!((info.scale() - scale).abs() < 3e-6);
            for (i, white) in W.iter().enumerate() {
                assert!((info.offset()[i] - (1.0 - scale) * white).abs() < 3e-6);
            }
        }
    }
}

/// Parent-authorized export of original synthetic fixtures for external black-box
/// comparison. Never reads or modifies manufacturer profiles.
#[test]
#[ignore = "explicit synthetic profile export for external comparison"]
fn export_original_cmyk_bpc_fixtures() {
    let directory = std::path::Path::new(".test-cms-compare");
    std::fs::create_dir_all(directory).unwrap();
    for (name, lab) in [
        ("bpc-original-cmyk-lab.icc", true),
        ("bpc-original-cmyk-xyz.icc", false),
    ] {
        std::fs::write(directory.join(name), source(lab)).unwrap();
    }
    std::fs::write(directory.join("bpc-original-rgb-matrix.icc"), destination()).unwrap();
}
#[test]
fn cmyk_bpc_invalid_auxiliary_probe_fails_then_valid_build_recovers() {
    let mut auxiliary = reverse();
    let matrix_offset = auxiliary.len();
    let mut matrix = vec![0; 48];
    for i in 0..3 {
        put(&mut matrix, i * 16, 65536);
    }
    put(&mut matrix, 36, 131072);
    auxiliary.extend(matrix);
    put(&mut auxiliary, 16, matrix_offset as u32);
    let m_offset = auxiliary.len();
    for _ in 0..3 {
        auxiliary.extend(curve(false));
    }
    put(&mut auxiliary, 20, m_offset as u32);
    let invalid = Profile::parse(&profile(
        *b"CMYK",
        *b"Lab ",
        vec![(*b"A2B1", forward(true)), (*b"B2A0", auxiliary)],
    ))
    .unwrap();
    let destination = Profile::parse(&destination()).unwrap();
    // Relative execution ignores the auxiliary reverse tag until BPC is on.
    Transform::new(&invalid, &destination, TransformOptions::default()).unwrap();
    for _ in 0..3 {
        assert!(Transform::new(&invalid, &destination, options(true)).is_err());
        let good = build(true, false, true);
        let mut out = [0.0; 3];
        good.transform_f32_atomic(&[0.8; 4], &mut out).unwrap();
        close(&out, &oracle([0.8; 4], true, false, true));
    }
}

#[test]
fn cmyk_bpc_caps_high_luminance_probe_at_lstar50() {
    let destination = Profile::parse(&destination()).unwrap();
    for lab in [false, true] {
        let mut high = forward(lab);
        // Constant, deliberately chromatic forward PCS black above L*=50.
        // This isolates the CMYK estimator cap from reverse interpolation.
        let codes: [u16; 3] = if lab {
            [50000, 34000, 32000]
        } else {
            [18000, 20000, 16000]
        };
        for corner in high[52..52 + 16 * 6].as_chunks_mut::<6>().0.iter_mut() {
            for (sample, code) in corner.as_chunks_mut::<2>().0.iter_mut().zip(codes) {
                sample.copy_from_slice(&code.to_be_bytes());
            }
        }
        let source = Profile::parse(&profile(
            *b"CMYK",
            if lab { *b"Lab " } else { *b"XYZ " },
            vec![(*b"A2B1", high), (*b"B2A0", reverse())],
        ))
        .unwrap();
        let transform = Transform::new(&source, &destination, options(false)).unwrap();
        for interpolation in [
            ClutInterpolation3D::Tetrahedral,
            ClutInterpolation3D::Trilinear,
        ] {
            let t = transform
                .clone()
                .with_algorithm_profile(AlgorithmProfile::default().with_clut_3d(interpolation));
            let diagnostic = t.cmyk_bpc_diagnostics().unwrap();
            let raw_l = if lab {
                50000.0 / 65535.0 * 100.0
            } else {
                116.0 * (20000.0_f64 / 32768.0).cbrt() - 16.0
            };
            assert!((diagnostic.raw_lstar() - raw_l).abs() < 2e-5);
            assert!(diagnostic.was_capped());
            assert_eq!(diagnostic.capped_lstar(), 50.0);
            let effective_y = (66.0_f64 / 116.0).powi(3);
            assert!((diagnostic.effective_y() - effective_y).abs() < 1e-12);
            let scale = (1.0 - 2000.0 / 65535.0) / (1.0 - effective_y);
            assert!((diagnostic.scale() - scale).abs() < 1e-7);
            for (i, white) in W.iter().enumerate() {
                assert!((diagnostic.offset()[i] - (1.0 - scale) * white).abs() < 1e-7);
            }
            let mut output = [0.0; 3];
            t.transform_f32_atomic(&[0.8; 4], &mut output).unwrap();
            let expected = std::array::from_fn::<_, 3, _>(|i| {
                (((scale * diagnostic.source_xyz()[i] + (1.0 - scale) * W[i]) / MW[i]
                    - 2000.0 / 65535.0)
                    / (1.0 - 2000.0 / 65535.0)) as f32
            });
            close(&output, &expected);
        }
    }
}

#[test]
fn cmyk_bpc_estimator_does_not_depend_on_unselected_a2b0() {
    // External engines can impose extra black-detection preconditions. Our
    // explicit B2A0 -> A2B1 estimator must not switch to an unselected A2B0.
    for lab in [false, true] {
        let minimal = build(lab, false, true);
        let with_extra = Profile::parse(&profile(
            *b"CMYK",
            if lab { *b"Lab " } else { *b"XYZ " },
            vec![
                (*b"A2B1", forward(lab)),
                (*b"B2A0", reverse()),
                (*b"A2B0", forward(lab)),
            ],
        ))
        .unwrap();
        let destination = Profile::parse(&destination()).unwrap();
        let extra = Transform::new(&with_extra, &destination, options(true)).unwrap();
        assert_eq!(minimal.cmyk_bpc_diagnostics(), extra.cmyk_bpc_diagnostics());
        for pixel in [[0.0; 4], [0.8; 4], [0.2, 0.4, 0.6, 0.8]] {
            let mut a = [0.0; 3];
            let mut b = [0.0; 3];
            minimal.transform_f32_atomic(&pixel, &mut a).unwrap();
            extra.transform_f32_atomic(&pixel, &mut b).unwrap();
            assert_eq!(a, b);
        }
    }
}
