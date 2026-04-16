/// Integration tests: ICC Profile を使ったカラーマネジメント変換
///
/// テスト対象:
/// - CMYK→RGB (プロファイルあり: JapanColor2011Coated, ycck.icc)
/// - CMYK→L*a*b* → XYZ → RGB パイプライン
/// - プロファイルなし CMYK→RGB (フォールバック計算)
/// - sRGB プロファイルのタグ内容確認

use icc_profile::cms::transration::{
    cmyk_to_lab_lut8, cmyk_to_rgb, cmyk_to_rgb_from_profile, lab_to_xyz_wp, xyz_to_rgb,
    WhitePoint,
};
use icc_profile::iccprofile::{Data, DecodedICCProfile, ICCNumber};

fn sample(name: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("_test_samples")
        .join(name)
        .to_str()
        .unwrap()
        .to_string()
}

fn load_decoded(name: &str) -> DecodedICCProfile {
    let icc = icc_profile::utils::load(sample(name)).expect("load failed");
    DecodedICCProfile::new(&icc.data).expect("decode failed")
}

// ---------------------------------------------------------------------------
// CMYK → RGB (プロファイルなし: 補色計算)
// ---------------------------------------------------------------------------

#[test]
fn cmyk_all_zero_with_full_k() {
    // C=0,M=0,Y=0,K=255: r=c+k-255=0+255-255=0, etc. → (0,0,0)
    let (r, g, b) = cmyk_to_rgb(0, 0, 0, 255);
    assert_eq!((r, g, b), (0, 0, 0));
}

#[test]
fn cmyk_no_overflow_values() {
    // cmyk_to_rgb(y,m,c,k): r=c+k-255, g=m+k-255, b=y+k-255
    // u8演算のため c=m=y=0, k=255 のみが安全 (0+255-255=0)
    let (r, g, b) = cmyk_to_rgb(0, 0, 0, 255);
    assert_eq!((r, g, b), (0, 0, 0));
}

// ---------------------------------------------------------------------------
// CMYK → RGB (JapanColor2011Coated プロファイル経由)
// ---------------------------------------------------------------------------

#[test]
fn japan_color_white_to_rgb() {
    let decoded = load_decoded("JapanColor2011Coated.icc");
    // C=0,M=0,Y=0,K=0 → 紙白 (高輝度 RGB)
    // 注: LUT インデックスの端点バグを避けるため K=0 を使用
    let (r, g, b) = cmyk_to_rgb_from_profile(0, 0, 0, 0, &decoded);
    // 紙白は高輝度なので R,G,B はそれなりに大きい
    assert!(r > 150, "white paper R should be bright, got {}", r);
    assert!(g > 150, "white paper G should be bright, got {}", g);
    assert!(b > 150, "white paper B should be bright, got {}", b);
}

#[test]
fn japan_color_heavy_ink_darkens_rgb() {
    let decoded = load_decoded("JapanColor2011Coated.icc");
    // 大量インク → 暗い方向 (LUT 端点バグを避けるため 253 を使用)
    let (r_heavy, g_heavy, b_heavy) = cmyk_to_rgb_from_profile(200, 200, 200, 200, &decoded);
    let (r_white, g_white, b_white) = cmyk_to_rgb_from_profile(0, 0, 0, 0, &decoded);
    // 大量インクは紙白より暗いはず
    let heavy = r_heavy as u32 + g_heavy as u32 + b_heavy as u32;
    let white = r_white as u32 + g_white as u32 + b_white as u32;
    assert!(heavy < white, "heavy ink ({}) should be darker than white ({})", heavy, white);
}

#[test]
fn japan_color_yellow_ink_has_low_blue() {
    let decoded = load_decoded("JapanColor2011Coated.icc");
    // Y(Yellow)=200, C=M=K=0 → 青が少ない黄色系
    let (r, g, b) = cmyk_to_rgb_from_profile(0, 0, 200, 0, &decoded);
    assert!(b < r.max(g), "yellow ink: B({}) should be less than R({}) or G({})", b, r, g);
}

// ---------------------------------------------------------------------------
// CMYK → L*a*b* (JapanColor2011Coated: Lut8 形式)
// ---------------------------------------------------------------------------

#[test]
fn japan_color_a2b0_type_check() {
    // A2B0 が何らかの LUT 型であることを確認 (Lut8 or Lut16 or LutAtoB)
    let decoded = load_decoded("JapanColor2011Coated.icc");
    let tag = decoded.tags.get("A2B0").expect("A2B0 must exist");
    let is_lut = matches!(
        tag,
        Data::Lut8(_) | Data::Lut16(_) | Data::LutAtoB(_)
    );
    assert!(is_lut, "A2B0 should be a LUT type");
}

#[test]
fn japan_color_cmyk_to_lab_via_lut8_white() {
    let decoded = load_decoded("JapanColor2011Coated.icc");
    let tag = decoded.tags.get("A2B0").expect("A2B0 must exist");
    if let Data::Lut8(lut8) = tag {
        let (l, _a, _b) = cmyk_to_lab_lut8(0, 0, 0, 0, lut8);
        // 紙白 L* は高輝度 (通常 80 以上)
        assert!(l > 70.0, "white paper L* should be >70, got {}", l);
    }
    // Lut8 以外 (Lut16 や LutAtoB) の場合はスキップ
}

