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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderingIntent {
    Perceptual,
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
}

impl Default for RenderingIntent {
    fn default() -> Self {
        Self::Perceptual
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
    data: Arc<[u8]>,
    length: usize,
    color_space: ColorSpace,
    pcs: Pcs,
    rendering_intent: RenderingIntent,
    pub(super) matrix: Option<MatrixProfile>,
    chad: Option<[[f32; 3]; 3]>,
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

    pub fn from_bytes_with_limits(
        data: &[u8],
        limits: ParseLimits,
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
        let mut tags = Vec::with_capacity(tag_count);
        for i in 0..tag_count {
            let p = 132 + i * 12;
            let sig = be_u32(profile_data, p)?;
            let off = be_u32(profile_data, p + 4)? as usize;
            let size = be_u32(profile_data, p + 8)? as usize;
            if size > limits.max_tag_size {
                return Err(TransformError::ResourceLimit("tag size"));
            }
            checked_range(profile_data, off, size)?;
            tags.push((sig, &profile_data[off..off + size]));
        }
        let chad = tags
            .iter()
            .find(|(s, _)| *s == u32::from_be_bytes(*b"chad"))
            .map(|(_, b)| parse_matrix_tag(b))
            .transpose()?;
        let matrix = if matches!(color_space, ColorSpace::Rgb | ColorSpace::Gray) {
            Some(parse_matrix_profile(color_space, pcs, &tags, limits)?)
        } else {
            None
        };
        Ok(Self(Arc::new(ProfileInner {
            data: Arc::from(data[..length].to_vec()),
            length,
            color_space,
            pcs,
            rendering_intent: intent,
            matrix,
            chad,
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
    pub fn bytes(&self) -> &[u8] {
        &self.0.data
    }
    pub fn chromatic_adaptation(&self) -> Option<[[f32; 3]; 3]> {
        self.0.chad
    }
}

fn parse_matrix_profile(
    space: ColorSpace,
    pcs: Pcs,
    tags: &[(u32, &[u8])],
    limits: ParseLimits,
) -> Result<MatrixProfile, TransformError> {
    if pcs != Pcs::Xyz {
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
                .find(|(s, _)| *s == u32::from_be_bytes(**name))
                .ok_or(TransformError::UnsupportedProfileFeature(
                    "matrix colorant tag",
                ))?;
            let v = parse_xyz(tag.1)?;
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
    let mut curves = Vec::with_capacity(channels);
    for name in names {
        let (_, data) = tags
            .iter()
            .find(|(s, _)| *s == *name)
            .ok_or(TransformError::UnsupportedProfileFeature("TRC tag"))?;
        curves.push(parse_curve(data, limits)?);
    }
    let inverse = if channels == 1 { None } else { invert(matrix) };
    Ok(MatrixProfile {
        pcs,
        matrix,
        inverse,
        curves,
    })
}

fn parse_xyz(data: &[u8]) -> Result<[f32; 3], TransformError> {
    if data.len() < 20 || super::reader::be_u32(data, 0)? != XYZ {
        return Err(TransformError::InvalidProfile("XYZ tag"));
    }
    Ok([
        be_i32(data, 8)? as f32 / 65536.0,
        be_i32(data, 12)? as f32 / 65536.0,
        be_i32(data, 16)? as f32 / 65536.0,
    ])
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
