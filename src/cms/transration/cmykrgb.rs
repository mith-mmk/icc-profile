use crate::cms::transration::*;
use crate::Mft2;
use std::io::Result;

pub fn cmyk_to_rgb(y: u8, m: u8, c: u8, k: u8) -> (u8, u8, u8) {
    let r = (c as i16 + k as i16 - 255).clamp(0, 255) as u8;
    let g = (m as i16 + k as i16 - 255).clamp(0, 255) as u8;
    let b = (y as i16 + k as i16 - 255).clamp(0, 255) as u8;
    (r, g, b)
}

/// Compatibility wrapper for [`try_cmyk_to_rgb_lut16`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn cmyk_to_rgb_lut16(c: u8, m: u8, y: u8, k: u8, lut: &Mft2, wp: &WhitePoint) -> (u8, u8, u8) {
    try_cmyk_to_rgb_lut16(c, m, y, k, lut, wp).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_cmyk_to_rgb_lut16(
    c: u8,
    m: u8,
    y: u8,
    k: u8,
    lut: &Mft2,
    wp: &WhitePoint,
) -> Result<(u8, u8, u8)> {
    super::lut_convert::check_whitepoint(wp)?;
    let (l, a, b) = try_cmyk_to_lab_lut16(c, m, y, k, lut)?;
    let (x, y, z) = lab_to_xyz_wp(l, a, b, wp);
    checked_xyz_to_rgb(x, y, z)
}

/// Compatibility wrapper for [`try_cmyk_to_rgb_lut8`].
///
/// # Panics
/// Panics on invalid LUT payloads or invalid numeric inputs. Use the checked API to return errors.
pub fn cmyk_to_rgb_lut8(c: u8, m: u8, y: u8, k: u8, lut: &Mft1, wp: &WhitePoint) -> (u8, u8, u8) {
    try_cmyk_to_rgb_lut8(c, m, y, k, lut, wp).expect("Invalid legacy LUT conversion input")
}

/// Checked legacy conversion with unchanged encoded units and matrix semantics.
///
/// Returns [`std::io::ErrorKind::InvalidInput`] for malformed LUT payloads or invalid numeric inputs.
pub fn try_cmyk_to_rgb_lut8(
    c: u8,
    m: u8,
    y: u8,
    k: u8,
    lut: &Mft1,
    wp: &WhitePoint,
) -> Result<(u8, u8, u8)> {
    super::lut_convert::check_whitepoint(wp)?;
    let (l, a, b) = try_cmyk_to_lab_lut8(c, m, y, k, lut)?;
    let (x, y, z) = lab_to_xyz_wp(l, a, b, wp);
    checked_xyz_to_rgb(x, y, z)
}

// Preserve the legacy matrix and rounding, but reject overflow before integer casts.
fn checked_xyz_to_rgb(x: f64, y: f64, z: f64) -> Result<(u8, u8, u8)> {
    let invalid = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Non-finite RGB conversion intermediate",
        )
    };
    if ![x, y, z].iter().all(|v| v.is_finite()) {
        return Err(invalid());
    }
    let (r, g, b) = xyz_to_rgb_to_f64(x, y, z);
    if ![r, g, b].iter().all(|v| v.is_finite()) {
        return Err(invalid());
    }
    // Clip finite out-of-gamut values before scaling to avoid overflow in *255.
    // For finite values this equals the legacy scale/cast/clamp sequence.
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    Ok((byte(r), byte(g), byte(b)))
}

/// Checked **legacy approximation**, not an ICC color-managed transform.
///
/// Supports output-class CMYK profiles with Lab PCS, v2/v4 headers, a D50
/// header illuminant and an explicit A2B0 LUT8. CMYK byte order is C,M,Y,K.
/// The inherited bridge uses historical Lab byte coding
/// and linear RGB matrix, without sRGB transfer or D50/D65 adaptation. The
/// header illuminant is not the media white point (`wtpt`). LUT16 is rejected
/// because its legacy Lab encoding cannot be represented by this byte bridge.
/// Use original profile bytes with [`crate::Profile`] and [`crate::Transform`]
/// for checked color management. No unsupported or malformed input falls back.
pub fn try_cmyk_to_rgb_from_profile(
    c: u8,
    m: u8,
    y: u8,
    k: u8,
    decoded: &DecodedICCProfile,
) -> std::io::Result<(u8, u8, u8)> {
    check_legacy_profile(decoded)?;
    match decoded.tags.get("A2B0") {
        Some(Data::Lut8(lut)) => {
            try_cmyk_to_rgb_lut8(c, m, y, k, lut, &WhitePoint::from_profile(decoded))
        }
        _ => Err(unsupported_profile(
            "Checked legacy bridge requires explicit A2B0 LUT8",
        )),
    }
}

/// Compatibility-only approximate conversion. Prefer
/// [`try_cmyk_to_rgb_from_profile`] to observe unsupported/malformed profiles.
/// Retains the approximate LUT16 route and the historical profile-free fallback
/// on failure; neither is a color-managed sRGB conversion. The fallback calls
/// `cmyk_to_rgb(c,m,y,k)` despite that function's legacy `(y,m,c,k)` signature.
pub fn cmyk_to_rgb_from_profile(
    c: u8,
    m: u8,
    y: u8,
    k: u8,
    decoded: &DecodedICCProfile,
) -> (u8, u8, u8) {
    if check_legacy_profile(decoded).is_ok() {
        if let Some(Data::Lut16(lut)) = decoded.tags.get("A2B0") {
            if let Ok(rgb) =
                try_cmyk_to_rgb_lut16(c, m, y, k, lut, &WhitePoint::from_profile(decoded))
            {
                return rgb;
            }
        }
        if let Ok(rgb) = try_cmyk_to_rgb_from_profile(c, m, y, k, decoded) {
            return rgb;
        }
    }
    cmyk_to_rgb(c, m, y, k)
}

