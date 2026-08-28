use std::sync::Arc;

use super::curve::{parse_curve, Curve};
use super::error::TransformError;
use super::limits::ParseLimits;
use super::reader::{be_i32, be_u32, checked_range, CMYK, D50, GRAY, LAB, RGB, XYZ};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    Gray,
    Rgb,
    Cmyk,
    NColor(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pcs {
    Xyz,
    Lab,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderingIntent {
    #[default]
    Perceptual,
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
}

/// A direction-specific compiled route model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteModel {
    Matrix,
    Lut,
}

/// Immutable metadata describing the route selected for a transform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteInfo {
    requested_intent: RenderingIntent,
    selected_tag: Option<[u8; 4]>,
    model: RouteModel,
    used_fallback: bool,
}

impl RouteInfo {
    pub(super) const fn new(
        requested_intent: RenderingIntent,
        selected_tag: Option<[u8; 4]>,
        model: RouteModel,
        used_fallback: bool,
    ) -> Self {
        Self {
            requested_intent,
            selected_tag,
            model,
            used_fallback,
        }
    }

    pub fn requested_intent(self) -> RenderingIntent {
        self.requested_intent
    }

    pub fn selected_tag(self) -> Option<[u8; 4]> {
        self.selected_tag
    }

    pub fn model(self) -> RouteModel {
        self.model
    }

