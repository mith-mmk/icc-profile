use icc_profile::cms::transration::*;
use icc_profile::{Data, DecodedICCProfile, Mft1, S15Fixed16Number, XYZNumber};
use std::collections::HashMap;
use std::io::ErrorKind;
fn fixture(n: u8, m: u8) -> Mft1 {
    Mft1 {
        input_channels: n,
        output_channels: m,
        number_of_clut_grid_points: 2,
        e_params: (0..9)
            .map(|i| S15Fixed16Number::new(if i % 4 == 0 { 1.0 } else { 0.0 }))
            .collect(),
        input_table: (0..n).flat_map(|_| 0..=255).collect(),
        clut_values: vec![0; (1 << n) * m as usize],
        output_table: (0..m).flat_map(|_| 0..=255).collect(),
    }
}
fn profile(tag: &str, lut: Mft1) -> DecodedICCProfile {
    DecodedICCProfile {
        length: 0,
        cmmid: 0,
        version: 0x04000000,
        device_class: u32::from_be_bytes(*b"prtr"),
        color_space: u32::from_be_bytes(*b"CMYK"),
        pcs: u32::from_be_bytes(*b"Lab "),
        create_date: String::new(),
        magicnumber_ascp: u32::from_be_bytes(*b"acsp"),
        platform: 0,
        flags: 0,
        manufacturer: 0,
        model: 0,
        attributes: 0,
        rendering_intent: 0,
        illuminate: XYZNumber {
            x: S15Fixed16Number::new(0.9642),
            y: S15Fixed16Number::new(1.0),
            z: S15Fixed16Number::new(0.8249),
        },
        creator: 0,
        profile_id: 0,
        tags: HashMap::from([(tag.into(), Data::Lut8(lut))]),
    }
}
#[test]
fn reverse_uses_input_and_output_tables_not_nearest_clut() {
    let mut lut = fixture(3, 4);
    // All inputs become the upper corner, independent of RGB/Lab conversion.
    lut.input_table.fill(255);
    lut.clut_values[28..32].copy_from_slice(&[17, 33, 65, 129]);
    for channel in 0..4 {
        for i in 0..256 {
            lut.output_table[channel * 256 + i] = 255 - i as u8;
        }
    }
    let p = profile("B2A0", lut);
    for rgb in [(0, 0, 0), (255, 255, 255), (81, 162, 243)] {
        let expected = (238, 222, 190, 126);
        assert_eq!(
            try_rgb_to_cmyk_from_profile(rgb.0, rgb.1, rgb.2, &p).unwrap(),
            expected
        );
        assert_eq!(rgb_to_cmyk_from_profile(rgb.0, rgb.1, rgb.2, &p), expected);
    }
}
#[test]
fn reverse_preserves_legacy_matrix_before_input_curves() {
    let mut lut = fixture(3, 4);
    // Zero matrix chooses coordinate zero. Constant 64 input curves then sample
    // the CLUT interior, proving interpolation instead of nearest selection.
    lut.e_params = (0..9).map(|_| S15Fixed16Number::new(0.0)).collect();
    lut.input_table.fill(64);
    for node in 0..8 {
        lut.clut_values[node * 4] = if node & 4 != 0 { 255 } else { 0 };
    }
    let p = profile("B2A0", lut);
    assert_eq!(
        try_rgb_to_cmyk_from_profile(255, 0, 0, &p).unwrap(),
        (64, 0, 0, 0)
    );
}
#[test]
fn invalid_metadata_and_missing_routes_return_errors_without_unwind() {
    for direction in [false, true] {
        for invalid in 0..9 {
            let mut p = profile(
                if direction { "B2A0" } else { "A2B0" },
                fixture(if direction { 3 } else { 4 }, if direction { 4 } else { 3 }),
            );
            match invalid {
                0 => p.color_space = u32::from_be_bytes(*b"RGB "),
                1 => p.pcs = u32::from_be_bytes(*b"XYZ "),
                2 => p.version = 0x05000000,
                3 => p.device_class = u32::from_be_bytes(*b"link"),
                4 => p.illuminate.x = S15Fixed16Number::new(0.95),
                5 => p.tags.clear(),
                6 => {
                    p.tags.insert(
                        if direction { "B2A0" } else { "A2B0" }.into(),
                        Data::Raw("mAB ".into(), vec![]),
                    );
                }
                7 => p.version = 0,
                _ => p.pcs = 0,
            }
            let result = std::panic::catch_unwind(|| {
                if direction {
                    try_rgb_to_cmyk_from_profile(1, 2, 3, &p).map(|_| ())
                } else {
                    try_cmyk_to_rgb_from_profile(1, 2, 3, 4, &p).map(|_| ())
                }
            });
            assert_eq!(result.unwrap().unwrap_err().kind(), ErrorKind::Unsupported);
        }
    }
}
#[test]
fn malformed_shapes_error_checked_and_panic_legacy_except_old_missing_node() {
    for direction in [false, true] {
        for fault in 0..7 {
            let mut lut = fixture(if direction { 3 } else { 4 }, if direction { 4 } else { 3 });
            match fault {
                0 => lut.number_of_clut_grid_points = 0,
                1 => lut.number_of_clut_grid_points = 1,
                2 => lut.input_channels = 1,
                3 => lut.output_channels = 1,
                4 => lut.input_table.clear(),
                5 => lut.output_table.clear(),
                _ => lut.clut_values.clear(),
            }
            let p = profile(if direction { "B2A0" } else { "A2B0" }, lut);
            if direction {
                assert_eq!(
                    try_rgb_to_cmyk_from_profile(0, 0, 0, &p)
                        .unwrap_err()
                        .kind(),
                    ErrorKind::InvalidInput
                );
                let legacy = std::panic::catch_unwind(|| rgb_to_cmyk_from_profile(0, 0, 0, &p));
                if fault == 6 {
                    assert_eq!(legacy.unwrap(), (255, 255, 255, 255));
                } else {
                    assert!(legacy.is_err());
                }
            } else {
                assert_eq!(
                    try_cmyk_to_rgb_from_profile(0, 0, 255, 255, &p)
                        .unwrap_err()
                        .kind(),
                    ErrorKind::InvalidInput
                );
                assert!(
                    std::panic::catch_unwind(|| cmyk_to_rgb_from_profile(0, 0, 255, 255, &p))
                        .is_err()
                );
            }
        }
    }
}
#[test]
fn both_supported_versions_work_and_wtpt_is_not_header_illuminant() {
    for version in [0x02000000, 0x04000000] {
        let mut p = profile("A2B0", fixture(4, 3));
        p.version = version;
        let before = try_cmyk_to_rgb_from_profile(0, 128, 255, 0, &p).unwrap();
        p.tags.insert(
            "wtpt".into(),
            Data::XYZNumber(XYZNumber {
                x: S15Fixed16Number::new(0.1),
                y: S15Fixed16Number::new(0.2),
                z: S15Fixed16Number::new(0.3),
            }),
        );
        assert_eq!(
            try_cmyk_to_rgb_from_profile(0, 128, 255, 0, &p).unwrap(),
            before
        );
    }
}
#[test]
fn nonidentity_matrix_changes_legacy_reverse_result() {
    let mut lut = fixture(3, 4);
    for node in 0..8 {
        lut.clut_values[node * 4] = if node & 4 != 0 { 255 } else { 0 };
    }
    let mut p = profile("B2A0", lut);
    let original = try_rgb_to_cmyk_from_profile(255, 255, 255, &p).unwrap();
    assert!(original.0 > 200);
    if let Some(Data::Lut8(lut)) = p.tags.get_mut("B2A0") {
        lut.e_params[0] = S15Fixed16Number::new(0.0);
    }
    assert_eq!(
        try_rgb_to_cmyk_from_profile(255, 255, 255, &p).unwrap(),
        (0, 0, 0, 0)
    );
}
#[test]
fn lut16_is_explicitly_unsupported_checked_but_full_stage_in_compatibility() {
    use icc_profile::Mft2;
    for version in [0x02000000, 0x04000000] {
        let mut p = profile("B2A0", fixture(3, 4));
        p.version = version;
        let mut lut = Mft2 {
            input_channels: 3,
            output_channels: 4,
            number_of_clut_grid_points: 2,
            e_params: (0..9)
                .map(|i| S15Fixed16Number::new(if i % 4 == 0 { 1.0 } else { 0.0 }))
                .collect(),
            input_table_enteries: 2,
            output_table_enteries: 2,
            input_table: vec![65535; 6],
            clut_values: vec![0; 32],
            output_table: vec![65535, 0, 65535, 0, 65535, 0, 65535, 0],
        };
        lut.clut_values[28..32].copy_from_slice(&[2570, 5140, 7710, 10280]);
        p.tags.insert("B2A0".into(), Data::Lut16(lut));
        assert_eq!(
            try_rgb_to_cmyk_from_profile(0, 0, 0, &p)
                .unwrap_err()
                .kind(),
            ErrorKind::Unsupported
        );
        assert_eq!(rgb_to_cmyk_from_profile(0, 0, 0, &p), (245, 235, 225, 215));
        if let Some(Data::Lut16(lut)) = p.tags.get_mut("B2A0") {
            lut.clut_values.truncate(4);
        }
        assert!(std::panic::catch_unwind(|| rgb_to_cmyk_from_profile(0, 0, 0, &p)).is_err());
        assert_eq!(rgb_to_cmyk_from_profile(255, 255, 255, &p), (0, 0, 0, 0));
        if let Some(Data::Lut16(lut)) = p.tags.get_mut("B2A0") {
            lut.clut_values.truncate(3);
        }
        assert_eq!(rgb_to_cmyk_from_profile(0, 0, 0, &p), (255, 255, 255, 255));
        if let Some(Data::Lut16(lut)) = p.tags.get_mut("B2A0") {
            lut.number_of_clut_grid_points = 0;
        }
        assert!(std::panic::catch_unwind(|| rgb_to_cmyk_from_profile(0, 0, 0, &p)).is_err());
    }
}
#[test]
fn forward_lut8_runs_all_stages_at_byte_endpoints() {
    let mut lut = fixture(4, 3);
    lut.input_table.fill(255);
    lut.clut_values[45..48].copy_from_slice(&[100, 127, 127]);
    // Zero output curves make any selected CLUT value become encoded zero Lab.
    lut.output_table.fill(0);
    let p = profile("A2B0", lut);
    let reference = profile("A2B0", fixture(4, 3));
    for input in [(0, 0, 0, 0), (255, 255, 255, 255), (25, 50, 75, 100)] {
        assert_eq!(
            try_cmyk_to_rgb_from_profile(input.0, input.1, input.2, input.3, &p).unwrap(),
            try_cmyk_to_rgb_from_profile(0, 0, 0, 0, &reference).unwrap()
        );
    }
}