/// Checked **legacy approximation**, not an ICC color-managed transform.
///
/// Requires output-class CMYK, Lab PCS, v2/v4, a D50 header illuminant and an
/// explicit B2A0 LUT8. RGB bytes retain the old linear-matrix interpretation:
/// no sRGB decoding or chromatic adaptation is performed. The historical Lab
/// encoding uses -127..127 chroma and quantizes to bytes before LUT evaluation.
/// All LUT stages, including the legacy three-channel matrix, are evaluated;
/// that matrix behavior is not the ICC Lab-PCS matrix rule. These limitations
/// make this an approximation even for accepted profiles. No `wtpt` is read.
/// LUT16 and other routes return Unsupported; malformed LUT8 returns InvalidInput.
/// Use original bytes with [`crate::Profile`] and [`crate::Transform`] for color
/// management rather than assuming this checked approximation is ICC-accurate.
pub fn try_rgb_to_cmyk_from_profile(
    r: u8,
    g: u8,
    b: u8,
    decoded: &DecodedICCProfile,
) -> std::io::Result<(u8, u8, u8, u8)> {
    check_legacy_profile(decoded)?;
    match decoded.tags.get("B2A0") {
        Some(Data::Lut8(lut)) => {
            let (l, a, b) = legacy_rgb_lab(r, g, b, decoded);
            try_d3_to_d4_lut8(
                (l / 100.0 * 255.0).clamp(0.0, 255.0) as u8,
                ((a + 127.0) / 254.0 * 255.0).clamp(0.0, 255.0) as u8,
                ((b + 127.0) / 254.0 * 255.0).clamp(0.0, 255.0) as u8,
                lut,
            )
        }
        _ => Err(unsupported_profile(
            "Checked legacy bridge requires explicit B2A0 LUT8",
        )),
    }
}

/// Compatibility-only approximate conversion with the historical subtractive
/// fallback on unsupported/malformed profiles. Prefer
/// [`try_rgb_to_cmyk_from_profile`] for explicit failure. Valid LUT routes now
/// evaluate matrix/input tables/interpolated CLUT/output tables, so results can
/// differ from the old nearest-only implementation. The retained LUT16 route
/// also quantizes its input to bytes and does not implement ICC Lab16 coding.
pub fn rgb_to_cmyk_from_profile(
    r: u8,
    g: u8,
    b: u8,
    decoded: &DecodedICCProfile,
) -> (u8, u8, u8, u8) {
    if check_legacy_profile(decoded).is_ok() {
        if let Some(Data::Lut16(lut)) = decoded.tags.get("B2A0") {
            let (l, a, b) = legacy_rgb_lab(r, g, b, decoded);
            if let Ok((c, m, y, k)) = try_d3_to_d4_lut16(
                (l / 100.0 * 255.0).clamp(0.0, 255.0) as u8,
                ((a + 127.0) / 254.0 * 255.0).clamp(0.0, 255.0) as u8,
                ((b + 127.0) / 254.0 * 255.0).clamp(0.0, 255.0) as u8,
                lut,
            ) {
                return (
                    (c / 257.0).round() as u8,
                    (m / 257.0).round() as u8,
                    (y / 257.0).round() as u8,
                    (k / 257.0).round() as u8,
                );
            }
        }
        if let Ok(cmyk) = try_rgb_to_cmyk_from_profile(r, g, b, decoded) {
            return cmyk;
        }
    }
    (
        255 - r,
        255 - g,
        255 - b,
        255 - ((r as u16 + g as u16 + b as u16) / 3) as u8,
    )
}

fn unsupported_profile(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Unsupported, message)
}

fn check_legacy_profile(decoded: &DecodedICCProfile) -> std::io::Result<()> {
    use crate::ICCNumber;
    if decoded.color_space != u32::from_be_bytes(*b"CMYK")
        || decoded.pcs != u32::from_be_bytes(*b"Lab ")
        || decoded.device_class != u32::from_be_bytes(*b"prtr")
        || !matches!(decoded.version >> 24, 2 | 4)
    {
        return Err(unsupported_profile(
            "Legacy bridge requires v2/v4 CMYK output profile with Lab PCS",
        ));
    }
    let white = [
        decoded.illuminate.x.as_f64(),
        decoded.illuminate.y.as_f64(),
        decoded.illuminate.z.as_f64(),
    ];
    // Allow two fixed-point steps, including the historical decoder's 65535 divisor.
    if white
        .iter()
        .zip([0.9642, 1.0, 0.8249])
        .any(|(v, w)| (*v - w).abs() > 2.0 / 65535.0)
    {
        return Err(unsupported_profile(
            "Legacy bridge requires a D50 PCS header illuminant",
        ));
    }
    Ok(())
}

fn legacy_rgb_lab(r: u8, g: u8, b: u8, decoded: &DecodedICCProfile) -> (f64, f64, f64) {
    let (x, y, z) = rgb_to_xyz_from_f64(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    xyz_to_lab_wp(x, y, z, &WhitePoint::from_profile(decoded))
}
