//! Profile-free scalar regression tests. Reference values use the CIE 1976
//! inverse equations (delta=6/29), independently evaluated with rational
//! arithmetic, not output from these helpers or a round trip.
use icc_profile::cms::transration::{lab_to_xyz, lab_to_xyz_wp, xyz_to_lab_wp, WhitePoint};

fn close(actual: (f64, f64, f64), expected: (f64, f64, f64), tolerance: f64) {
    for (got, want) in [
        (actual.0, expected.0),
        (actual.1, expected.1),
        (actual.2, expected.2),
    ] {
        assert!(
            (got - want).abs() <= tolerance,
            "got {got:.16}, expected {want:.16}"
        );
    }
}

#[test]
fn dark_chromatic_lab_uses_cubic_x_and_z_independently() {
    close(
        lab_to_xyz(5.0, 100.0, -100.0),
        (0.0525774196072008, 0.00553528229939727, 0.343918405326172),
        1e-14,
    );
}

#[test]
fn bright_chromatic_lab_uses_linear_x_and_z_independently() {
    // L=50 gives fy=33/58; a=-200 and b=100 put fx/fz below delta.
    // Negative XYZ is deliberately not clamped by these mathematical helpers.
    let unit = WhitePoint::new(1.0, 1.0, 1.0);
    close(
        lab_to_xyz_wp(50.0, -200.0, 100.0, &unit),
        (486.0 / 121945.0, 35937.0 / 195112.0, -216.0 / 24389.0),
        1e-14,
    );
}

#[test]
fn each_component_crosses_six_over_twenty_nine_independently() {
    let wp = WhitePoint::new(1.0, 1.0, 1.0);
    // Choose f = 5/29, 6/29, 7/29, with exact inverse values.
    let cases = [
        (5.0 / 29.0, 108.0 / 24389.0),
        (6.0 / 29.0, 216.0 / 24389.0),
        (7.0 / 29.0, 343.0 / 24389.0),
    ];
    for &(fx, x) in &cases {
        for &(fy, y) in &cases {
            for &(fz, z) in &cases {
                close(
                    lab_to_xyz_wp(116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz), &wp),
                    (x, y, z),
                    1e-14,
                );
            }
        }
    }
}

#[test]
fn neutral_l_eight_boundary_is_continuous() {
    let wp = WhitePoint::new(1.0, 1.0, 1.0);
    let at = 216.0 / 24389.0;
    for l in [8.0 - 1e-8, 8.0, 8.0 + 1e-8] {
        close(lab_to_xyz_wp(l, 0.0, 0.0, &wp), (at, at, at), 2e-11);
    }
}

#[test]
fn standard_whitepoint_constants_and_neutral_conversions() {
    // CIE 1931 2-degree illuminants, rounded to four decimals. ICC PCS
    // uses its specified D50 encoding reference (Z=0.8249, not 0.8251).
    for (wp, xyz) in [
        (WhitePoint::d50(), (0.9642, 1.0, 0.8251)),
        (WhitePoint::d55(), (0.9568, 1.0, 0.9214)),
        (WhitePoint::icc(), (0.9642, 1.0, 0.8249)),
        (WhitePoint::d65(), (0.9504, 1.0, 1.0888)),
    ] {
        close((wp.x, wp.y, wp.z), xyz, 0.0);
        close(lab_to_xyz_wp(100.0, 0.0, 0.0, &wp), xyz, 1e-14);
        close(
            xyz_to_lab_wp(xyz.0, xyz.1, xyz.2, &wp),
            (100.0, 0.0, 0.0),
            1e-12,
        );
        close(lab_to_xyz_wp(0.0, 0.0, 0.0, &wp), (0.0, 0.0, 0.0), 1e-14);
    }
}

#[test]
fn chromatic_roundtrips_preserve_lab_for_all_standard_whites() {
    for wp in [
        WhitePoint::d50(),
        WhitePoint::d55(),
        WhitePoint::d65(),
        WhitePoint::icc(),
    ] {
        for lab in [
            (5.0, 100.0, -100.0),
            (50.0, -200.0, 100.0),
            (8.0, -127.0, 127.0),
            (80.0, 127.0, -127.0),
        ] {
            let xyz = lab_to_xyz_wp(lab.0, lab.1, lab.2, &wp);
            close(xyz_to_lab_wp(xyz.0, xyz.1, xyz.2, &wp), lab, 1e-10);
        }
    }
}
#[test]
fn lab_batch_helpers_reject_short_or_overflowing_counts_without_panicking() {
    use icc_profile::cms::transration::{lab_to_xyz_entries, lab_to_xyz_entries_f64};
    let wp = WhitePoint::d65();
    assert!(lab_to_xyz_entries(&[], 0, &wp).unwrap().is_empty());
    assert!(lab_to_xyz_entries_f64(&[], 0, &wp).unwrap().is_empty());
    for count in [1, usize::MAX / 3 + 1, usize::MAX] {
        assert!(lab_to_xyz_entries(&[0, 0], count, &wp).is_err());
        assert!(lab_to_xyz_entries_f64(&[0.0, 0.0], count, &wp).is_err());
    }
    let xyz = lab_to_xyz_entries_f64(&[5.0, 100.0, -100.0, 99.0], 1, &wp).unwrap();
    close(
        (xyz[0], xyz[1], xyz[2]),
        (0.0525774196072008, 0.00553528229939727, 0.343918405326172),
        1e-14,
    );
    assert_eq!(
        lab_to_xyz_entries(&[255, 128, 128, 17], 1, &wp).unwrap(),
        [242, 255, 255]
    );
}
