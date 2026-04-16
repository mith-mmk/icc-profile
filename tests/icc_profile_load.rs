/// Integration tests: ICC Profile ファイルのロードとヘッダー検証
///
/// 各プロファイルで確認する項目:
/// - ICCProfile::new() でロード成功すること
/// - DecodedICCProfile::new() でデコード成功すること
/// - magicnumber ("acsp" = 0x61637370) が正しいこと
/// - 主要ヘッダーフィールド (version / device_class / color_space / pcs) が期待値と一致すること
/// - tags に最低限の必須タグが存在すること

use icc_profile::iccprofile::{DecodedICCProfile, ICCProfile};

fn sample(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("_test_samples")
        .join(name)
}

fn load(name: &str) -> (ICCProfile, DecodedICCProfile) {
    let path = sample(name).to_str().unwrap().to_string();
    let icc = icc_profile::utils::load(path).expect("load failed");
    let decoded = DecodedICCProfile::new(&icc.data).expect("decode failed");
    (icc, decoded)
}

// ---------------------------------------------------------------------------
// ヘルパー: FourCC u32 → &[u8;4] → "XXXX"
// ---------------------------------------------------------------------------
fn fourcc(v: u32) -> String {
    let b = v.to_be_bytes();
    b.iter().map(|&c| if c.is_ascii_graphic() || c == b' ' { c as char } else { '.' }).collect()
}

// acsp マジックナンバー定数
const ACSP: u32 = 0x61637370;

// device_class FourCCs
const MNTR: u32 = 0x6d6e7472; // Display / Monitor
const PRTR: u32 = 0x70727472; // Output / Printer
const SPAC: u32 = 0x73706163; // Color Space

// color_space FourCCs
const RGB_: u32 = 0x52474220;
const CMYK: u32 = 0x434d594b;
const LAB_: u32 = 0x4c616220;

// PCS FourCCs
const XYZ_: u32 = 0x58595a20;

// ===========================================================================
// sRGB v4 ICC preference
// ===========================================================================

#[test]
fn srgb_v4_loads_successfully() {
    let (_icc, _decoded) = load("sRGB_v4_ICC_preference.icc");
}

#[test]
fn srgb_v4_magic_number() {
    let (icc, _) = load("sRGB_v4_ICC_preference.icc");
    assert_eq!(icc.magicnumber_ascp, ACSP, "magic must be 'acsp'");
}

#[test]
fn srgb_v4_version() {
    // version 4.20 → 0x04200000
    let (icc, _) = load("sRGB_v4_ICC_preference.icc");
    let major = (icc.version >> 24) & 0xFF;
    let minor = (icc.version >> 16) & 0xFF;
    assert_eq!(major, 4, "major version should be 4");
    assert_eq!(minor, 0x20, "minor version should be 0x20 (v4.2)");
}

#[test]
fn srgb_v4_device_class_and_colorspace() {
    let (icc, _) = load("sRGB_v4_ICC_preference.icc");
    assert_eq!(icc.device_class, SPAC, "device_class should be 'spac', got '{}'", fourcc(icc.device_class));
    assert_eq!(icc.color_space, RGB_, "color_space should be 'RGB ', got '{}'", fourcc(icc.color_space));
}

#[test]
fn srgb_v4_pcs_is_lab() {
    let (icc, _) = load("sRGB_v4_ICC_preference.icc");
    assert_eq!(icc.pcs, LAB_, "PCS should be 'Lab ', got '{}'", fourcc(icc.pcs));
}

#[test]
fn srgb_v4_has_required_tags() {
    let (_, decoded) = load("sRGB_v4_ICC_preference.icc");
    // sRGB color space profile に必須のタグ
    for tag in &["A2B0", "B2A0"] {
        assert!(decoded.tags.contains_key(*tag), "tag '{}' must exist", tag);
    }
}

#[test]
fn srgb_v4_data_length_matches() {
    let (icc, _) = load("sRGB_v4_ICC_preference.icc");
    assert_eq!(icc.length as usize, icc.data.len(), "length field must match raw data size");
}

