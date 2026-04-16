/// Integration tests: 色空間変換関数の数値検証
///
/// テスト対象:
/// - RGB ↔ XYZ (sRGB D65 行列)
/// - XYZ ↔ L*a*b* (D65 白色点)
/// - YUV(YCbCr) ↔ RGB (BT.601)
/// - WhitePoint 各標準値

use icc_profile::cms::transration::{
    lab_to_xyz, lab_to_xyz_wp,
    rgb_to_xyz, rgb_to_xyz_from_f64,
    xyz_to_lab, xyz_to_lab_wp, xyz_to_rgb,
    yuv_to_rgb, yuv_to_rgb_with_mode, YUVToRGBCoefficient, WhitePoint,
};

// 浮動小数点の近似比較ヘルパー (絶対誤差)
fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn assert_approx(label: &str, got: f64, expected: f64, tol: f64) {
    assert!(
        approx(got, expected, tol),
        "{}: expected {:.6}, got {:.6} (tol={:.6})",
        label, expected, got, tol
    );
}

// ===========================================================================
// RGB → XYZ
// ===========================================================================

#[test]
fn rgb_black_to_xyz_is_zero() {
    let (x, y, z) = rgb_to_xyz(0, 0, 0);
    assert_approx("X", x, 0.0, 1e-9);
    assert_approx("Y", y, 0.0, 1e-9);
    assert_approx("Z", z, 0.0, 1e-9);
}

#[test]
fn rgb_white_to_xyz_approx_d65() {
    // sRGB D65 白点: XYZ ≈ (0.9505, 1.0000, 1.0888)  (値は線形RGB換算)
    let (x, y, z) = rgb_to_xyz(255, 255, 255);
    assert_approx("X", x, 0.9505, 0.01);
    assert_approx("Y", y, 1.0000, 0.01);
    assert_approx("Z", z, 1.0888, 0.02);
}

#[test]
fn rgb_red_to_xyz() {
    // sRGB primary red ≈ (0.4124, 0.2126, 0.0193)
    let (x, y, z) = rgb_to_xyz(255, 0, 0);
    assert_approx("X", x, 0.4124, 0.01);
    assert_approx("Y", y, 0.2126, 0.01);
    assert_approx("Z", z, 0.0193, 0.01);
}

#[test]
fn rgb_green_to_xyz() {
    // sRGB primary green ≈ (0.3576, 0.7152, 0.1192)
    let (x, y, z) = rgb_to_xyz(0, 255, 0);
    assert_approx("X", x, 0.3576, 0.01);
    assert_approx("Y", y, 0.7152, 0.01);
    assert_approx("Z", z, 0.1192, 0.01);
}

#[test]
fn rgb_blue_to_xyz() {
    // sRGB primary blue ≈ (0.1805, 0.0722, 0.9505)
    let (x, y, z) = rgb_to_xyz(0, 0, 255);
    assert_approx("X", x, 0.1805, 0.01);
    assert_approx("Y", y, 0.0722, 0.01);
    assert_approx("Z", z, 0.9505, 0.01);
}

// ===========================================================================
// XYZ → RGB (逆変換の整合性)
// ===========================================================================

#[test]
fn xyz_to_rgb_black() {
    let (r, g, b) = xyz_to_rgb(0.0, 0.0, 0.0);
    assert_eq!((r, g, b), (0, 0, 0));
}

#[test]
fn xyz_to_rgb_white_approx() {
    // D65 白点
    let (r, g, b) = xyz_to_rgb(0.9505, 1.0000, 1.0888);
    assert!(r >= 250, "R should be near 255, got {}", r);
    assert!(g >= 250, "G should be near 255, got {}", g);
    assert!(b >= 250, "B should be near 255, got {}", b);
}