#[test]
fn forward_c_and_y_axes_are_not_permuted_and_legacy_fallback_is_explicit() {
    let mut lut = fixture(4, 3);
    // C varies slowest: L follows only C; a=b=127 encode neutral chroma.
    // At C=255 this gives L*=100 -> XYZ=D50 -> legacy linear RGB
    // (255,249,184), using the documented matrix, no production helper oracle.
    for node in 0..16 {
        lut.clut_values[node * 3] = if node & 8 != 0 { 255 } else { 0 };
        lut.clut_values[node * 3 + 1] = 127;
        lut.clut_values[node * 3 + 2] = 127;
    }
    let mut p = profile("A2B0", lut);
    assert_eq!(
        try_cmyk_to_rgb_from_profile(255, 0, 0, 0, &p).unwrap(),
        (255, 249, 184)
    );
    assert_eq!(
        try_cmyk_to_rgb_from_profile(0, 0, 255, 0, &p).unwrap(),
        (0, 0, 0)
    );
    p.tags.clear();
    assert!(try_cmyk_to_rgb_from_profile(0, 0, 255, 255, &p).is_err());
    // Exact old fallback calls (c,m,y,k) into a (y,m,c,k) function. Do not
    // silently change this to the corrected LUT channel convention.
    assert_eq!(cmyk_to_rgb_from_profile(0, 0, 255, 255, &p), (255, 0, 0));
    assert_eq!(cmyk_to_rgb_from_profile(255, 0, 0, 255, &p), (0, 0, 255));
}