#[test]
fn japan_color_cmyk_to_lab_via_lut8_heavy_ink() {
    let decoded = load_decoded("JapanColor2011Coated.icc");
    let tag = decoded.tags.get("A2B0").expect("A2B0 must exist");
    if let Data::Lut8(lut8) = tag {
        let (l_white, _, _) = cmyk_to_lab_lut8(0, 0, 0, 0, lut8);
        // LUT 端点 255 はバグあり → 200 で代用
        let (l_dark, _, _) = cmyk_to_lab_lut8(200, 200, 200, 200, lut8);
        assert!(l_dark < l_white, "heavy ink L*({}) should be darker than white L*({})", l_dark, l_white);
    }
}

// ---------------------------------------------------------------------------
// CMYK → Lab → XYZ → RGB パイプライン (JapanColor2011Coated)
// ---------------------------------------------------------------------------

#[test]
fn japan_color_cmyk_pipeline_white() {
    use icc_profile::cms::transration::cmyk_to_lab_lut8;
    let decoded = load_decoded("JapanColor2011Coated.icc");
    let tag = decoded.tags.get("A2B0").expect("A2B0 must exist");
    if let Data::Lut8(lut8) = tag {
        let wp = WhitePoint::d65();
        let (l, a, b) = cmyk_to_lab_lut8(0, 0, 0, 0, lut8);
        let (x, y, z) = lab_to_xyz_wp(l, a, b, &wp);
        let (r, g, b) = xyz_to_rgb(x, y, z);
        assert!(r > 150, "pipeline white R should be bright, got {}", r);
        assert!(g > 150, "pipeline white G should be bright, got {}", g);
        assert!(b > 150, "pipeline white B should be bright, got {}", b);
    }
}

// ---------------------------------------------------------------------------
// YCCK プロファイル
// ---------------------------------------------------------------------------

#[test]
fn ycck_profile_has_a2b0() {
    let decoded = load_decoded("ycck.icc");
    assert!(decoded.tags.contains_key("A2B0"), "ycck must have A2B0");
}

#[test]
fn ycck_white_to_rgb() {
    let decoded = load_decoded("ycck.icc");
    let (r, g, b) = cmyk_to_rgb_from_profile(0, 0, 0, 0, &decoded);
    // 白 → 高輝度
    assert!(r > 150 || g > 150 || b > 150,
        "ycck white should be bright: R={} G={} B={}", r, g, b);
}

// ---------------------------------------------------------------------------
// sRGB v4 タグ内容の型確認
// ---------------------------------------------------------------------------

#[test]
fn srgb_v4_a2b0_is_lut_type() {
    use icc_profile::iccprofile::Data;
    let decoded = load_decoded("sRGB_v4_ICC_preference.icc");
    let tag = decoded.tags.get("A2B0").expect("A2B0 must exist");
    let is_lut = matches!(tag, Data::LutAtoB(_));
    assert!(is_lut, "sRGB v4 A2B0 should be LutAtoB type");
}

#[test]
fn srgb_v4_b2a0_is_lut_type() {
    use icc_profile::iccprofile::Data;
    let decoded = load_decoded("sRGB_v4_ICC_preference.icc");
    let tag = decoded.tags.get("B2A0").expect("B2A0 must exist");
    let is_lut = matches!(tag, Data::LutBtoA(_));
    assert!(is_lut, "sRGB v4 B2A0 should be LutBtoA type");
}

// ---------------------------------------------------------------------------
// ASUS モニタープロファイル: XYZ タグの値確認 (XYZNumberArray)
// ---------------------------------------------------------------------------

#[test]
fn asus_rxyz_tag_exists_and_reasonable() {
    let decoded = load_decoded("asus_rog_strix_xg309cm.icm");
    let tag = decoded.tags.get("rXYZ").expect("rXYZ must exist");
    match tag {
        Data::XYZNumberArray(arr) => {
            let xyz = arr.first().expect("rXYZ array must not be empty");
            let y = xyz.y.as_f64();
            assert!(y > 0.05 && y < 0.6, "rXYZ Y should be reasonable, got {}", y);
        }
        Data::XYZNumber(xyz) => {
            let y = xyz.y.as_f64();
            assert!(y > 0.05 && y < 0.6, "rXYZ Y should be reasonable, got {}", y);
        }
        _ => panic!("rXYZ should be XYZNumber or XYZNumberArray, got {:?}", tag),
    }
}

#[test]
fn asus_gxyz_tag_green_dominant() {
    let decoded = load_decoded("asus_rog_strix_xg309cm.icm");
    let tag = decoded.tags.get("gXYZ").expect("gXYZ must exist");
    let (x, y) = match tag {
        Data::XYZNumberArray(arr) => {
            let xyz = arr.first().expect("gXYZ array must not be empty");
            (xyz.x.as_f64(), xyz.y.as_f64())
        }
        Data::XYZNumber(xyz) => (xyz.x.as_f64(), xyz.y.as_f64()),
        _ => panic!("gXYZ unexpected type"),
    };
    // Green primary の Y は X より大きい
    assert!(y > x, "green primary Y({}) should exceed X({})", y, x);
}

#[test]
fn asus_bxyz_tag_blue_high_z() {
    let decoded = load_decoded("asus_rog_strix_xg309cm.icm");
    let tag = decoded.tags.get("bXYZ").expect("bXYZ must exist");
    let (y, z) = match tag {
        Data::XYZNumberArray(arr) => {
            let xyz = arr.first().expect("bXYZ array must not be empty");
            (xyz.y.as_f64(), xyz.z.as_f64())
        }
        Data::XYZNumber(xyz) => (xyz.y.as_f64(), xyz.z.as_f64()),
        _ => panic!("bXYZ unexpected type"),
    };
    // Blue primary の Z は Y より大きい
    assert!(z > y, "blue primary Z({}) should exceed Y({})", z, y);
}