#[test]
fn rgb_xyz_rgb_roundtrip() {
    // rgb_to_xyz(u8) → XYZ(0-1 範囲) → xyz_to_rgb(u8) がほぼ一致すること (量子化誤差 ±2)
    for &(r0, g0, b0) in &[(128u8, 64, 192u8), (200u8, 100, 0u8), (50u8, 200, 50u8)] {
        let (x, y, z) = rgb_to_xyz(r0, g0, b0);
        let (r1, g1, b1) = xyz_to_rgb(x, y, z);
        assert!((r0 as i32 - r1 as i32).abs() <= 2, "R roundtrip: {} → {}", r0, r1);
        assert!((g0 as i32 - g1 as i32).abs() <= 2, "G roundtrip: {} → {}", g0, g1);
        assert!((b0 as i32 - b1 as i32).abs() <= 2, "B roundtrip: {} → {}", b0, b1);
    }
}

#[test]
fn rgb_to_xyz_from_f64_normalized() {
    // rgb_to_xyz_from_f64 は 0.0-1.0 の入力を期待するので正規化して呪び出す
    let (x0, y0, z0) = rgb_to_xyz(128u8, 64, 192);
    let (x1, y1, z1) = rgb_to_xyz_from_f64(128.0 / 255.0, 64.0 / 255.0, 192.0 / 255.0);
    assert_approx("X", x1, x0, 1e-6);
    assert_approx("Y", y1, y0, 1e-6);
    assert_approx("Z", z1, z0, 1e-6);
}

// ===========================================================================
// XYZ → L*a*b*
// ===========================================================================

#[test]
fn xyz_black_to_lab() {
    let (l, a, b) = xyz_to_lab(0.0, 0.0, 0.0);
    assert_approx("L*", l, 0.0, 0.5);
    assert_approx("a*", a, 0.0, 0.5);
    assert_approx("b*", b, 0.0, 0.5);
}

#[test]
fn xyz_d65_white_to_lab() {
    // D65 白点 → L*=100, a*=0, b*=0
    let wp = WhitePoint::d65();
    let (l, a, b) = xyz_to_lab_wp(wp.x, wp.y, wp.z, &wp);
    assert_approx("L*", l, 100.0, 0.5);
    assert_approx("a*", a, 0.0, 1.0);
    assert_approx("b*", b, 0.0, 1.0);
}

#[test]
fn xyz_d50_white_to_lab_with_d50_wp() {
    let wp = WhitePoint::d50();
    let (l, a, b) = xyz_to_lab_wp(wp.x, wp.y, wp.z, &wp);
    assert_approx("L*", l, 100.0, 0.5);
    assert_approx("a*", a, 0.0, 1.0);
    assert_approx("b*", b, 0.0, 1.0);
}

// ===========================================================================
// L*a*b* → XYZ
// ===========================================================================

#[test]
fn lab_white_to_xyz_d65() {
    // L*=100, a*=0, b*=0 → D65 XYZ
    let wp = WhitePoint::d65();
    let (x, y, z) = lab_to_xyz_wp(100.0, 0.0, 0.0, &wp);
    assert_approx("X", x, wp.x, 0.01);
    assert_approx("Y", y, wp.y, 0.01);
    assert_approx("Z", z, wp.z, 0.01);
}

#[test]
fn lab_black_to_xyz() {
    let (x, y, z) = lab_to_xyz(0.0, 0.0, 0.0);
    assert_approx("X", x, 0.0, 0.001);
    assert_approx("Y", y, 0.0, 0.001);
    assert_approx("Z", z, 0.0, 0.001);
}

// ===========================================================================
// L*a*b* ↔ XYZ ラウンドトリップ
// ===========================================================================