#[test]
fn legacy_metadata_admission_does_not_gain_new_fallback() {
    let mut forward = profile("A2B0", fixture(4, 3));
    let expected_forward = cmyk_to_rgb_from_profile(0, 0, 255, 255, &forward);
    let mut reverse = profile("B2A0", fixture(3, 4));
    let expected_reverse = rgb_to_cmyk_from_profile(0, 0, 0, &reverse);
    for p in [&mut forward, &mut reverse] {
        p.pcs = u32::from_be_bytes(*b"XYZ ");
        p.version = 0x05000000;
        p.device_class = u32::from_be_bytes(*b"link");
    }
    assert!(try_cmyk_to_rgb_from_profile(0, 0, 255, 255, &forward).is_err());
    assert!(try_rgb_to_cmyk_from_profile(0, 0, 0, &reverse).is_err());
    assert_eq!(
        cmyk_to_rgb_from_profile(0, 0, 255, 255, &forward),
        expected_forward
    );
    assert_eq!(
        rgb_to_cmyk_from_profile(0, 0, 0, &reverse),
        expected_reverse
    );
    assert_ne!(expected_forward, (255, 0, 0));
    assert_ne!(expected_reverse, (255, 255, 255, 255));
}

#[test]
fn reverse_short_clut_fallback_uses_only_historical_node_availability() {
    let mut p = profile("B2A0", fixture(3, 4));
    if let Some(Data::Lut8(lut)) = p.tags.get_mut("B2A0") {
        lut.clut_values.truncate(4);
    }
    // Black historically selects node0: present, but the full LUT is invalid.
    assert!(std::panic::catch_unwind(|| rgb_to_cmyk_from_profile(0, 0, 0, &p)).is_err());
    // White selects a later node, absent from the old nearest lookup.
    assert_eq!(rgb_to_cmyk_from_profile(255, 255, 255, &p), (0, 0, 0, 0));
    assert!(try_rgb_to_cmyk_from_profile(255, 255, 255, &p).is_err());
    if let Some(Data::Lut8(lut)) = p.tags.get_mut("B2A0") {
        lut.clut_values.truncate(3);
    }
    assert_eq!(rgb_to_cmyk_from_profile(0, 0, 0, &p), (255, 255, 255, 255));
}

