use icc_profile::cms::transration::*;
use icc_profile::iccprofile::ICCNumber;
use icc_profile::{Mft1, Mft2, S15Fixed16Number};
fn matrix() -> Vec<S15Fixed16Number> {
    (0..9)
        .map(|i| S15Fixed16Number::new(if i % 4 == 0 { 1.0 } else { 0.0 }))
        .collect()
}
fn fixture8(n: u8, m: u8) -> Mft1 {
    Mft1 {
        input_channels: n,
        output_channels: m,
        number_of_clut_grid_points: 2,
        e_params: matrix(),
        input_table: (0..n).flat_map(|_| 0..=255).collect(),
        clut_values: vec![0; (1 << n) * m as usize],
        output_table: (0..m).flat_map(|_| 0..=255).collect(),
    }
}
fn fixture16(n: u8, m: u8, entries: u16) -> Mft2 {
    Mft2 {
        input_channels: n,
        output_channels: m,
        number_of_clut_grid_points: 2,
        e_params: matrix(),
        input_table_enteries: entries,
        output_table_enteries: entries,
        input_table: (0..n)
            .flat_map(|_| (0..entries).map(|i| (i as u32 * 65535 / (entries - 1) as u32) as u16))
            .collect(),
        clut_values: vec![0; (1 << n) * m as usize],
        output_table: (0..m)
            .flat_map(|_| (0..entries).map(|i| (i as u32 * 65535 / (entries - 1) as u32) as u16))
            .collect(),
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
#[test]
fn endpoints_and_percent_units() {
    let a = fixture8(3, 4);
    assert_eq!(d3_to_d4_lut8(255, 255, 255, &a), (0, 0, 0, 0));
    let mut b = fixture16(3, 4, 256);
    b.clut_values.fill(65535);
    assert_eq!(
        lab_to_cmyk_lut16(255, 255, 255, &b),
        (100.0, 100.0, 100.0, 100.0)
    );
    assert_eq!(lab_to_cmyk_lut16_u8(0, 0, 0, &b), (255, 255, 255, 255));
}
#[test]
fn all_lab_batches_use_packed_triplets_and_check_lengths() {
    let a = fixture8(3, 4);
    let b = fixture16(3, 4, 256);
    macro_rules! check {
        ($f:ident,$buf:expr,$lut:expr) => {{
            assert_eq!($f($buf, 2, $lut).unwrap().len(), 8);
            assert!($f(&$buf[..5], 2, $lut).is_err());
            assert!($f(&[], usize::MAX, $lut).is_err());
            assert!($f(&[], 0, $lut).unwrap().is_empty());
        }};
    }
    check!(lab_to_cmyk_entries_lut8, &[0; 6], &a);
    check!(lab_to_cmyk_entries_lut8_u8, &[0; 6], &a);
    check!(lab_to_cmyk_entries_lut16, &[0; 6], &b);
    check!(lab_to_cmyk_entries_lut16_u8, &[0; 6], &b);
    check!(lab_f64_to_cmyk_entries_lut8, &[0.0; 6], &a);
    check!(lab_f64_to_cmyk_entries_lut8_u8, &[0.0; 6], &a);
}
#[test]
fn three_axes_and_fourth_output_are_independent() {
    for entries in [2, 4, 256] {
        let mut lut = fixture16(3, 4, entries);
        // Analytic field (x,y,z,1-x). Trilinear interpolation reproduces affine fields exactly.
        for x in 0..2 {
            for y in 0..2 {
                for z in 0..2 {
                    let p = ((x * 2 + y) * 2 + z) * 4;
                    lut.clut_values[p..p + 4].copy_from_slice(&[
                        x as u16 * 65535,
                        y as u16 * 65535,
                        z as u16 * 65535,
                        (1 - x) as u16 * 65535,
                    ]);
                }
            }
        }
        for (x, y, z) in [(0, 255, 0), (255, 0, 255), (37, 129, 203), (255, 255, 255)] {
            let v = d3_to_d4_lut16(x, y, z, &lut);
            close(v.0, x as f64 * 257.0);
            close(v.1, y as f64 * 257.0);
            close(v.2, z as f64 * 257.0);
            close(v.3, (255 - x) as f64 * 257.0);
        }
    }
    let mut lut = fixture8(3, 3);
    for x in 0..2 {
        for y in 0..2 {
            for z in 0..2 {
                let p = ((x * 2 + y) * 2 + z) * 3;
                lut.clut_values[p..p + 3].copy_from_slice(&[
                    x as u8 * 255,
                    y as u8 * 255,
                    z as u8 * 255,
                ]);
            }
        }
    }
    assert_eq!(d3_to_d3_lut8(37, 129, 203, &lut), (37, 129, 203));
}
#[test]
fn four_axes_include_all_sixteen_corners_without_cyan_yellow_swap() {
    let mut a = fixture8(4, 3);
    let mut b = fixture16(4, 3, 2);
    // Multi-affine field (C, Y, M*K). Product verifies mixed-axis corner weights.
    for c in 0..2 {
        for m in 0..2 {
            for y in 0..2 {
                for k in 0..2 {
                    let p = (((c * 2 + m) * 2 + y) * 2 + k) * 3;
                    a.clut_values[p..p + 3].copy_from_slice(&[
                        c as u8 * 255,
                        y as u8 * 255,
                        (m * k) as u8 * 255,
                    ]);
                    b.clut_values[p..p + 3].copy_from_slice(&[
                        c as u16 * 65535,
                        y as u16 * 65535,
                        (m * k) as u16 * 65535,
                    ]);
                }
            }
        }
    }
    assert_eq!(d4_to_d3_lut8(31, 85, 211, 153, &a), (31, 211, 51));
    let v = d4_to_d3_lut16(31, 85, 211, 153, &b);
    close(v.0, 31.0 * 257.0);
    close(v.1, 211.0 * 257.0);
    close(v.2, 65535.0 / 5.0);
    assert_eq!(cmyk_to_lab_lut8_u8(31, 85, 211, 153, &a), (31, 211, 51));
    let wp = WhitePoint::d65();
    let lab = cmyk_to_lab_lut8(31, 85, 211, 153, &a);
    let xyz = lab_to_xyz_wp(lab.0, lab.1, lab.2, &wp);
    assert_eq!(
        cmyk_to_rgb_lut8(31, 85, 211, 153, &a, &wp),
        xyz_to_rgb(xyz.0, xyz.1, xyz.2)
    );
    let lab = cmyk_to_lab_lut16(31, 85, 211, 153, &b);
    let xyz = lab_to_xyz_wp(lab.0, lab.1, lab.2, &wp);
    assert_eq!(
        cmyk_to_rgb_lut16(31, 85, 211, 153, &b, &wp),
        xyz_to_rgb(xyz.0, xyz.1, xyz.2)
    );
    assert!(cmyk_to_lab_entries_lut8(&[], usize::MAX, &a).is_err());
    assert!(cmyk_to_lab_entries_lut16(&[], usize::MAX, &b).is_err());
}
#[test]
fn nonlinear_output_tables_interpolate_values_not_indices() {
    let mut lut = fixture16(3, 1, 4);
    lut.clut_values.fill(32768);
    lut.output_table = vec![60000, 40000, 10000, 0];
    // 32768 lies just above the midpoint between knots 21845 and 43690.
    close(
        d3_to_d1_lut16(17, 91, 255, &lut),
        40000.0 - 30000.0 * (32768.0 - 21845.0) / 21845.0,
    );
    let mut a = fixture8(3, 1);
    a.clut_values.fill(255);
    assert_eq!(d3_to_d1_lut8(255, 255, 255, &a), 255);
}
#[test]
fn malformed_public_payloads_are_explicit_not_silent_fallbacks() {
    for kind in 0..6 {
        let mut a = fixture8(3, 4);
        let mut b = fixture16(3, 4, 2);
        match kind {
            0 => {
                a.input_channels = 4;
                b.input_channels = 4
            }
            1 => {
                a.number_of_clut_grid_points = 0;
                b.number_of_clut_grid_points = 1
            }
            2 => {
                a.e_params.clear();
                b.e_params.clear()
            }
            3 => {
                a.input_table.clear();
                b.input_table_enteries = 0
            }
            4 => {
                a.clut_values.pop();
                b.clut_values.pop();
            }
            _ => {
                a.output_table.clear();
                b.output_table.clear();
            }
        }
        assert!(std::panic::catch_unwind(|| d3_to_d4_lut8(0, 0, 0, &a)).is_err());
        assert!(std::panic::catch_unwind(|| d3_to_d4_lut16(0, 0, 0, &b)).is_err());
        assert!(lab_to_cmyk_entries_lut8(&[0; 3], 1, &a).is_err());
        assert!(lab_to_cmyk_entries_lut16(&[0; 3], 1, &b).is_err());
    }
}
#[test]
fn packed_batches_preserve_distinct_pixels() {
    let mut a = fixture8(3, 4);
    let mut b = fixture16(3, 4, 2);
    for x in 0..2 {
        for y in 0..2 {
            for z in 0..2 {
                let p = ((x * 2 + y) * 2 + z) * 4;
                a.clut_values[p..p + 4].copy_from_slice(&[
                    x as u8 * 255,
                    y as u8 * 255,
                    z as u8 * 255,
                    255,
                ]);
                b.clut_values[p..p + 4].copy_from_slice(&[
                    x as u16 * 65535,
                    y as u16 * 65535,
                    z as u16 * 65535,
                    65535,
                ]);
            }
        }
    }
    let bytes = [0, 255, 0, 255, 0, 255];
    let expected = vec![0, 255, 0, 255, 255, 0, 255, 255];
    assert_eq!(
        lab_to_cmyk_entries_lut8_u8(&bytes, 2, &a).unwrap(),
        expected
    );
    assert_eq!(
        lab_to_cmyk_entries_lut16_u8(&bytes, 2, &b).unwrap(),
        expected
    );
    let percent = vec![0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0, 100.0];
    assert_eq!(lab_to_cmyk_entries_lut8(&bytes, 2, &a).unwrap(), percent);
    assert_eq!(lab_to_cmyk_entries_lut16(&bytes, 2, &b).unwrap(), percent);
    let labs = [0.0, 127.5, -127.5, 100.0, -127.5, 127.5];
    assert_eq!(
        lab_f64_to_cmyk_entries_lut8_u8(&labs, 2, &a).unwrap(),
        expected
    );
    assert_eq!(lab_f64_to_cmyk_entries_lut8(&labs, 2, &a).unwrap(), percent);
}
#[test]
fn input_curve_and_matrix_clipping_are_applied_before_clut() {
    let mut b = fixture16(3, 3, 4);
    b.input_table[..4].copy_from_slice(&[0, 1000, 50000, 65535]);
    for x in 0..2 {
        for y in 0..2 {
            for z in 0..2 {
                let p = ((x * 2 + y) * 2 + z) * 3;
                b.clut_values[p..p + 3].copy_from_slice(&[
                    x as u16 * 65535,
                    y as u16 * 65535,
                    z as u16 * 65535,
                ]);
            }
        }
    }
    close(
        d3_to_d3_lut16(128, 0, 0, &b).0,
        1000.0 + 49000.0 * (128.0 / 85.0 - 1.0),
    );
    b.e_params[0] = S15Fixed16Number::new(2.0);
    b.e_params[4] = S15Fixed16Number::new(-1.0);
    assert_eq!(d3_to_d3_lut16(255, 255, 0, &b), (65535.0, 0.0, 0.0));
    let mut a = fixture8(3, 1);
    a.e_params[0] = S15Fixed16Number::new(2.0);
    a.clut_values[4..].fill(255);
    assert_eq!(d3_to_d1_lut8(255, 0, 0, &a), 255);
}
#[test]
fn four_point_grid_checks_both_sides_of_interior_cell_boundaries() {
    let mut lut = fixture16(3, 4, 2);
    lut.number_of_clut_grid_points = 4;
    lut.clut_values.clear();
    // First output has slopes 9000, 30000, 21000 across its three cells.
    // Other outputs isolate Y/Z strides and a mixed X*Y field.
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                lut.clut_values.extend_from_slice(&[
                    [0, 9000, 39000, 60000][x],
                    y as u16 * 21845,
                    z as u16 * 21845,
                    (x * y * 7000) as u16,
                ]);
            }
        }
    }
    for x in [0, 84, 85, 86, 169, 170, 171, 255] {
        let expected = if x <= 85 {
            x as f64 * 9000.0 / 85.0
        } else if x <= 170 {
            9000.0 + (x - 85) as f64 * 30000.0 / 85.0
        } else {
            39000.0 + (x - 170) as f64 * 21000.0 / 85.0
        };
        for y in [84, 85, 86, 169, 170, 171] {
            let z = 255 - y;
            let actual = d3_to_d4_lut16(x, y, z, &lut);
            close(actual.0, expected);
            close(actual.1, y as f64 * 257.0);
            close(actual.2, z as f64 * 257.0);
            close(actual.3, 7000.0 * x as f64 * y as f64 / (85.0 * 85.0));
        }
    }
}