#[test]
fn lab_xyz_lab_roundtrip() {
    let wp = WhitePoint::d65();
    for &(l0, a0, b0) in &[
        (50.0f64, 20.0, -30.0),
        (75.0, -10.0, 50.0),
        (30.0, 60.0, -60.0),
    ] {
        let (x, y, z) = lab_to_xyz_wp(l0, a0, b0, &wp);
        let (l1, a1, b1) = xyz_to_lab_wp(x, y, z, &wp);
        assert_approx(&format!("L* roundtrip for ({},{},{})", l0, a0, b0), l1, l0, 0.01);
        assert_approx(&format!("a* roundtrip for ({},{},{})", l0, a0, b0), a1, a0, 0.01);
        assert_approx(&format!("b* roundtrip for ({},{},{})", l0, a0, b0), b1, b0, 0.01);
    }
}

// ===========================================================================
// YUV(YCbCr) ↔ RGB (BT.601)
// Cb/Cr は 128 が中性点 (オフセット値)
// ===========================================================================

#[test]
fn yuv_black() {
    // Y=0, Cb=128, Cr=128 → 純粋な黒
    let (r, g, b) = yuv_to_rgb(0, 128, 128);
    assert_eq!((r, g, b), (0, 0, 0));
}

#[test]
fn yuv_white() {
    // Y=255, Cb=128, Cr=128 → 純粋な白
    let (r, g, b) = yuv_to_rgb(255, 128, 128);
    assert_eq!((r, g, b), (255, 255, 255));
}

#[test]
fn yuv_mid_gray() {
    // Y=128, Cb=128, Cr=128 → 中間グレー
    let (r, g, b) = yuv_to_rgb(128, 128, 128);
    assert_eq!((r, g, b), (128, 128, 128));
}

#[test]
fn yuv_rgb_roundtrip() {
    use icc_profile::cms::transration::{rgb_to_yuv, yuv_to_rgb};
    // RGB → YUV → RGB がほぼ一致すること (量子化誤差 ±1)
    for &(r0, g0, b0) in &[(200u8, 50, 80), (100u8, 200, 30), (128u8, 128, 128)] {
        let (y, cb, cr) = rgb_to_yuv(r0, g0, b0);
        let (r1, g1, b1) = yuv_to_rgb(y, cb, cr);
        assert!((r0 as i32 - r1 as i32).abs() <= 2, "R roundtrip: {} → {}", r0, r1);
        assert!((g0 as i32 - g1 as i32).abs() <= 2, "G roundtrip: {} → {}", g0, g1);
        assert!((b0 as i32 - b1 as i32).abs() <= 2, "B roundtrip: {} → {}", b0, b1);
    }
}

#[test]
fn yuv_consistency_bt601_vs_bt709() {
    // BT.601 と BT.709 は同じ入力で異なる出力を返すはず (crr: 1.402 vs 1.5748)
    let (r601, _, _) = yuv_to_rgb(100, 100, 200);
    let (r709, _, _) = yuv_to_rgb_with_mode(100, 100, 200, &YUVToRGBCoefficient::Bt709);
    // Cr=200 → offset後 72。crr の差 (0.1728) で ≥ 12 の差がある
    assert!((r601 as i32 - r709 as i32).abs() >= 10,
        "BT.601({}) and BT.709({}) should differ for non-neutral chroma", r601, r709);
}

// ===========================================================================
// WhitePoint 標準値の確認
// ===========================================================================

#[test]
fn whitepoint_d65_y_is_one() {
    let wp = WhitePoint::d65();
    assert_approx("D65 Y", wp.y, 1.0, 1e-9);
}

#[test]
fn whitepoint_d50_y_is_one() {
    let wp = WhitePoint::d50();
    assert_approx("D50 Y", wp.y, 1.0, 1e-9);
}

#[test]
fn whitepoint_a_y_is_one() {
    let wp = WhitePoint::a();
    assert_approx("A Y", wp.y, 1.0, 1e-9);
}

#[test]
fn whitepoint_d65_x_approx() {
    let wp = WhitePoint::d65();
    assert_approx("D65 X", wp.x, 0.9504, 0.005);
}

#[test]
fn whitepoint_d50_x_approx() {
    let wp = WhitePoint::d50();
    assert_approx("D50 X", wp.x, 0.9568, 0.005);
}
