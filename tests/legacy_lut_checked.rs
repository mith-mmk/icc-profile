use icc_profile::cms::transration::*;
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

fn invalid<T: std::fmt::Debug>(f: impl FnOnce() -> std::io::Result<T> + std::panic::UnwindSafe) {
    let result = std::panic::catch_unwind(f).expect("checked API must not unwind");
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn checked_d4_to_d3_lut8_valid_and_malformed() {
    let mut lut = fixture8(4, 3);
    lut.clut_values.fill(255);
    assert_eq!(
        try_d4_to_d3_lut8(255, 255, 255, 255, &lut).unwrap(),
        d4_to_d3_lut8(255, 255, 255, 255, &lut)
    );
    for change in 0..7 {
        let mut lut = fixture8(4, 3);
        match change {
            0 => lut.input_channels = 255,
            1 => lut.output_channels = 0,
            2 => lut.number_of_clut_grid_points = 0,
            3 => lut.number_of_clut_grid_points = 1,
            4 => {
                lut.input_table.pop();
            }
            5 => {
                lut.clut_values.pop();
            }
            6 => {
                lut.output_table.pop();
            }
            _ => unreachable!(),
        }
        invalid(|| try_d4_to_d3_lut8(255, 255, 255, 255, &lut));
        assert!(std::panic::catch_unwind(|| d4_to_d3_lut8(255, 255, 255, 255, &lut)).is_err());
    }
}

#[test]
fn checked_d3_to_d4_lut8_valid_and_malformed() {
    let mut lut = fixture8(3, 4);
    lut.clut_values.fill(255);
    assert_eq!(
        try_d3_to_d4_lut8(255, 255, 255, &lut).unwrap(),
        d3_to_d4_lut8(255, 255, 255, &lut)
    );
    for change in 0..7 {
        let mut lut = fixture8(3, 4);
        match change {
            0 => lut.input_channels = 255,
            1 => lut.output_channels = 0,
            2 => lut.number_of_clut_grid_points = 0,
            3 => lut.number_of_clut_grid_points = 1,
            4 => {
                lut.input_table.pop();
            }
            5 => {
                lut.clut_values.pop();
            }
            6 => {
                lut.output_table.pop();
            }
            _ => unreachable!(),
        }
        invalid(|| try_d3_to_d4_lut8(255, 255, 255, &lut));
        assert!(std::panic::catch_unwind(|| d3_to_d4_lut8(255, 255, 255, &lut)).is_err());
    }
    lut.e_params.truncate(8);
    invalid(|| try_d3_to_d4_lut8(255, 255, 255, &lut));
}

#[test]
fn checked_d3_to_d3_lut8_valid_and_malformed() {
    let mut lut = fixture8(3, 3);
    lut.clut_values.fill(255);
    assert_eq!(
        try_d3_to_d3_lut8(255, 255, 255, &lut).unwrap(),
        d3_to_d3_lut8(255, 255, 255, &lut)
    );
    for change in 0..7 {
        let mut lut = fixture8(3, 3);
        match change {
            0 => lut.input_channels = 255,
            1 => lut.output_channels = 0,
            2 => lut.number_of_clut_grid_points = 0,
            3 => lut.number_of_clut_grid_points = 1,
            4 => {
                lut.input_table.pop();
            }
            5 => {
                lut.clut_values.pop();
            }
            6 => {
                lut.output_table.pop();
            }
            _ => unreachable!(),
        }
        invalid(|| try_d3_to_d3_lut8(255, 255, 255, &lut));
        assert!(std::panic::catch_unwind(|| d3_to_d3_lut8(255, 255, 255, &lut)).is_err());
    }
    lut.e_params.truncate(8);
    invalid(|| try_d3_to_d3_lut8(255, 255, 255, &lut));
}

#[test]
fn checked_d3_to_d1_lut8_valid_and_malformed() {
    let mut lut = fixture8(3, 1);
    lut.clut_values.fill(255);
    assert_eq!(
        try_d3_to_d1_lut8(255, 255, 255, &lut).unwrap(),
        d3_to_d1_lut8(255, 255, 255, &lut)
    );
    for change in 0..7 {
        let mut lut = fixture8(3, 1);
        match change {
            0 => lut.input_channels = 255,
            1 => lut.output_channels = 0,
            2 => lut.number_of_clut_grid_points = 0,
            3 => lut.number_of_clut_grid_points = 1,
            4 => {
                lut.input_table.pop();
            }
            5 => {
                lut.clut_values.pop();
            }
            6 => {
                lut.output_table.pop();
            }
            _ => unreachable!(),
        }
        invalid(|| try_d3_to_d1_lut8(255, 255, 255, &lut));
        assert!(std::panic::catch_unwind(|| d3_to_d1_lut8(255, 255, 255, &lut)).is_err());
    }
    lut.e_params.truncate(8);
    invalid(|| try_d3_to_d1_lut8(255, 255, 255, &lut));
}

#[test]
fn checked_d4_to_d3_lut16_valid_and_malformed() {
    let mut lut = fixture16(4, 3, 2);
    lut.clut_values.fill(65535);
    assert_eq!(
        try_d4_to_d3_lut16(255, 255, 255, 255, &lut).unwrap(),
        d4_to_d3_lut16(255, 255, 255, 255, &lut)
    );
    for change in 0..9 {
        let mut lut = fixture16(4, 3, 2);
        match change {
            0 => lut.input_channels = 255,
            1 => lut.output_channels = 0,
            2 => lut.number_of_clut_grid_points = 0,
            3 => lut.number_of_clut_grid_points = 1,
            4 => {
                lut.input_table.pop();
            }
            5 => {
                lut.clut_values.pop();
            }
            6 => {
                lut.output_table.pop();
            }
            7 => lut.input_table_enteries = 1,
            8 => lut.output_table_enteries = 1,
            _ => unreachable!(),
        }
        invalid(|| try_d4_to_d3_lut16(255, 255, 255, 255, &lut));
        assert!(std::panic::catch_unwind(|| d4_to_d3_lut16(255, 255, 255, 255, &lut)).is_err());
    }
}

#[test]
fn checked_d3_to_d4_lut16_valid_and_malformed() {
    let mut lut = fixture16(3, 4, 2);
    lut.clut_values.fill(65535);
    assert_eq!(
        try_d3_to_d4_lut16(255, 255, 255, &lut).unwrap(),
        d3_to_d4_lut16(255, 255, 255, &lut)
    );
    for change in 0..9 {
        let mut lut = fixture16(3, 4, 2);
        match change {
            0 => lut.input_channels = 255,
            1 => lut.output_channels = 0,
            2 => lut.number_of_clut_grid_points = 0,
            3 => lut.number_of_clut_grid_points = 1,
            4 => {
                lut.input_table.pop();
            }
            5 => {
                lut.clut_values.pop();
            }
            6 => {
                lut.output_table.pop();
            }
            7 => lut.input_table_enteries = 1,
            8 => lut.output_table_enteries = 1,
            _ => unreachable!(),
        }
        invalid(|| try_d3_to_d4_lut16(255, 255, 255, &lut));
        assert!(std::panic::catch_unwind(|| d3_to_d4_lut16(255, 255, 255, &lut)).is_err());
    }
    lut.e_params.truncate(8);
    invalid(|| try_d3_to_d4_lut16(255, 255, 255, &lut));
}

#[test]
fn checked_d3_to_d3_lut16_valid_and_malformed() {
    let mut lut = fixture16(3, 3, 2);
    lut.clut_values.fill(65535);
    assert_eq!(
        try_d3_to_d3_lut16(255, 255, 255, &lut).unwrap(),
        d3_to_d3_lut16(255, 255, 255, &lut)
    );
    for change in 0..9 {
        let mut lut = fixture16(3, 3, 2);
        match change {
            0 => lut.input_channels = 255,
            1 => lut.output_channels = 0,
            2 => lut.number_of_clut_grid_points = 0,
            3 => lut.number_of_clut_grid_points = 1,
            4 => {
                lut.input_table.pop();
            }
            5 => {
                lut.clut_values.pop();
            }
            6 => {
                lut.output_table.pop();
            }
            7 => lut.input_table_enteries = 1,
            8 => lut.output_table_enteries = 1,
            _ => unreachable!(),
        }
        invalid(|| try_d3_to_d3_lut16(255, 255, 255, &lut));
        assert!(std::panic::catch_unwind(|| d3_to_d3_lut16(255, 255, 255, &lut)).is_err());
    }
    lut.e_params.truncate(8);
    invalid(|| try_d3_to_d3_lut16(255, 255, 255, &lut));
}

#[test]
fn checked_d3_to_d1_lut16_valid_and_malformed() {
    let mut lut = fixture16(3, 1, 2);
    lut.clut_values.fill(65535);
    assert_eq!(
        try_d3_to_d1_lut16(255, 255, 255, &lut).unwrap(),
        d3_to_d1_lut16(255, 255, 255, &lut)
    );
    for change in 0..9 {
        let mut lut = fixture16(3, 1, 2);
        match change {
            0 => lut.input_channels = 255,
            1 => lut.output_channels = 0,
            2 => lut.number_of_clut_grid_points = 0,
            3 => lut.number_of_clut_grid_points = 1,
            4 => {
                lut.input_table.pop();
            }
            5 => {
                lut.clut_values.pop();
            }
            6 => {
                lut.output_table.pop();
            }
            7 => lut.input_table_enteries = 1,
            8 => lut.output_table_enteries = 1,
            _ => unreachable!(),
        }
        invalid(|| try_d3_to_d1_lut16(255, 255, 255, &lut));
        assert!(std::panic::catch_unwind(|| d3_to_d1_lut16(255, 255, 255, &lut)).is_err());
    }
    lut.e_params.truncate(8);
    invalid(|| try_d3_to_d1_lut16(255, 255, 255, &lut));
}

#[test]
fn checked_cmyk_to_lab_lut16_u8_matches_and_rejects() {
    let mut lut = fixture16(4, 3, 2);
    lut.clut_values.fill(65535);
    assert_eq!(
        try_cmyk_to_lab_lut16_u8(255, 255, 255, 255, &lut).unwrap(),
        cmyk_to_lab_lut16_u8(255, 255, 255, 255, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_cmyk_to_lab_lut16_u8(255, 255, 255, 255, &lut));
}

#[test]
fn checked_cmyk_to_lab_lut16_matches_and_rejects() {
    let mut lut = fixture16(4, 3, 2);
    lut.clut_values.fill(65535);
    assert_eq!(
        try_cmyk_to_lab_lut16(255, 255, 255, 255, &lut).unwrap(),
        cmyk_to_lab_lut16(255, 255, 255, 255, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_cmyk_to_lab_lut16(255, 255, 255, 255, &lut));
}

#[test]
fn checked_cmyk_to_lab_lut8_matches_and_rejects() {
    let mut lut = fixture8(4, 3);
    lut.clut_values.fill(255);
    assert_eq!(
        try_cmyk_to_lab_lut8(255, 255, 255, 255, &lut).unwrap(),
        cmyk_to_lab_lut8(255, 255, 255, 255, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_cmyk_to_lab_lut8(255, 255, 255, 255, &lut));
}

#[test]
fn checked_cmyk_to_lab_lut8_u8_matches_and_rejects() {
    let mut lut = fixture8(4, 3);
    lut.clut_values.fill(255);
    assert_eq!(
        try_cmyk_to_lab_lut8_u8(255, 255, 255, 255, &lut).unwrap(),
        cmyk_to_lab_lut8_u8(255, 255, 255, 255, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_cmyk_to_lab_lut8_u8(255, 255, 255, 255, &lut));
}

#[test]
fn checked_lab_to_cmyk_lut16_u8_matches_and_rejects() {
    let mut lut = fixture16(3, 4, 2);
    lut.clut_values.fill(65535);
    assert_eq!(
        try_lab_to_cmyk_lut16_u8(255, 255, 255, &lut).unwrap(),
        lab_to_cmyk_lut16_u8(255, 255, 255, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_lab_to_cmyk_lut16_u8(255, 255, 255, &lut));
}

#[test]
fn checked_lab_to_cmyk_lut16_matches_and_rejects() {
    let mut lut = fixture16(3, 4, 2);
    lut.clut_values.fill(65535);
    assert_eq!(
        try_lab_to_cmyk_lut16(255, 255, 255, &lut).unwrap(),
        lab_to_cmyk_lut16(255, 255, 255, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_lab_to_cmyk_lut16(255, 255, 255, &lut));
}

#[test]
fn checked_lab_f64_to_cmyk_lut16_matches_and_rejects() {
    let mut lut = fixture16(3, 4, 2);
    lut.clut_values.fill(65535);
    assert_eq!(
        try_lab_f64_to_cmyk_lut16(50.0, 50.0, 50.0, &lut).unwrap(),
        lab_f64_to_cmyk_lut16(50.0, 50.0, 50.0, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_lab_f64_to_cmyk_lut16(50.0, 50.0, 50.0, &lut));
}

#[test]
fn checked_lab_to_cmyk_lut8_matches_and_rejects() {
    let mut lut = fixture8(3, 4);
    lut.clut_values.fill(255);
    assert_eq!(
        try_lab_to_cmyk_lut8(255, 255, 255, &lut).unwrap(),
        lab_to_cmyk_lut8(255, 255, 255, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_lab_to_cmyk_lut8(255, 255, 255, &lut));
}

#[test]
fn checked_lab_f64_to_cmyk_lut8_matches_and_rejects() {
    let mut lut = fixture8(3, 4);
    lut.clut_values.fill(255);
    assert_eq!(
        try_lab_f64_to_cmyk_lut8(50.0, 50.0, 50.0, &lut).unwrap(),
        lab_f64_to_cmyk_lut8(50.0, 50.0, 50.0, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_lab_f64_to_cmyk_lut8(50.0, 50.0, 50.0, &lut));
}

#[test]
fn checked_lab_to_cmyk_lut8_u8_matches_and_rejects() {
    let mut lut = fixture8(3, 4);
    lut.clut_values.fill(255);
    assert_eq!(
        try_lab_to_cmyk_lut8_u8(255, 255, 255, &lut).unwrap(),
        lab_to_cmyk_lut8_u8(255, 255, 255, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_lab_to_cmyk_lut8_u8(255, 255, 255, &lut));
}

#[test]
fn checked_lab_f64_to_cmyk_lut8_u8_matches_and_rejects() {
    let mut lut = fixture8(3, 4);
    lut.clut_values.fill(255);
    assert_eq!(
        try_lab_f64_to_cmyk_lut8_u8(50.0, 50.0, 50.0, &lut).unwrap(),
        lab_f64_to_cmyk_lut8_u8(50.0, 50.0, 50.0, &lut)
    );
    lut.clut_values.clear();
    invalid(|| try_lab_f64_to_cmyk_lut8_u8(50.0, 50.0, 50.0, &lut));
}

#[test]
fn checked_cmyk_to_rgb_lut16_matches_and_rejects() {
    let mut lut = fixture16(4, 3, 2);
    lut.clut_values.fill(65535);
    assert_eq!(
        try_cmyk_to_rgb_lut16(255, 255, 255, 255, &lut, &WhitePoint::d50()).unwrap(),
        cmyk_to_rgb_lut16(255, 255, 255, 255, &lut, &WhitePoint::d50())
    );
    lut.clut_values.clear();
    invalid(|| try_cmyk_to_rgb_lut16(255, 255, 255, 255, &lut, &WhitePoint::d50()));
}

#[test]
fn checked_cmyk_to_rgb_lut8_matches_and_rejects() {
    let mut lut = fixture8(4, 3);
    lut.clut_values.fill(255);
    assert_eq!(
        try_cmyk_to_rgb_lut8(255, 255, 255, 255, &lut, &WhitePoint::d50()).unwrap(),
        cmyk_to_rgb_lut8(255, 255, 255, 255, &lut, &WhitePoint::d50())
    );
    lut.clut_values.clear();
    invalid(|| try_cmyk_to_rgb_lut8(255, 255, 255, 255, &lut, &WhitePoint::d50()));
}

#[test]
fn float_inputs_and_whitepoints_are_checked() {
    let a = fixture8(3, 4);
    let b = fixture16(3, 4, 2);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for axis in 0..3 {
            let mut lab = [50.0, 0.0, 0.0];
            lab[axis] = value;
            invalid(|| try_lab_f64_to_cmyk_lut8(lab[0], lab[1], lab[2], &a));
            assert!(
                std::panic::catch_unwind(|| lab_f64_to_cmyk_lut8(lab[0], lab[1], lab[2], &a))
                    .is_err()
            );
            invalid(|| try_lab_f64_to_cmyk_lut8_u8(lab[0], lab[1], lab[2], &a));
            assert!(std::panic::catch_unwind(|| lab_f64_to_cmyk_lut8_u8(
                lab[0], lab[1], lab[2], &a
            ))
            .is_err());
            invalid(|| try_lab_f64_to_cmyk_lut16(lab[0], lab[1], lab[2], &b));
            assert!(
                std::panic::catch_unwind(|| lab_f64_to_cmyk_lut16(lab[0], lab[1], lab[2], &b))
                    .is_err()
            );
            invalid(|| lab_f64_to_cmyk_entries_lut8(&lab, 1, &a));
            invalid(|| lab_f64_to_cmyk_entries_lut8_u8(&lab, 1, &a));
        }
    }
    let a = fixture8(4, 3);
    let b = fixture16(4, 3, 2);
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for axis in 0..3 {
            let mut xyz = [1.0; 3];
            xyz[axis] = value;
            let wp = WhitePoint::new(xyz[0], xyz[1], xyz[2]);
            invalid(|| try_cmyk_to_rgb_lut8(0, 0, 0, 0, &a, &wp));
            assert!(std::panic::catch_unwind(|| cmyk_to_rgb_lut8(0, 0, 0, 0, &a, &wp)).is_err());
            invalid(|| try_cmyk_to_rgb_lut16(0, 0, 0, 0, &b, &wp));
            assert!(std::panic::catch_unwind(|| cmyk_to_rgb_lut16(0, 0, 0, 0, &b, &wp)).is_err());
        }
    }
}
#[test]
fn all_batches_return_errors_without_unwinding_and_keep_empty_compatibility() {
    macro_rules! check {
        ($f:ident, $buf:expr, $lut:expr) => {{
            let mut lut = $lut;
            invalid(|| $f(&[], 1, &lut));
            invalid(|| $f(&[], usize::MAX, &lut));
            lut.clut_values.clear();
            invalid(|| $f($buf, 1, &lut));
            assert!($f(&[], 0, &lut).unwrap().is_empty());
        }};
    }
    check!(cmyk_to_lab_entries_lut8, &[0; 4], fixture8(4, 3));
    check!(cmyk_to_lab_entries_lut16, &[0; 4], fixture16(4, 3, 2));
    check!(lab_to_cmyk_entries_lut8, &[0; 3], fixture8(3, 4));
    check!(lab_to_cmyk_entries_lut8_u8, &[0; 3], fixture8(3, 4));
    check!(lab_to_cmyk_entries_lut16, &[0; 3], fixture16(3, 4, 2));
    check!(lab_to_cmyk_entries_lut16_u8, &[0; 3], fixture16(3, 4, 2));
    check!(lab_f64_to_cmyk_entries_lut8, &[0.0; 3], fixture8(3, 4));
    check!(lab_f64_to_cmyk_entries_lut8_u8, &[0.0; 3], fixture8(3, 4));
}
#[test]
#[should_panic(expected = "Invalid legacy LUT conversion input")]
fn compatibility_wrapper_explicitly_panics() {
    let mut lut = fixture8(4, 3);
    lut.input_table.clear();
    cmyk_to_lab_lut8(0, 0, 0, 0, &lut);
}

#[test]
fn oversized_grid_and_declared_tables_return_errors() {
    let mut a = fixture8(4, 3);
    a.number_of_clut_grid_points = 255;
    invalid(|| try_d4_to_d3_lut8(255, 255, 255, 255, &a));
    let mut b = fixture16(4, 3, 2);
    b.number_of_clut_grid_points = 255;
    invalid(|| try_d4_to_d3_lut16(255, 255, 255, 255, &b));
    b.number_of_clut_grid_points = 2;
    b.input_table_enteries = u16::MAX;
    invalid(|| try_d4_to_d3_lut16(255, 255, 255, 255, &b));
    b.input_table_enteries = 2;
    b.output_table_enteries = u16::MAX;
    invalid(|| try_d4_to_d3_lut16(255, 255, 255, 255, &b));
}

#[test]
fn finite_out_of_range_lab_preserves_saturating_conversion() {
    let mut a = fixture8(3, 4);
    let mut b = fixture16(3, 4, 2);
    // Distinct corners expose each encoded coordinate instead of a constant LUT.
    for x in 0..2 {
        for y in 0..2 {
            for z in 0..2 {
                let offset = ((x * 2 + y) * 2 + z) * 4;
                a.clut_values[offset..offset + 4].copy_from_slice(&[
                    x as u8 * 255,
                    y as u8 * 255,
                    z as u8 * 255,
                    255,
                ]);
                b.clut_values[offset..offset + 4].copy_from_slice(&[
                    x as u16 * 65535,
                    y as u16 * 65535,
                    z as u16 * 65535,
                    65535,
                ]);
            }
        }
    }
    for lab in [
        [-1.0, -200.0, 200.0],
        [101.0, 200.0, -200.0],
        [f64::MAX, f64::MAX, -f64::MAX],
        [-f64::MAX, 0.0, 0.0],
    ] {
        let l8 = (lab[0] / 100.0 * 255.0 + 0.5) as u8;
        let l16 = (lab[0] / 100.0 * 255.0) as u8;
        let a_byte = (lab[1] + 127.5) as u8;
        let b_byte = (lab[2] + 127.5) as u8;
        let expected8 = lab_to_cmyk_lut8(l8, a_byte, b_byte, &a);
        let expected8_u8 = lab_to_cmyk_lut8_u8(l8, a_byte, b_byte, &a);
        let expected16 = lab_to_cmyk_lut16(l16, a_byte, b_byte, &b);
        assert_eq!(
            try_lab_f64_to_cmyk_lut8(lab[0], lab[1], lab[2], &a).unwrap(),
            expected8
        );
        assert_eq!(
            try_lab_f64_to_cmyk_lut8_u8(lab[0], lab[1], lab[2], &a).unwrap(),
            expected8_u8
        );
        assert_eq!(
            try_lab_f64_to_cmyk_lut16(lab[0], lab[1], lab[2], &b).unwrap(),
            expected16
        );
        assert_eq!(lab_f64_to_cmyk_lut8(lab[0], lab[1], lab[2], &a), expected8);
        assert_eq!(
            lab_f64_to_cmyk_lut8_u8(lab[0], lab[1], lab[2], &a),
            expected8_u8
        );
        assert_eq!(
            lab_f64_to_cmyk_lut16(lab[0], lab[1], lab[2], &b),
            expected16
        );
        assert_eq!(
            lab_f64_to_cmyk_entries_lut8(&lab, 1, &a).unwrap(),
            vec![expected8.0, expected8.1, expected8.2, expected8.3]
        );
        assert_eq!(
            lab_f64_to_cmyk_entries_lut8_u8(&lab, 1, &a).unwrap(),
            vec![
                expected8_u8.0,
                expected8_u8.1,
                expected8_u8.2,
                expected8_u8.3
            ]
        );
    }
}