#[test]
fn lut8_interpolates_nonlinear_output_curve_before_rounding() {
    let mut lut = fixture8(3, 1);
    // At x=128 the CLUT field is 128/255, between output knots 0 and 1.
    lut.clut_values[4..].fill(1);
    lut.output_table.fill(0);
    lut.output_table[1] = 200;
    lut.output_table[2] = 10;
    // 200*128/255 = 100.392..., rounds to 100. Truncating the CLUT first yields 0.
    assert_eq!(d3_to_d1_lut8(128, 73, 201, &lut), 100);
    assert_eq!(d3_to_d1_lut8(0, 73, 201, &lut), 0);
    assert_eq!(d3_to_d1_lut8(255, 73, 201, &lut), 200);
}

#[test]
fn fractional_off_diagonal_matrix_preserves_axis_mixing() {
    let mut lut = fixture16(3, 3, 2);
    // Test LUT matrix application using the coefficients represented by the
    // public numeric type. Raw fixed-point encoding/decoding correctness belongs
    // to numeric-type tests, not this independent matrix/interpolation oracle.
    lut.e_params = [0.5, 0.25, 0.0, 0.0, 0.5, 0.25, 0.25, 0.0, 0.5]
        .into_iter()
        .map(S15Fixed16Number::new)
        .collect();
    for x in 0..2 {
        for y in 0..2 {
            for z in 0..2 {
                let p = ((x * 2 + y) * 2 + z) * 3;
                lut.clut_values[p..p + 3].copy_from_slice(&[
                    x as u16 * 65535,
                    y as u16 * 65535,
                    z as u16 * 65535,
                ]);
            }
        }
    }
    let actual = d3_to_d3_lut16(40, 120, 200, &lut);
    let e: Vec<f64> = lut.e_params.iter().map(ICCNumber::as_f64).collect();
    // The identity CLUT/curves leave the analytic matrix result unchanged.
    // Write out the three dot products without calling the LUT matrix evaluator.
    close(
        actual.0,
        (e[0] * 40.0 + e[1] * 120.0 + e[2] * 200.0) * 257.0,
    );
    close(
        actual.1,
        (e[3] * 40.0 + e[4] * 120.0 + e[5] * 200.0) * 257.0,
    );
    close(
        actual.2,
        (e[6] * 40.0 + e[7] * 120.0 + e[8] * 200.0) * 257.0,
    );
}