    /// Reports whether the designated route was unavailable and selection
    /// used the same-direction zero tag or a legal matrix model instead.
    pub fn used_fallback(self) -> bool {
        self.used_fallback
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TransformOptions {
    pub rendering_intent: RenderingIntent,
    pub black_point_compensation: bool,
    pub clamp: bool,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            rendering_intent: RenderingIntent::RelativeColorimetric,
            black_point_compensation: false,
            clamp: true,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct MatrixProfile {
    pub(super) pcs: Pcs,
    pub(super) matrix: [[f32; 3]; 3],
    pub(super) inverse: Option<[[f32; 3]; 3]>,
    pub(super) curves: Vec<Curve>,
}

#[derive(Clone, Debug)]
pub(super) struct ProfileInner {
    // The immutable ProfileInner is already Arc-owned. Keeping the Vec here
    // avoids a second full payload allocation when converting Vec into Arc<[u8]>.
    data: Vec<u8>,
    length: usize,
    raw_version: [u8; 4],
    raw_device_class: [u8; 4],
    color_space: ColorSpace,
    pcs: Pcs,
    rendering_intent: RenderingIntent,
    chad: Option<[[f32; 3]; 3]>,
    media_white: Option<[f32; 3]>,
    chad_range: Option<(usize, usize)>,
    media_white_range: Option<(usize, usize)>,
    pub(super) limits: ParseLimits,
    /// Tag ranges are retained as offsets into `data`.  Keeping ranges rather
    /// than copying tag payloads makes structural parsing cheap and ensures
    /// every compiler path uses the same checked profile boundary.
    pub(super) tags: Vec<(u32, usize, usize)>,
}

/// An immutable, checked ICC profile. Cloning is cheap and safe across threads.
#[derive(Clone, Debug)]
pub struct Profile(pub(super) Arc<ProfileInner>);

impl Profile {
    pub fn new(data: &[u8]) -> Result<Self, TransformError> {
        Self::from_bytes_with_limits(data, ParseLimits::default())
    }
    pub fn from_bytes(data: &[u8]) -> Result<Self, TransformError> {
        Self::new(data)
    }

    /// Parse the checked ICC container structure without compiling any
    /// direction-specific transform stages.  The returned profile owns the
    /// validated bytes and tag ranges; unsupported or non-invertible stages
    /// are diagnosed only when a direction is compiled.
    pub fn parse(data: &[u8]) -> Result<Self, TransformError> {
        Self::parse_with_limits(data, ParseLimits::default())
    }

    pub fn parse_with_limits(data: &[u8], limits: ParseLimits) -> Result<Self, TransformError> {
        Self::parse_impl(data, limits, false)
    }

    pub fn from_bytes_with_limits(
        data: &[u8],
        limits: ParseLimits,
    ) -> Result<Self, TransformError> {
        Self::parse_impl(data, limits, true)
    }

    fn parse_impl(
        data: &[u8],
        limits: ParseLimits,
        validate_semantics: bool,
    ) -> Result<Self, TransformError> {
        if data.len() > limits.max_profile_size {
            return Err(TransformError::ResourceLimit("profile size"));
        }
        if data.len() < 132 {
            return Err(TransformError::InvalidProfile(
                "header or tag table is truncated",
            ));
        }
        let length = be_u32(data, 0)? as usize;
        if length < 132 || length > data.len() {
            return Err(TransformError::InvalidProfile(
                "profile length is outside the input",
            ));
        }
        if be_u32(data, 36)? != u32::from_be_bytes(*b"acsp") {
            return Err(TransformError::InvalidProfile("missing acsp signature"));
        }
        let color_sig = be_u32(data, 16)?;
        let color_space = match color_sig {
            GRAY => ColorSpace::Gray,
            RGB => ColorSpace::Rgb,
            CMYK => ColorSpace::Cmyk,
            _ if (((color_sig >> 24) >= u32::from(b'2')
                && (color_sig >> 24) <= u32::from(b'9'))
                || ((color_sig >> 24) >= u32::from(b'A')
                    && (color_sig >> 24) <= u32::from(b'F')))
                && (color_sig & 0x00ff_ffff) == u32::from_be_bytes([0, b'C', b'L', b'R']) =>
            {
                let lead = (color_sig >> 24) as u8;
                ColorSpace::NColor(if lead <= b'9' {
                    lead - b'0'
                } else {
                    lead - b'A' + 10
                })
            }
            _ => return Err(TransformError::UnsupportedProfileFeature("color space")),
        };
        let pcs = match be_u32(data, 20)? {
            XYZ => Pcs::Xyz,
            LAB => Pcs::Lab,
            _ => return Err(TransformError::UnsupportedProfileFeature("PCS")),
        };
        let intent = match be_u32(data, 64)? {
            0 => RenderingIntent::Perceptual,
            1 => RenderingIntent::RelativeColorimetric,
            2 => RenderingIntent::Saturation,
            3 => RenderingIntent::AbsoluteColorimetric,
            _ => return Err(TransformError::InvalidProfile("rendering intent")),
        };
        let tag_count = be_u32(data, 128)? as usize;
        if tag_count > limits.max_tag_count {
            return Err(TransformError::ResourceLimit("tag count"));
        }
        let table_size = tag_count
            .checked_mul(12)
            .ok_or(TransformError::ResourceLimit("tag table arithmetic"))?;
        let profile_data = &data[..length];
        checked_range(profile_data, 132, table_size)?;
        let mut tags = Vec::new();
        tags.try_reserve_exact(tag_count)
            .map_err(|_| TransformError::ResourceLimit("tag table allocation"))?;
        for i in 0..tag_count {
            let p = 132 + i * 12;
            let sig = be_u32(profile_data, p)?;
            let off = be_u32(profile_data, p + 4)? as usize;
            let size = be_u32(profile_data, p + 8)? as usize;
            if size > limits.max_tag_size {
                return Err(TransformError::ResourceLimit("tag size"));
            }
            checked_range(profile_data, off, size)?;
            tags.push((sig, off, size));
        }
        let chad = if validate_semantics {
            tags.iter()
                .find(|(s, _, _)| *s == u32::from_be_bytes(*b"chad"))
                .map(|(_, off, size)| parse_matrix_tag(&profile_data[*off..*off + *size]))
                .transpose()?
        } else {
            None
        };
        let chad_range = tags
            .iter()
            .find(|(s, _, _)| *s == u32::from_be_bytes(*b"chad"))
            .map(|(_, off, size)| (*off, *size));
        let media_white = if validate_semantics {
            tags.iter()
                .find(|(s, _, _)| *s == u32::from_be_bytes(*b"wtpt"))
                .map(|(_, off, size)| parse_xyz_tag(&profile_data[*off..*off + *size]))
                .transpose()?
        } else {
            None
        };
        let media_white_range = tags
            .iter()
            .find(|(s, _, _)| *s == u32::from_be_bytes(*b"wtpt"))
            .map(|(_, off, size)| (*off, *size));
        if validate_semantics && matches!(color_space, ColorSpace::Rgb | ColorSpace::Gray) {
            let has_matrix = if color_space == ColorSpace::Gray {
                tags.iter()
                    .any(|(s, _, _)| *s == u32::from_be_bytes(*b"kTRC"))
            } else {
                [b"rXYZ", b"gXYZ", b"bXYZ", b"rTRC", b"gTRC", b"bTRC"]
                    .iter()
                    .all(|name| {
                        tags.iter()
                            .any(|(s, _, _)| *s == u32::from_be_bytes(**name))
                    })
            };
            if has_matrix {
                parse_matrix_profile(color_space, pcs, &tags, profile_data, limits, true)?;
            }
        }
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(length)
            .map_err(|_| TransformError::ResourceLimit("profile allocation"))?;
        owned.extend_from_slice(&data[..length]);
        Ok(Self(Arc::new(ProfileInner {
            data: owned,
            length,
            raw_version: data[8..12].try_into().expect("ICC version header"),
            raw_device_class: data[12..16].try_into().expect("ICC class header"),
            color_space,
            pcs,
            rendering_intent: intent,
            chad,
            media_white,
            chad_range,
            media_white_range,
            limits,
            tags,
        })))
    }

    pub fn color_space(&self) -> ColorSpace {
        self.0.color_space
    }
    pub fn pcs(&self) -> Pcs {
        self.0.pcs
    }
    pub fn rendering_intent(&self) -> RenderingIntent {
        self.0.rendering_intent
    }
    pub fn size(&self) -> usize {
        self.0.length
    }

    pub fn raw_version(&self) -> [u8; 4] {
        self.0.raw_version
    }

    pub fn raw_device_class(&self) -> [u8; 4] {
        self.0.raw_device_class
    }
    pub fn bytes(&self) -> &[u8] {
        &self.0.data
    }
    pub fn chromatic_adaptation(&self) -> Option<[[f32; 3]; 3]> {
        self.0.chad.or_else(|| {
            self.0.chad_range.and_then(|(offset, size)| {
                parse_matrix_tag(&self.0.data[offset..offset + size]).ok()
            })
        })
    }

    pub fn chromatic_adaptation_checked(&self) -> Result<Option<[[f32; 3]; 3]>, TransformError> {
        self.0
            .chad_range
            .map(|(offset, size)| parse_matrix_tag(&self.0.data[offset..offset + size]).map(Some))
            .unwrap_or(Ok(self.0.chad))
    }

    pub(super) fn media_white(&self) -> Option<[f32; 3]> {
        self.0.media_white.or_else(|| {
            self.0
                .media_white_range
                .and_then(|(offset, size)| parse_xyz_tag(&self.0.data[offset..offset + size]).ok())
        })
    }

    pub(super) fn media_white_checked(&self) -> Result<Option<[f32; 3]>, TransformError> {
        self.0
            .media_white_range
            .map(|(offset, size)| parse_xyz_tag(&self.0.data[offset..offset + size]).map(Some))
            .unwrap_or(Ok(self.0.media_white))
    }

    pub(super) fn tag(&self, signature: u32) -> Option<&[u8]> {
        self.0
            .tags
            .iter()
            .find(|(s, _, _)| *s == signature)
            .map(|(_, off, size)| &self.0.data[*off..*off + *size])
    }

    pub(super) fn limits(&self) -> ParseLimits {
        self.0.limits
    }
}

fn parse_matrix_profile(
    space: ColorSpace,
    pcs: Pcs,
    tags: &[(u32, usize, usize)],
    profile_data: &[u8],
    limits: ParseLimits,
    inverse_direction: bool,
) -> Result<MatrixProfile, TransformError> {
    if pcs != Pcs::Xyz && !(space == ColorSpace::Gray && pcs == Pcs::Lab) {
        return Err(TransformError::UnsupportedProfileFeature(
            "matrix/TRC profiles require XYZ PCS",
        ));
    }
    let channels = if space == ColorSpace::Gray { 1 } else { 3 };
    let matrix = if channels == 1 {
        [[D50[0], 0.0, 0.0], [D50[1], 0.0, 0.0], [D50[2], 0.0, 0.0]]
    } else {
        let mut m = [[0.0; 3]; 3];
        for (i, name) in [b"rXYZ", b"gXYZ", b"bXYZ"].iter().enumerate() {
            let tag = tags
                .iter()
                .find(|(s, _, _)| *s == u32::from_be_bytes(**name))
                .ok_or(TransformError::UnsupportedProfileFeature(
                    "matrix colorant tag",
                ))?;
            let v = parse_xyz_tag(&profile_data[tag.1..tag.1 + tag.2])?;
            m[0][i] = v[0];
            m[1][i] = v[1];
            m[2][i] = v[2];
        }
        m
    };
    let names: &[u32] = if channels == 1 {
        &[u32::from_be_bytes(*b"kTRC")]
    } else {
        &[
            u32::from_be_bytes(*b"rTRC"),
            u32::from_be_bytes(*b"gTRC"),
            u32::from_be_bytes(*b"bTRC"),
        ]
    };
    let mut curves = Vec::new();
    curves
        .try_reserve_exact(channels)
        .map_err(|_| TransformError::ResourceLimit("curve allocation"))?;
    for name in names {
        let (_, offset, size) = tags
            .iter()
            .find(|(s, _, _)| *s == *name)
            .ok_or(TransformError::UnsupportedProfileFeature("TRC tag"))?;
        let curve_data = &profile_data[*offset..*offset + *size];
        curves.push(if inverse_direction {
            parse_curve(curve_data, limits)?
        } else {
            super::curve::parse_curve_forward(curve_data, limits)?
        });
    }
    let inverse = if channels == 1 { None } else { invert(matrix) };
    Ok(MatrixProfile {
        pcs,
        matrix,
        inverse,
        curves,
    })
}

pub(super) fn parse_xyz_tag(data: &[u8]) -> Result<[f32; 3], TransformError> {
    if data.len() < 20 || super::reader::be_u32(data, 0)? != XYZ {
        return Err(TransformError::InvalidProfile("XYZ tag"));
    }
    Ok([
        be_i32(data, 8)? as f32 / 65536.0,
        be_i32(data, 12)? as f32 / 65536.0,
        be_i32(data, 16)? as f32 / 65536.0,
    ])
}

pub(super) fn invert_matrix(matrix: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    invert(matrix)
}

fn parse_matrix_tag(data: &[u8]) -> Result<[[f32; 3]; 3], TransformError> {
    if data.len() < 44 || super::reader::be_u32(data, 0)? != u32::from_be_bytes(*b"sf32") {
        return Err(TransformError::InvalidProfile("chad tag"));
    }
    let mut m = [[0.0; 3]; 3];
    for i in 0..9 {
        m[i / 3][i % 3] = be_i32(data, 8 + i * 4)? as f32 / 65536.0;
    }
    Ok(m)
}

pub(super) fn invert(m: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let d = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if d.abs() < 1e-12 {
        return None;
    }
    Some([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) / d,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) / d,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) / d,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) / d,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) / d,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) / d,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) / d,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) / d,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) / d,
        ],
    ])
}