// ===========================================================================
// ASUS ROG Strix XG309CM (display monitor profile, RGB, version 2)
// ===========================================================================

#[test]
fn asus_monitor_loads_successfully() {
    let (_icc, _decoded) = load("asus_rog_strix_xg309cm.icm");
}

#[test]
fn asus_monitor_magic_number() {
    let (icc, _) = load("asus_rog_strix_xg309cm.icm");
    assert_eq!(icc.magicnumber_ascp, ACSP);
}

#[test]
fn asus_monitor_version_2() {
    let (icc, _) = load("asus_rog_strix_xg309cm.icm");
    let major = (icc.version >> 24) & 0xFF;
    assert_eq!(major, 2, "version major should be 2");
}

#[test]
fn asus_monitor_device_class_is_mntr() {
    let (icc, _) = load("asus_rog_strix_xg309cm.icm");
    assert_eq!(icc.device_class, MNTR, "device_class should be 'mntr', got '{}'", fourcc(icc.device_class));
}

#[test]
fn asus_monitor_colorspace_rgb_pcs_xyz() {
    let (icc, _) = load("asus_rog_strix_xg309cm.icm");
    assert_eq!(icc.color_space, RGB_);
    assert_eq!(icc.pcs, XYZ_);
}

#[test]
fn asus_monitor_has_trc_and_xyz_tags() {
    let (_, decoded) = load("asus_rog_strix_xg309cm.icm");
    for tag in &["rXYZ", "gXYZ", "bXYZ", "rTRC", "gTRC", "bTRC", "wtpt", "chad"] {
        assert!(decoded.tags.contains_key(*tag), "tag '{}' must exist", tag);
    }
}

#[test]
fn asus_monitor_illuminate_approx_d50() {
    use icc_profile::iccprofile::ICCNumber;
    let (icc, _) = load("asus_rog_strix_xg309cm.icm");
    let y = icc.illuminate.y.as_f64();
    // D50 white point の Y は 1.0 に非常に近い
    assert!((y - 1.0).abs() < 0.01, "illuminate Y should be ~1.0, got {}", y);
}

// ===========================================================================
// JapanColor2011Coated (CMYK printer profile)
// ===========================================================================

#[test]
fn japan_color_loads_successfully() {
    let (_icc, _decoded) = load("JapanColor2011Coated.icc");
}

#[test]
fn japan_color_magic_number() {
    let (icc, _) = load("JapanColor2011Coated.icc");
    assert_eq!(icc.magicnumber_ascp, ACSP);
}

#[test]
fn japan_color_device_class_is_prtr() {
    let (icc, _) = load("JapanColor2011Coated.icc");
    assert_eq!(icc.device_class, PRTR, "device_class should be 'prtr', got '{}'", fourcc(icc.device_class));
}

#[test]
fn japan_color_colorspace_is_cmyk() {
    let (icc, _) = load("JapanColor2011Coated.icc");
    assert_eq!(icc.color_space, CMYK, "color_space should be 'CMYK', got '{}'", fourcc(icc.color_space));
}

#[test]
fn japan_color_has_a2b0_lut() {
    let (_, decoded) = load("JapanColor2011Coated.icc");
    assert!(decoded.tags.contains_key("A2B0"), "CMYK profile must have A2B0 LUT");
}

#[test]
fn japan_color_data_length_matches() {
    let (icc, _) = load("JapanColor2011Coated.icc");
    assert_eq!(icc.length as usize, icc.data.len());
}

// ===========================================================================
// YCCK profile
// ===========================================================================

#[test]
fn ycck_loads_successfully() {
    let (_icc, _decoded) = load("ycck.icc");
}

#[test]
fn ycck_magic_number() {
    let (icc, _) = load("ycck.icc");
    assert_eq!(icc.magicnumber_ascp, ACSP);
}

#[test]
fn ycck_is_cmyk_colorspace() {
    let (icc, _) = load("ycck.icc");
    assert_eq!(icc.color_space, CMYK, "ycck color_space should be CMYK, got '{}'", fourcc(icc.color_space));
}

