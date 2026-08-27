//! Checked ICC profile parsing and the small, allocation-free matrix/TRC
//! transform used by the public CMS API.
//!
//! The legacy `iccprofile` module remains available for source compatibility.
//! This module intentionally owns its parsed representation so malformed tags
//! cannot make the older, diagnostic-oriented structures unsafe to use.

use std::fmt;
use std::sync::Arc;

const D50: [f32; 3] = [0.9642, 1.0, 0.8249];
const RGB: u32 = u32::from_be_bytes(*b"RGB ");
const GRAY: u32 = u32::from_be_bytes(*b"GRAY");
const XYZ: u32 = u32::from_be_bytes(*b"XYZ ");
const LAB: u32 = u32::from_be_bytes(*b"Lab ");
const CMYK: u32 = u32::from_be_bytes(*b"CMYK");

/// Limits applied before any allocation derived from an ICC profile.
#[derive(Clone, Copy, Debug)]
pub struct ParseLimits {
    pub max_profile_size: usize,
    pub max_tag_count: usize,
    pub max_tag_size: usize,
    pub max_curve_entries: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_profile_size: 64 * 1024 * 1024,
            max_tag_count: 4096,
            max_tag_size: 64 * 1024 * 1024,
            max_curve_entries: 1 << 20,
        }
    }
}

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
            // Matrix/TRC profiles have no intent-specific LUT or black point
            // stage. Relative colorimetric is therefore the only honest
            // default for this first implementation slice.
            rendering_intent: RenderingIntent::RelativeColorimetric,
            black_point_compensation: false,
            clamp: true,
        }
    }
}