#[test]
fn nonfinite_legacy_bridge_panics_before_actual_evaluation_not_fallback() {
    let mut p = profile("B2A0", fixture(3, 4));
    p.illuminate.y = S15Fixed16Number::new(0.0);
    assert!(try_rgb_to_cmyk_from_profile(0, 0, 0, &p).is_err());
    assert!(std::panic::catch_unwind(|| rgb_to_cmyk_from_profile(0, 0, 0, &p)).is_err());
    // Only historical missing-node fallback precedes finite/evaluator checks.
    if let Some(Data::Lut8(lut)) = p.tags.get_mut("B2A0") {
        lut.clut_values.clear();
    }
    assert_eq!(rgb_to_cmyk_from_profile(0, 0, 0, &p), (255, 255, 255, 255));
}

#[test]
fn legacy_absent_unsupported_and_non_cmyk_fallbacks_are_unchanged() {
    for case in 0..3 {
        let mut forward = profile("A2B0", fixture(4, 3));
        let mut reverse = profile("B2A0", fixture(3, 4));
        match case {
            0 => {
                forward.tags.clear();
                reverse.tags.clear();
            }
            1 => {
                forward
                    .tags
                    .insert("A2B0".into(), Data::Raw("mAB ".into(), vec![]));
                reverse
                    .tags
                    .insert("B2A0".into(), Data::Raw("mBA ".into(), vec![]));
            }
            _ => {
                forward.color_space = u32::from_be_bytes(*b"RGB ");
                reverse.color_space = forward.color_space;
            }
        }
        assert_eq!(
            cmyk_to_rgb_from_profile(0, 0, 255, 255, &forward),
            (255, 0, 0)
        );
        assert_eq!(
            rgb_to_cmyk_from_profile(0, 0, 0, &reverse),
            (255, 255, 255, 255)
        );
        assert!(try_cmyk_to_rgb_from_profile(0, 0, 255, 255, &forward).is_err());
        assert!(try_rgb_to_cmyk_from_profile(0, 0, 0, &reverse).is_err());
    }
}