// ===========================================================================
// sample1.icc / sample2.icc  (generic smoke tests)
// ===========================================================================

#[test]
fn sample1_loads_and_magic_ok() {
    let (icc, _) = load("sample1.icc");
    assert_eq!(icc.magicnumber_ascp, ACSP);
}

#[test]
fn sample1_data_length_matches() {
    let (icc, _) = load("sample1.icc");
    assert_eq!(icc.length as usize, icc.data.len());
}

#[test]
fn sample2_loads_and_magic_ok() {
    let (icc, _) = load("sample2.icc");
    assert_eq!(icc.magicnumber_ascp, ACSP);
}

#[test]
fn sample2_data_length_matches() {
    let (icc, _) = load("sample2.icc");
    assert_eq!(icc.length as usize, icc.data.len());
}

// ===========================================================================
// Spec400_10_700-IllumA-Abs_2deg.icc
// ===========================================================================

#[test]
fn spec400_loads_successfully() {
    let (_icc, _decoded) = load("Spec400_10_700-IllumA-Abs_2deg.icc");
}

#[test]
fn spec400_magic_number() {
    let (icc, _) = load("Spec400_10_700-IllumA-Abs_2deg.icc");
    assert_eq!(icc.magicnumber_ascp, ACSP);
}

#[test]
fn spec400_data_length_matches() {
    let (icc, _) = load("Spec400_10_700-IllumA-Abs_2deg.icc");
    assert_eq!(icc.length as usize, icc.data.len());
}

// ===========================================================================
// エラーケース
// ===========================================================================

#[test]
fn empty_data_returns_error() {
    let result = ICCProfile::new(&vec![]);
    assert!(result.is_err(), "empty data should return an error");
}

#[test]
fn truncated_header_returns_error() {
    // 127 バイト (128 バイト未満) → エラー
    let short = vec![0u8; 127];
    let result = ICCProfile::new(&short);
    assert!(result.is_err(), "data shorter than 128 bytes should return an error");
}

// ===========================================================================
// サンプルプロファイル詳細メタデータ検証
// ===========================================================================

#[test]
fn spec400_metadata_extraction() {
    let (icc, decoded) = load("Spec400_10_700-IllumA-Abs_2deg.icc");
    
    // device_class, color_space, pcs を確認
    println!("Spec400 metadata:");
    println!("  device_class: {} (0x{:08x})", fourcc(icc.device_class), icc.device_class);
    println!("  color_space:  {} (0x{:08x})", fourcc(icc.color_space), icc.color_space);
    println!("  pcs:          {} (0x{:08x})", fourcc(icc.pcs), icc.pcs);
    
    // ロード成功と基本的なタグ存在を確認
    assert!(icc.data.len() > 0);
    assert!(decoded.tags.len() > 0);
}

#[test]
fn sample1_metadata_extraction() {
    let (icc, decoded) = load("sample1.icc");
    
    println!("sample1.icc metadata:");
    println!("  device_class: {} (0x{:08x})", fourcc(icc.device_class), icc.device_class);
    println!("  color_space:  {} (0x{:08x})", fourcc(icc.color_space), icc.color_space);
    println!("  pcs:          {} (0x{:08x})", fourcc(icc.pcs), icc.pcs);
    println!("  tags count:   {}", decoded.tags.len());
    
    // ロード成功と基本的なタグ存在を確認
    assert!(icc.data.len() > 0);
    assert!(decoded.tags.len() > 0);
}

#[test]
fn sample2_metadata_extraction() {
    let (icc, decoded) = load("sample2.icc");
    
    println!("sample2.icc metadata:");
    println!("  device_class: {} (0x{:08x})", fourcc(icc.device_class), icc.device_class);
    println!("  color_space:  {} (0x{:08x})", fourcc(icc.color_space), icc.color_space);
    println!("  pcs:          {} (0x{:08x})", fourcc(icc.pcs), icc.pcs);
    println!("  tags count:   {}", decoded.tags.len());
    
    // ロード成功と基本的なタグ存在を確認
    assert!(icc.data.len() > 0);
    assert!(decoded.tags.len() > 0);
}