#[derive(Debug)]
pub enum TransformError {
    InvalidProfile(&'static str),
    MalformedProfile(String),
    UnsupportedProfileFeature(&'static str),
    InvalidBufferLength { expected: usize, actual: usize },
    NonFiniteInput,
    ResourceLimit(&'static str),
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(s) => write!(f, "invalid ICC profile: {s}"),
            Self::MalformedProfile(s) => write!(f, "malformed ICC profile: {s}"),
            Self::UnsupportedProfileFeature(s) => write!(f, "unsupported ICC profile feature: {s}"),
            Self::InvalidBufferLength { expected, actual } => write!(
                f,
                "invalid buffer length: expected {expected}, got {actual}"
            ),
            Self::NonFiniteInput => f.write_str("transform input contains a non-finite value"),
            Self::ResourceLimit(s) => write!(f, "ICC resource limit exceeded: {s}"),
        }
    }
}

impl std::error::Error for TransformError {}

#[derive(Clone, Debug)]
enum Curve {
    Identity,
    Gamma(f32),
    Table(Vec<f32>),
    Para {
        function: u16,
        values: Vec<f32>,
        direction: i8,
    },
}

impl Curve {
    fn eval(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let value = match self {
            Self::Identity => x,
            Self::Gamma(g) => x.powf(*g),
            Self::Table(t) => {
                if t.is_empty() {
                    return x;
                }
                if t.len() == 1 {
                    return t[0];
                }
                let p = x * (t.len() - 1) as f32;
                let i = p.floor() as usize;
                let j = (i + 1).min(t.len() - 1);
                t[i] + (t[j] - t[i]) * (p - i as f32)
            }
            Self::Para {
                function, values, ..
            } => match *function {
                0 if !values.is_empty() => x.powf(values[0]),
                1 if values.len() >= 3 => {
                    let (g, a, b) = (values[0], values[1], values[2]);
                    if a != 0.0 && x >= -b / a {
                        (a * x + b).powf(g)
                    } else {
                        0.0
                    }
                }
                2 if values.len() >= 4 => {
                    let (g, a, b, c) = (values[0], values[1], values[2], values[3]);
                    if a != 0.0 && x >= -b / a {
                        (a * x + b).powf(g) + c
                    } else {
                        c
                    }
                }
                3 if values.len() >= 5 => {
                    let (g, a, b, c, d) = (values[0], values[1], values[2], values[3], values[4]);
                    if x >= d {
                        (a * x + b).powf(g)
                    } else {
                        c * x
                    }
                }
                4 if values.len() >= 7 => {
                    let (g, a, b, c, d, e, f) = (
                        values[0], values[1], values[2], values[3], values[4], values[5], values[6],
                    );
                    if x >= d {
                        (a * x + b).powf(g) + e
                    } else {
                        c * x + f
                    }
                }
                _ => x,
            },
        };
        value.clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug)]
struct MatrixProfile {
    pcs: Pcs,
    matrix: [[f32; 3]; 3],
    inverse: Option<[[f32; 3]; 3]>,
    curves: Vec<Curve>,
}

#[derive(Clone, Debug)]
struct ProfileInner {
    data: Arc<[u8]>,
    length: usize,
    color_space: ColorSpace,
    pcs: Pcs,
    rendering_intent: RenderingIntent,
    matrix: Option<MatrixProfile>,
    chad: Option<[[f32; 3]; 3]>,
}

/// An immutable, checked ICC profile. Cloning is cheap and is safe to share
/// across threads.
#[derive(Clone, Debug)]
pub struct Profile(Arc<ProfileInner>);

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
                let channels = (color_sig >> 24) as u8;
                let channels = if channels <= b'9' {
                    channels - b'0'
                } else {
                    channels - b'A' + 10
                };
                ColorSpace::NColor(channels)
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
        checked_range(&data[..length], 132, table_size)?;
        let profile_data = &data[..length];
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

/// Compiles two profiles into an immutable transform. The compiled object is
/// `Send + Sync`; use `TransformWorker` when a caller wants per-thread scratch.
#[derive(Clone, Debug)]
pub struct Transform {
    input: MatrixProfile,
    output: MatrixProfile,
    options: TransformOptions,
}

impl Transform {
    pub fn new(
        input: &Profile,
        output: &Profile,
        options: TransformOptions,
    ) -> Result<Self, TransformError> {
        if options.black_point_compensation {
            return Err(TransformError::UnsupportedProfileFeature(
                "black point compensation requires a profile black-point stage",
            ));
        }
        if options.rendering_intent != RenderingIntent::RelativeColorimetric {
            return Err(TransformError::UnsupportedProfileFeature(
                "matrix/TRC profiles do not provide intent-specific stages",
            ));
        }
        let input = input
            .0
            .matrix
            .clone()
            .ok_or(TransformError::UnsupportedProfileFeature(
                "input profile requires a LUT or CMYK adapter",
            ))?;
        let output = output
            .0
            .matrix
            .clone()
            .ok_or(TransformError::UnsupportedProfileFeature(
                "output profile requires a LUT or CMYK adapter",
            ))?;
        if input.pcs != output.pcs {
            return Err(TransformError::UnsupportedProfileFeature("mixed PCS"));
        }
        if input.curves.len() > 1 && input.inverse.is_none() {
            return Err(TransformError::InvalidProfile("input matrix is singular"));
        }
        if output.curves.len() > 1 && output.inverse.is_none() {
            return Err(TransformError::InvalidProfile("output matrix is singular"));
        }
        Ok(Self {
            input,
            output,
            options,
        })
    }

    pub fn worker(&self) -> TransformWorker {
        TransformWorker {
            transform: self.clone(),
            scratch: Vec::new(),
            output_scratch: Vec::new(),
        }
    }

    pub fn input_channels(&self) -> usize {
        self.input.curves.len()
    }
    pub fn output_channels(&self) -> usize {
        self.output.curves.len()
    }

    /// Transform normalized samples. Input and output are tightly packed
    /// pixels; all values are in the ICC normalized range [0, 1].
    pub fn transform_f32(&self, input: &[f32], output: &mut [f32]) -> Result<(), TransformError> {
        let ic = self.input_channels();
        let oc = self.output_channels();
        if input.len() % ic != 0 {
            return Err(TransformError::InvalidBufferLength {
                expected: ic,
                actual: input.len(),
            });
        }
        let pixels = input.len() / ic;
        let expected = pixels
            .checked_mul(oc)
            .ok_or(TransformError::ResourceLimit("output length"))?;
        if output.len() != expected {
            return Err(TransformError::InvalidBufferLength {
                expected,
                actual: output.len(),
            });
        }
        for (src, dst) in input.chunks_exact(ic).zip(output.chunks_exact_mut(oc)) {
            if src.iter().any(|x| !x.is_finite()) {
                return Err(TransformError::NonFiniteInput);
            }
            let xyz = device_to_xyz(&self.input, src);
            xyz_to_device(&self.output, xyz, dst, self.options.clamp)?;
        }
        Ok(())
    }

    pub fn transform_f32_vec(&self, input: &[f32]) -> Result<Vec<f32>, TransformError> {
        if input.len() % self.input_channels() != 0 {
            return Err(TransformError::InvalidBufferLength {
                expected: self.input_channels(),
                actual: input.len(),
            });
        }
        let mut result = vec![0.0; input.len() / self.input_channels() * self.output_channels()];
        self.transform_f32(input, &mut result)?;
        Ok(result)
    }

    pub fn transform_u8(&self, input: &[u8], output: &mut [u8]) -> Result<(), TransformError> {
        let values: Vec<f32> = input.iter().map(|x| *x as f32 / 255.0).collect();
        let mut converted = vec![0.0; output.len()];
        self.transform_f32(&values, &mut converted)?;
        for (d, x) in output.iter_mut().zip(converted) {
            *d = (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        Ok(())
    }

    pub fn transform_u16(&self, input: &[u16], output: &mut [u16]) -> Result<(), TransformError> {
        let values: Vec<f32> = input.iter().map(|x| *x as f32 / 65535.0).collect();
        let mut converted = vec![0.0; output.len()];
        self.transform_f32(&values, &mut converted)?;
        for (d, x) in output.iter_mut().zip(converted) {
            *d = (x.clamp(0.0, 1.0) * 65535.0).round() as u16;
        }
        Ok(())
    }
}

pub struct TransformWorker {
    transform: Transform,
    scratch: Vec<f32>,
    output_scratch: Vec<f32>,
}

impl TransformWorker {
    pub fn transform_f32(
        &mut self,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<(), TransformError> {
        self.scratch.clear();
        self.scratch.extend_from_slice(input);
        self.transform.transform_f32(&self.scratch, output)
    }
    pub fn transform_u8(&mut self, input: &[u8], output: &mut [u8]) -> Result<(), TransformError> {
        self.scratch.clear();
        self.scratch
            .extend(input.iter().map(|value| *value as f32 / 255.0));
        self.output_scratch.resize(output.len(), 0.0);
        self.transform
            .transform_f32(&self.scratch, &mut self.output_scratch)?;
        for (destination, value) in output.iter_mut().zip(&self.output_scratch) {
            *destination = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        Ok(())
    }
    pub fn transform_u16(
        &mut self,
        input: &[u16],
        output: &mut [u16],
    ) -> Result<(), TransformError> {
        self.scratch.clear();
        self.scratch
            .extend(input.iter().map(|value| *value as f32 / 65535.0));
        self.output_scratch.resize(output.len(), 0.0);
        self.transform
            .transform_f32(&self.scratch, &mut self.output_scratch)?;
        for (destination, value) in output.iter_mut().zip(&self.output_scratch) {
            *destination = (value.clamp(0.0, 1.0) * 65535.0).round() as u16;
        }
        Ok(())
    }
}

fn device_to_xyz(profile: &MatrixProfile, input: &[f32]) -> [f32; 3] {
    let mut v = [0.0; 3];
    for i in 0..3.min(input.len()) {
        v[i] = profile.curves[i].eval(input[i]);
    }
    let m = profile.matrix;
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn xyz_to_device(
    profile: &MatrixProfile,
    xyz: [f32; 3],
    output: &mut [f32],
    clamp: bool,
) -> Result<(), TransformError> {
    if profile.curves.len() == 1 {
        output[0] = inverse_curve(&profile.curves[0], (xyz[1] / D50[1]).clamp(0.0, 1.0));
        if clamp {
            output[0] = output[0].clamp(0.0, 1.0);
        }
        return Ok(());
    }
    let inv = profile
        .inverse
        .ok_or(TransformError::InvalidProfile("matrix is singular"))?;
    let linear = [
        inv[0][0] * xyz[0] + inv[0][1] * xyz[1] + inv[0][2] * xyz[2],
        inv[1][0] * xyz[0] + inv[1][1] * xyz[1] + inv[1][2] * xyz[2],
        inv[2][0] * xyz[0] + inv[2][1] * xyz[1] + inv[2][2] * xyz[2],
    ];
    for (i, value) in linear.iter().enumerate() {
        // ICC F.3 applies the PCS range clip before the inverse device curve.
        let value = inverse_curve(&profile.curves[i], value.clamp(0.0, 1.0));
        output[i] = if clamp { value.clamp(0.0, 1.0) } else { value };
    }
    Ok(())
}

fn inverse_curve(curve: &Curve, y: f32) -> f32 {
    match curve {
        Curve::Identity => y,
        Curve::Gamma(g) if *g != 0.0 => y.max(0.0).powf(1.0 / *g),
        Curve::Table(t) => {
            if t.len() < 2 {
                return y;
            }
            let increasing = t[0] <= *t.last().unwrap();
            let y = y.clamp(t[0].min(*t.last().unwrap()), t[0].max(*t.last().unwrap()));
            // Find the complete equal-value run before selecting its F.1
            // endpoint. This matters when y is reached after a sloped segment.
            let mut first_equal = None;
            let mut last_equal = None;
            for (index, value) in t.iter().enumerate() {
                if *value == y {
                    first_equal.get_or_insert(index);
                    last_equal = Some(index);
                }
            }
            if let (Some(first), Some(last)) = (first_equal, last_equal) {
                return (if last + 1 == t.len() { first } else { last }) as f32
                    / (t.len() - 1) as f32;
            }
            let mut i = 0;
            if increasing {
                while i + 1 < t.len() && t[i + 1] < y {
                    i += 1;
                }
            } else {
                while i + 1 < t.len() && t[i + 1] > y {
                    i += 1;
                }
            }
            let d = t[i + 1] - t[i];
            (i as f32 + (y - t[i]) / d) / (t.len() - 1) as f32
        }
        Curve::Para { direction, .. } => {
            let direction = *direction as f32;
            let target = y.clamp(0.0, 1.0) * direction;
            let mut lower_lo = 0.0;
            let mut lower_hi = 1.0;
            let mut upper_lo = 0.0;
            let mut upper_hi = 1.0;
            for _ in 0..32 {
                let lower_mid = (lower_lo + lower_hi) * 0.5;
                if curve.eval(lower_mid) * direction < target {
                    lower_lo = lower_mid;
                } else {
                    lower_hi = lower_mid;
                }
                let upper_mid = (upper_lo + upper_hi) * 0.5;
                if curve.eval(upper_mid) * direction <= target {
                    upper_lo = upper_mid;
                } else {
                    upper_hi = upper_mid;
                }
            }
            if curve.eval(1.0) * direction == target {
                // The equal-value run reaches the domain endpoint: choose
                // its first point. Otherwise choose the right edge of the
                // internal plateau, even when it is very close to 1.
                lower_hi
            } else {
                upper_lo
            }
        }
        Curve::Gamma(_) => y,
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
    let mut curves = Vec::with_capacity(channels);
    for name in if channels == 1 {
        vec![u32::from_be_bytes(*b"kTRC")]
    } else {
        vec![
            u32::from_be_bytes(*b"rTRC"),
            u32::from_be_bytes(*b"gTRC"),
            u32::from_be_bytes(*b"bTRC"),
        ]
    } {
        let (_, data) = tags
            .iter()
            .find(|(s, _)| *s == name)
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

fn parse_curve(data: &[u8], limits: ParseLimits) -> Result<Curve, TransformError> {
    if data.len() < 12 {
        return Err(TransformError::InvalidProfile("curve tag is truncated"));
    }
    match be_u32(data, 0)? {
        s if s == u32::from_be_bytes(*b"curv") => {
            let count = be_u32(data, 8)? as usize;
            if count > limits.max_curve_entries {
                return Err(TransformError::ResourceLimit("curve entries"));
            }
            if count == 0 {
                return Ok(Curve::Identity);
            }
            if count == 1 {
                let gamma = be_u16(data, 12)? as f32 / 256.0;
                if !gamma.is_finite() || gamma <= 0.0 {
                    return Err(TransformError::InvalidProfile("curve gamma"));
                }
                return Ok(Curve::Gamma(gamma));
            }
            checked_range(
                data,
                12,
                count
                    .checked_mul(2)
                    .ok_or(TransformError::ResourceLimit("curve arithmetic"))?,
            )?;
            let table = (0..count)
                .map(|i| be_u16(data, 12 + i * 2).map(|v| v as f32 / 65535.0))
                .collect::<Result<Vec<_>, _>>()?;
            let increasing = table.windows(2).all(|pair| pair[0] <= pair[1]);
            let decreasing = table.windows(2).all(|pair| pair[0] >= pair[1]);
            if table.first() == table.last() || (!increasing && !decreasing) {
                return Err(TransformError::InvalidProfile(
                    "sampled curve must be monotonic and non-constant",
                ));
            }
            Ok(Curve::Table(table))
        }
        s if s == u32::from_be_bytes(*b"para") => {
            let function = be_u16(data, 8)?;
            let count = match function {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => {
                    return Err(TransformError::UnsupportedProfileFeature(
                        "parametric curve function",
                    ))
                }
            };
            checked_range(data, 12, count * 4)?;
            let mut values = Vec::with_capacity(count);
            for i in 0..count {
                values.push(be_i32(data, 12 + i * 4)? as f32 / 65536.0);
            }
            if values.iter().any(|v| !v.is_finite()) || values[0] <= 0.0 {
                return Err(TransformError::InvalidProfile(
                    "parametric curve parameters",
                ));
            }
            if matches!(function, 1 | 2) && values[1] == 0.0 {
                return Err(TransformError::InvalidProfile(
                    "parametric curve has zero coefficient",
                ));
            }
            if matches!(function, 3 | 4) && !(0.0..=1.0).contains(&values[4]) {
                return Err(TransformError::InvalidProfile(
                    "parametric curve threshold is outside the domain",
                ));
            }
            let direction = validate_parametric_curve(function, &values)?;
            let curve = Curve::Para {
                function,
                values: values.clone(),
                direction,
            };
            for sample in [0.0, 1.0, values.get(4).copied().unwrap_or(0.0)] {
                if !curve.eval(sample).is_finite() {
                    return Err(TransformError::InvalidProfile(
                        "parametric curve has an invalid domain",
                    ));
                }
            }
            Ok(Curve::Para {
                function,
                values,
                direction,
            })
        }
        _ => Err(TransformError::UnsupportedProfileFeature("TRC type")),
    }
}

fn validate_parametric_curve(function: u16, values: &[f32]) -> Result<i8, TransformError> {
    let curve = Curve::Para {
        function,
        values: values.to_vec(),
        direction: 1,
    };
    let reject = || TransformError::UnsupportedProfileFeature("non-monotonic parametric curve");
    let sign = |value: f32| {
        if value > 0.0 {
            1
        } else if value < 0.0 {
            -1
        } else {
            0
        }
    };
    let mut direction = 0i8;
    let mut add = |value: f32| -> Result<(), TransformError> {
        let value = sign(value);
        if value != 0 {
            if direction != 0 && direction != value {
                return Err(reject());
            }
            direction = value;
        }
        Ok(())
    };
    match function {
        0 => add(curve.eval(1.0))?,
        1 | 2 => {
            let a = values[1];
            let b = values[2];
            let threshold = -b / a;
            let start = threshold.clamp(0.0, 1.0);
            if start >= 1.0 {
                return Err(reject());
            }
            let end = curve.eval(1.0);
            if !end.is_finite() {
                return Err(reject());
            }
            add(end - curve.eval(start))?;
        }
        3 | 4 => {
            let a = values[1];
            let b = values[2];
            let c = values[3];
            let d = values[4];
            let high_start = a * d + b;
            let high_end = a + b;
            if high_start < 0.0 || high_end < 0.0 {
                return Err(reject());
            }
            if d > 0.0 {
                let low_start = curve.eval(0.0);
                let low_end = if function == 3 {
                    (c * d).clamp(0.0, 1.0)
                } else {
                    (c * d + values[6]).clamp(0.0, 1.0)
                };
                add(low_end - low_start)?;
                add(curve.eval(d) - low_end)?;
            }
            if d < 1.0 {
                add(curve.eval(1.0) - curve.eval(d))?;
            }
        }
        _ => {
            return Err(TransformError::UnsupportedProfileFeature(
                "parametric curve function",
            ))
        }
    }
    if direction == 0 {
        Err(reject())
    } else {
        Ok(direction)
    }
}

fn parse_xyz(data: &[u8]) -> Result<[f32; 3], TransformError> {
    if data.len() < 20 || be_u32(data, 0)? != XYZ {
        return Err(TransformError::InvalidProfile("XYZ tag"));
    }
    Ok([
        be_i32(data, 8)? as f32 / 65536.0,
        be_i32(data, 12)? as f32 / 65536.0,
        be_i32(data, 16)? as f32 / 65536.0,
    ])
}

fn parse_matrix_tag(data: &[u8]) -> Result<[[f32; 3]; 3], TransformError> {
    if data.len() < 44 || be_u32(data, 0)? != u32::from_be_bytes(*b"sf32") {
        return Err(TransformError::InvalidProfile("chad tag"));
    }
    let mut m = [[0.0; 3]; 3];
    for i in 0..9 {
        m[i / 3][i % 3] = be_i32(data, 8 + i * 4)? as f32 / 65536.0;
    }
    Ok(m)
}

fn invert(m: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
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

fn checked_range(data: &[u8], offset: usize, size: usize) -> Result<(), TransformError> {
    let end = offset
        .checked_add(size)
        .ok_or(TransformError::ResourceLimit("offset arithmetic"))?;
    if end > data.len() {
        return Err(TransformError::InvalidProfile(
            "tag range is outside the profile",
        ));
    }
    Ok(())
}
fn be_u16(data: &[u8], p: usize) -> Result<u16, TransformError> {
    checked_range(data, p, 2)?;
    Ok(u16::from_be_bytes([data[p], data[p + 1]]))
}
fn be_u32(data: &[u8], p: usize) -> Result<u32, TransformError> {
    checked_range(data, p, 4)?;
    Ok(u32::from_be_bytes([
        data[p],
        data[p + 1],
        data[p + 2],
        data[p + 3],
    ]))
}
fn be_i32(data: &[u8], p: usize) -> Result<i32, TransformError> {
    Ok(be_u32(data, p)? as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_cover_parametric_functions() {
        for (function, values) in [
            (0, vec![2.0]),
            (1, vec![2.0, 1.0, 0.0]),
            (2, vec![2.0, 1.0, 0.0, 0.1]),
            (3, vec![2.0, 1.0, 0.0, 0.5, 0.1]),
            (4, vec![2.0, 1.0, 0.0, 0.1, 0.0, 0.5, 0.0]),
        ] {
            let c = Curve::Para {
                function,
                values,
                direction: 1,
            };
            assert!(c.eval(0.5).is_finite());
        }
    }

    #[test]
    fn matrix_inverse_roundtrip() {
        let m = [[0.4, 0.2, 0.1], [0.1, 0.7, 0.2], [0.0, 0.1, 0.9]];
        let inv = invert(m).unwrap();
        for row in 0..3 {
            for col in 0..3 {
                let got = (0..3).map(|k| m[row][k] * inv[k][col]).sum::<f32>();
                assert!((got - if row == col { 1.0 } else { 0.0 }).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn sampled_inverse_uses_complete_equal_runs() {
        let curve = Curve::Table(vec![0.0, 32768.0 / 65535.0, 32768.0 / 65535.0, 1.0]);
        assert!((inverse_curve(&curve, 32768.0 / 65535.0) - 2.0 / 3.0).abs() < 1e-7);
        let endpoint = Curve::Table(vec![0.0, 1.0, 1.0]);
        assert!((inverse_curve(&endpoint, 1.0) - 0.5).abs() < 1e-7);
    }

    #[test]
    fn parametric_inverse_preserves_near_endpoint_plateau() {
        let curve = Curve::Para {
            function: 1,
            values: vec![1.0, 32767.0, -32766.984375],
            direction: 1,
        };
        let expected = -curve_values(&curve)[2] / curve_values(&curve)[1];
        assert!((inverse_curve(&curve, 0.0) - expected).abs() < 1e-6);
    }

    fn curve_values(curve: &Curve) -> &[f32] {
        match curve {
            Curve::Para { values, .. } => values,
            _ => unreachable!(),
        }
    }
}
