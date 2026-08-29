use super::compile_budget::CompileBudget;
use super::curve::inverse_curve_checked;
use super::error::TransformError;
use super::execution::{checked_output_allocation, checked_output_len, try_new_f32_output};
use super::limits::{ExecutionLimits, TransformLimits};
use super::lut::{LutTransform, PcsEncoding};
use super::profile::{MatrixProfile, Pcs, Profile, RenderingIntent, RouteInfo, TransformOptions};
use super::reader::D50;
use super::route_plan::{admit_pair, plan_route, plan_route_with_policy, OwnerPolicy};
use std::sync::Arc;

const REFERENCE_BLACK_XYZ: [f32; 3] = [0.003357, 0.003479, 0.002869];
const REFERENCE_WHITE_XYZ: [f32; 3] = [0.9642, 1.0, 0.8249];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReversePcsConnection {
    V2MatrixToV4B2A0ReferenceBlack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformDirection {
    DeviceToPcs,
    PcsToDevice,
}

#[derive(Clone, Debug)]
pub(super) struct CompiledDirection {
    pub(super) direction: TransformDirection,
    pub(super) matrix: Option<Arc<MatrixProfile>>,
    pub(super) lut: Option<Arc<LutTransform>>,
    pub(super) pcs: Pcs,
    pub(super) input_channels: usize,
    pub(super) output_channels: usize,
    pub(super) route_info: RouteInfo,
    pub(super) raw_version: [u8; 4],
    pub(super) raw_device_class: [u8; 4],
}

/// An immutable, direction-specific transform stage.  Large curve and CLUT
/// payloads are shared by clones and are not copied when a worker is created.
#[derive(Clone, Debug)]
pub struct CompiledProfile(pub(super) Arc<CompiledDirection>);

impl Profile {
    pub fn compile(
        &self,
        direction: TransformDirection,
        intent: RenderingIntent,
        limits: TransformLimits,
    ) -> Result<CompiledProfile, TransformError> {
        CompiledProfile::compile(self, direction, intent, limits)
    }
}

impl CompiledProfile {
    fn compile(
        profile: &Profile,
        direction: TransformDirection,
        intent: RenderingIntent,
        limits: TransformLimits,
    ) -> Result<Self, TransformError> {
        let route = plan_route(profile, direction, intent, limits)?;
        let mut budget = CompileBudget::new(limits);
        route.admit(&mut budget)?;
        let checkpoint = budget.checkpoint();
        let stage = route.materialize(&mut budget, profile.limits())?;
        if let Err(error) = validate_compiled_direction(&stage) {
            drop(stage);
            budget.rollback(checkpoint);
            return Err(error);
        }
        Ok(Self(Arc::new(stage)))
    }

    pub fn direction(&self) -> TransformDirection {
        self.0.direction
    }

    pub fn input_channels(&self) -> usize {
        self.0.input_channels
    }

    pub fn output_channels(&self) -> usize {
        self.0.output_channels
    }

    pub fn route_info(&self) -> RouteInfo {
        self.0.route_info
    }

    pub fn raw_version(&self) -> [u8; 4] {
        self.0.raw_version
    }

    pub fn raw_device_class(&self) -> [u8; 4] {
        self.0.raw_device_class
    }

    /// Evaluate the compiled stage at its explicit physical D50 XYZ boundary.
    /// Device samples are normalized and must be in `[0, 1]`; no implicit
    /// clipping or HDR-unit reinterpretation occurs here.
    pub fn transform_f32(&self, input: &[f32], output: &mut [f32]) -> Result<(), TransformError> {
        if input.len() != self.0.input_channels || output.len() != self.0.output_channels {
            return Err(TransformError::InvalidBufferLength {
                expected: self.0.input_channels,
                actual: input.len(),
            });
        }
        if input.iter().any(|value| !value.is_finite()) {
            return Err(TransformError::NonFiniteInput);
        }
        if self.0.direction == TransformDirection::DeviceToPcs
            && input.iter().any(|value| !(0.0..=1.0).contains(value))
        {
            return Err(TransformError::MalformedProfile(
                "device sample is outside the normalized domain".into(),
            ));
        }
        match self.0.direction {
            TransformDirection::DeviceToPcs => {
                if let Some(lut) = &self.0.lut {
                    let mut encoded = [0.0; 3];
                    lut.eval_with_domain(input, &mut encoded, false)?;
                    decode_lut_pcs(&mut encoded, lut.pcs_encoding())?;
                    convert_pcs(&mut encoded, self.0.pcs, Pcs::Xyz)?;
                    output.copy_from_slice(&encoded);
                } else {
                    output.copy_from_slice(&device_to_xyz(
                        self.0.matrix.as_ref().expect("matrix stage"),
                        input,
                        false,
                    ));
                }
            }
            TransformDirection::PcsToDevice => {
                if let Some(lut) = &self.0.lut {
                    let mut encoded = [input[0], input[1], input[2]];
                    convert_pcs(&mut encoded, Pcs::Xyz, self.0.pcs)?;
                    encode_lut_pcs(&mut encoded, lut.pcs_encoding())?;
                    lut.eval_with_domain(&encoded, output, false)?;
                } else {
                    xyz_to_device(
                        self.0.matrix.as_ref().expect("matrix stage"),
                        [input[0], input[1], input[2]],
                        output,
                        false,
                    )?;
                }
            }
        }
        if output.iter().any(|value| !value.is_finite()) {
            return Err(TransformError::MalformedProfile(
                "compiled stage produced a non-finite value".into(),
            ));
        }
        Ok(())
    }
}

struct CompiledPair {
    input: CompiledDirection,
    output: CompiledDirection,
}

fn materialize_admitted_pair<I, O, MakeI, MakeO, ValidateI, ValidateO>(
    budget: &mut CompileBudget,
    checkpoint: super::compile_budget::BudgetCheckpoint,
    make_input: MakeI,
    make_output: MakeO,
    validate_input: ValidateI,
    validate_output: ValidateO,
) -> Result<(I, O), TransformError>
where
    MakeI: FnOnce(&mut CompileBudget) -> Result<I, TransformError>,
    MakeO: FnOnce(&mut CompileBudget) -> Result<O, TransformError>,
    ValidateI: FnOnce(&I) -> Result<(), TransformError>,
    ValidateO: FnOnce(&O) -> Result<(), TransformError>,
{
    let input = match make_input(budget) {
        Ok(value) => value,
        Err(error) => {
            budget.rollback(checkpoint);
            return Err(error);
        }
    };
    if let Err(error) = validate_input(&input) {
        drop(input);
        budget.rollback(checkpoint);
        return Err(error);
    }
    let output = match make_output(budget) {
        Ok(value) => value,
        Err(error) => {
            drop(input);
            budget.rollback(checkpoint);
            return Err(error);
        }
    };
    if let Err(error) = validate_output(&output) {
        drop(output);
        drop(input);
        budget.rollback(checkpoint);
        return Err(error);
    }
    Ok((input, output))
}

/// Materialize two already-admitted routes transactionally.  The checkpoint
/// is taken after admission; any partial output and completed input are
/// dropped before restoring that exact admitted state.
fn materialize_pair(
    input_route: super::route_plan::RoutePlan<'_>,
    output_route: super::route_plan::RoutePlan<'_>,
    budget: &mut CompileBudget,
    input_limits: super::limits::ParseLimits,
    output_limits: super::limits::ParseLimits,
) -> Result<CompiledPair, TransformError> {
    let checkpoint = budget.checkpoint();
    let (input, output) = materialize_admitted_pair(
        budget,
        checkpoint,
        |budget| input_route.materialize(budget, input_limits),
        |budget| output_route.materialize(budget, output_limits),
        validate_compiled_direction,
        validate_compiled_direction,
    )?;
    Ok(CompiledPair { input, output })
}

#[derive(Clone, Debug)]
pub struct Transform {
    // Retain the immutable source owners used during construction. Besides
    // making the transform lifetime explicit, this prevents a caller from
    // passing unrelated profiles to memory_usage_with_profiles and receiving
    // an understated build charge.
    pub(super) input_profile: Profile,
    pub(super) output_profile: Profile,
    pub(super) input: Option<Arc<MatrixProfile>>,
    pub(super) output: Option<Arc<MatrixProfile>>,
    pub(super) lut_input: Option<Arc<LutTransform>>,
    pub(super) lut_output: Option<Arc<LutTransform>>,
    input_pcs: super::profile::Pcs,
    output_pcs: super::profile::Pcs,
    input_white: Option<[f32; 3]>,
    output_white: Option<[f32; 3]>,
    input_route_info: RouteInfo,
    output_route_info: RouteInfo,
    input_raw_version: [u8; 4],
    output_raw_version: [u8; 4],
    input_raw_device_class: [u8; 4],
    output_raw_device_class: [u8; 4],
    reverse_pcs_connection: Option<ReversePcsConnection>,
    pub(super) options: TransformOptions,
}

impl Transform {
    /// Parse both profiles with the supplied structural limits and compile a
    /// transform with the supplied decoded-owner limits as one checked
    /// operation.  Parsing and stage admission each happen before their
    /// derived allocations; callers can inspect the retained ownership with
    /// [`Transform::memory_usage`].
    pub fn from_bytes_with_limits(
        input: &[u8],
        output: &[u8],
        options: TransformOptions,
        parse_limits: super::limits::ParseLimits,
        transform_limits: TransformLimits,
    ) -> Result<Self, TransformError> {
        let input_profile = Profile::parse_with_limits(input, parse_limits)?;
        let output_profile = Profile::parse_with_limits(output, parse_limits)?;
        Self::new_with_limits(&input_profile, &output_profile, options, transform_limits)
    }

    pub fn new(
        input: &Profile,
        output: &Profile,
        options: TransformOptions,
    ) -> Result<Self, TransformError> {
        Self::new_with_limits(input, output, options, TransformLimits::default())
    }

    pub fn new_with_limits(
        input: &Profile,
        output: &Profile,
        options: TransformOptions,
        limits: TransformLimits,
    ) -> Result<Self, TransformError> {
        if options.black_point_compensation {
            return Err(TransformError::UnsupportedProfileFeature(
                "black point compensation requires a profile black-point stage",
            ));
        }
        let input_route = plan_route_with_policy(
            input,
            TransformDirection::DeviceToPcs,
            options.rendering_intent,
            limits,
            OwnerPolicy::TransformStage,
        )?;
        let output_route = plan_route_with_policy(
            output,
            TransformDirection::PcsToDevice,
            options.rendering_intent,
            limits,
            OwnerPolicy::TransformStage,
        )?;
        // A matrix/TRC route can be used for the relative colorimetric
        // default without a white-point bridge.  Other intents require the
        // profile's media white; do not silently turn an incomplete synthetic
        // profile into a successful non-relative transform.
        let input_white = selected_media_white(input, options.rendering_intent)?;
        let output_white = selected_media_white(output, options.rendering_intent)?;
        if options.rendering_intent != RenderingIntent::RelativeColorimetric
            && (input_route.stage.is_matrix() || output_route.stage.is_matrix())
            && (input_white.is_none() || output_white.is_none())
        {
            return Err(TransformError::UnsupportedProfileFeature(
                "matrix/TRC non-relative intent requires media white",
            ));
        }
        let reverse_pcs_connection = select_reverse_pcs_connection(
            &input_route,
            &output_route,
            input,
            output,
            options.rendering_intent,
        )?;
        let mut budget = CompileBudget::new(limits);
        admit_pair(&input_route, &output_route, &mut budget)?;
        let pair = materialize_pair(
            input_route,
            output_route,
            &mut budget,
            input.limits(),
            output.limits(),
        )?;
        let input_stage = pair.input;
        let output_stage = pair.output;
        let input_matrix = input_stage.matrix.clone();
        let output_matrix = output_stage.matrix.clone();
        let lut_input = input_stage.lut.clone();
        let lut_output = output_stage.lut.clone();
        Ok(Self {
            input_profile: input.clone(),
            output_profile: output.clone(),
            input: input_matrix,
            output: output_matrix,
            lut_input,
            lut_output,
            input_pcs: input.pcs(),
            output_pcs: output.pcs(),
            input_white,
            output_white,
            input_route_info: input_stage.route_info,
            output_route_info: output_stage.route_info,
            input_raw_version: input.raw_version(),
            output_raw_version: output.raw_version(),
            input_raw_device_class: input.raw_device_class(),
            output_raw_device_class: output.raw_device_class(),
            reverse_pcs_connection,
            options,
        })
    }

    pub fn input_route_info(&self) -> RouteInfo {
        self.input_route_info
    }

    pub fn output_route_info(&self) -> RouteInfo {
        self.output_route_info
    }

    pub fn input_raw_version(&self) -> [u8; 4] {
        self.input_raw_version
    }

    pub fn output_raw_version(&self) -> [u8; 4] {
        self.output_raw_version
    }

    pub fn input_raw_device_class(&self) -> [u8; 4] {
        self.input_raw_device_class
    }

    pub fn output_raw_device_class(&self) -> [u8; 4] {
        self.output_raw_device_class
    }

    pub fn worker(&self) -> super::worker::TransformWorker {
        super::worker::TransformWorker {
            transform: self.clone(),
        }
    }
    pub fn input_channels(&self) -> usize {
        self.lut_input.as_ref().map_or_else(
            || self.input.as_ref().map_or(0, |p| p.curves.len()),
            |p| p.input_channels,
        )
    }
    pub fn output_channels(&self) -> usize {
        self.lut_output.as_ref().map_or_else(
            || self.output.as_ref().map_or(0, |p| p.curves.len()),
            |p| p.output_channels,
        )
    }

    pub(super) fn validate_buffer_lengths(
        &self,
        input_len: usize,
        output_len: usize,
    ) -> Result<(), TransformError> {
        let expected =
            checked_output_len(input_len, self.input_channels(), self.output_channels())?;
        if output_len != expected {
            return Err(TransformError::InvalidBufferLength {
                expected,
                actual: output_len,
            });
        }
        Ok(())
    }

    pub fn transform_f32(&self, input: &[f32], output: &mut [f32]) -> Result<(), TransformError> {
        self.validate_buffer_lengths(input.len(), output.len())?;
        let ic = self.input_channels();
        let oc = self.output_channels();
        for (src, dst) in input.chunks_exact(ic).zip(output.chunks_exact_mut(oc)) {
            if src.iter().any(|x| !x.is_finite()) {
                return Err(TransformError::NonFiniteInput);
            }
            let mut normalized = [0.0; 3];
            normalized[..src.len()].copy_from_slice(src);
            if !self.options.clamp
                && normalized[..src.len()]
                    .iter()
                    .any(|value| !(0.0..=1.0).contains(value))
            {
                return Err(TransformError::MalformedProfile(
                    "device sample is outside the normalized domain".into(),
                ));
            }
            if self.options.clamp {
                for value in &mut normalized[..src.len()] {
                    *value = value.clamp(0.0, 1.0);
                }
            }
            let src = &normalized[..src.len()];
            if let Some(input_lut) = &self.lut_input {
                let mut pcs = [0.0; 3];
                input_lut.eval_with_domain(
                    src,
                    &mut pcs[..input_lut.output_channels],
                    self.options.clamp,
                )?;
                decode_lut_pcs(
                    &mut pcs[..input_lut.output_channels],
                    input_lut.pcs_encoding(),
                )?;
                bridge_pcs(
                    &mut pcs,
                    self.input_pcs,
                    Pcs::Xyz,
                    self.options.rendering_intent == RenderingIntent::AbsoluteColorimetric,
                    self.input_white,
                    self.output_white,
                )?;
                if let Some(output_lut) = &self.lut_output {
                    let mut encoded = pcs;
                    encode_lut_pcs(&mut encoded[..3], output_lut.pcs_encoding())?;
                    output_lut.eval_with_domain(&encoded[..3], dst, self.options.clamp)?;
                } else {
                    xyz_to_device(
                        self.output.as_ref().expect("matrix output path"),
                        pcs,
                        dst,
                        self.options.clamp,
                    )?;
                }
                if self.options.clamp {
                    for x in dst.iter_mut() {
                        *x = x.clamp(0.0, 1.0);
                    }
                }
            } else if let Some(output_lut) = &self.lut_output {
                let xyz = device_to_xyz(
                    self.input.as_ref().expect("matrix path"),
                    src,
                    self.options.clamp,
                );
                let mut pcs = xyz;
                if let Some(connection) = self.reverse_pcs_connection {
                    connection.apply(&mut pcs)?;
                }
                bridge_pcs(
                    &mut pcs,
                    super::profile::Pcs::Xyz,
                    self.output_pcs,
                    self.options.rendering_intent == RenderingIntent::AbsoluteColorimetric,
                    self.input_white,
                    self.output_white,
                )?;
                let mut encoded = pcs;
                encode_lut_pcs(&mut encoded, output_lut.pcs_encoding())?;
                output_lut.eval_with_domain(&encoded, dst, self.options.clamp)?;
                if self.options.clamp {
                    for x in dst.iter_mut() {
                        *x = x.clamp(0.0, 1.0);
                    }
                }
            } else {
                let mut xyz = device_to_xyz(
                    self.input.as_ref().expect("matrix path"),
                    src,
                    self.options.clamp,
                );
                if self.options.rendering_intent == RenderingIntent::AbsoluteColorimetric {
                    xyz = absolute_pcs_xyz(xyz, self.input_white, self.output_white)?;
                }
                xyz_to_device(
                    self.output.as_ref().expect("matrix path"),
                    xyz,
                    dst,
                    self.options.clamp,
                )?;
            }
            if dst.iter().any(|value| !value.is_finite()) {
                return Err(TransformError::MalformedProfile(
                    "transform produced a non-finite value".into(),
                ));
            }
        }
        Ok(())
    }

    /// Transforms into owned storage using the default 64 MiB output bound.
    /// Use [`Self::transform_f32_vec_with_limits`] to select another bound.
    pub fn transform_f32_vec(&self, input: &[f32]) -> Result<Vec<f32>, TransformError> {
        self.transform_f32_vec_with_limits(input, ExecutionLimits::default())
    }

    /// Transforms into owned storage using an explicit output allocation bound.
    /// The bound applies to the returned vector, not to borrowed F32 or
    /// integer execution, which uses a fixed stack workspace.
    pub fn transform_f32_vec_with_limits(
        &self,
        input: &[f32],
        limits: ExecutionLimits,
    ) -> Result<Vec<f32>, TransformError> {
        let plan = checked_output_allocation(
            input.len(),
            self.input_channels(),
            self.output_channels(),
            limits,
        )?;
        let mut result = try_new_f32_output(&plan, limits)?;
        self.transform_f32(input, &mut result)?;
        Ok(result)
    }

    pub fn transform_u8(&self, input: &[u8], output: &mut [u8]) -> Result<(), TransformError> {
        super::worker::transform_u8(self, input, output)
    }

    pub fn transform_u16(&self, input: &[u16], output: &mut [u16]) -> Result<(), TransformError> {
        super::worker::transform_u16(self, input, output)
    }
}

fn select_reverse_pcs_connection(
    input_route: &super::route_plan::RoutePlan<'_>,
    output_route: &super::route_plan::RoutePlan<'_>,
    input: &Profile,
    output: &Profile,
    intent: RenderingIntent,
) -> Result<Option<ReversePcsConnection>, TransformError> {
    if !matches!(
        intent,
        RenderingIntent::Perceptual | RenderingIntent::Saturation
    ) {
        return Ok(None);
    }

    let candidate_shape = input.color_space() == super::profile::ColorSpace::Rgb
        && output.color_space() == super::profile::ColorSpace::Rgb
        && input.raw_version()[0] == 2
        && output.raw_version()[0] == 4
        && input_route.is_matrix()
        && output_route.is_lut()
        && output_route.route_info().selected_tag() == Some(*b"B2A0");
    if !candidate_shape {
        return Ok(None);
    }

    if input.raw_device_class() != *b"mntr" || output.raw_device_class() != *b"mntr" {
        return Err(TransformError::UnsupportedProfileFeature(
            "reverse reference-black route requires an RGB monitor pairing",
        ));
    }
    let matrix = input_route
        .matrix()
        .ok_or(TransformError::InvalidProfile("missing matrix route plan"))?;
    if !matrix.zero_black_status()? {
        return Err(TransformError::UnsupportedProfileFeature(
            "reverse reference-black route requires an exact zero source black",
        ));
    }
    Ok(Some(ReversePcsConnection::V2MatrixToV4B2A0ReferenceBlack))
}

impl ReversePcsConnection {
    fn apply(self, xyz: &mut [f32; 3]) -> Result<(), TransformError> {
        match self {
            Self::V2MatrixToV4B2A0ReferenceBlack => {
                for ((value, black), white) in xyz
                    .iter_mut()
                    .zip(REFERENCE_BLACK_XYZ)
                    .zip(REFERENCE_WHITE_XYZ)
                {
                    if !value.is_finite() {
                        return Err(TransformError::NonFiniteInput);
                    }
                    *value = *value * (1.0 - black / white) + black;
                    if !value.is_finite() {
                        return Err(TransformError::MalformedProfile(
                            "reference-black bridge produced a non-finite XYZ value".into(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

fn validate_compiled_direction(stage: &CompiledDirection) -> Result<(), TransformError> {
    if stage.direction == TransformDirection::PcsToDevice
        && stage
            .matrix
            .as_ref()
            .is_some_and(|value| value.curves.len() > 1 && value.inverse.is_none())
    {
        return Err(TransformError::InvalidProfile("matrix is singular"));
    }
    Ok(())
}

pub(super) fn selected_media_white(
    profile: &Profile,
    intent: RenderingIntent,
) -> Result<Option<[f32; 3]>, TransformError> {
    if intent != RenderingIntent::AbsoluteColorimetric {
        return Ok(profile.media_white());
    }
    let white = profile
        .media_white_checked()?
        .ok_or(TransformError::UnsupportedProfileFeature(
            "absolute colorimetric transform requires media white",
        ))?;
    if white
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(TransformError::InvalidProfile(
            "media white must contain finite positive XYZ components",
        ));
    }
    Ok(Some(white))
}

pub(super) fn device_channels(space: super::profile::ColorSpace) -> usize {
    match space {
        super::profile::ColorSpace::Gray => 1,
        super::profile::ColorSpace::Rgb => 3,
        super::profile::ColorSpace::Cmyk | super::profile::ColorSpace::NColor(_) => 0,
    }
}

fn absolute_pcs_xyz(
    mut xyz: [f32; 3],
    input_white: Option<[f32; 3]>,
    output_white: Option<[f32; 3]>,
) -> Result<[f32; 3], TransformError> {
    let source = input_white.ok_or(TransformError::UnsupportedProfileFeature(
        "absolute intent requires source media white",
    ))?;
    let destination = output_white.ok_or(TransformError::UnsupportedProfileFeature(
        "absolute intent requires destination media white",
    ))?;
    for i in 0..3 {
        if !source[i].is_finite()
            || !destination[i].is_finite()
            || source[i] <= 0.0
            || destination[i] <= 0.0
        {
            return Err(TransformError::InvalidProfile("invalid media white"));
        }
        xyz[i] *= source[i] / D50[i];
        xyz[i] *= D50[i] / destination[i];
    }
    Ok(xyz)
}

fn bridge_pcs(
    values: &mut [f32],
    from: Pcs,
    to: Pcs,
    absolute: bool,
    input_white: Option<[f32; 3]>,
    output_white: Option<[f32; 3]>,
) -> Result<(), TransformError> {
    if absolute {
        convert_pcs(values, from, Pcs::Xyz)?;
        let xyz = [values[0], values[1], values[2]];
        let xyz = absolute_pcs_xyz(xyz, input_white, output_white)?;
        values.copy_from_slice(&xyz);
        convert_pcs(values, Pcs::Xyz, to)
    } else {
        convert_pcs(values, from, to)
    }
}

fn device_to_xyz(profile: &MatrixProfile, input: &[f32], clamp: bool) -> [f32; 3] {
    let mut v = [0.0; 3];
    for i in 0..3.min(input.len()) {
        let sample = if clamp {
            input[i].clamp(0.0, 1.0)
        } else {
            input[i]
        };
        v[i] = if clamp {
            profile.curves[i].eval(sample)
        } else {
            profile.curves[i].eval_unclamped(sample)
        };
    }
    if profile.pcs == Pcs::Lab && profile.curves.len() == 1 {
        return gray_lab_l_to_xyz(v[0]);
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
        let gray = if profile.pcs == Pcs::Lab {
            gray_lab_l_from_xyz(xyz)?
        } else {
            xyz[1] / D50[1]
        };
        output[0] = inverse_curve_checked(&profile.curves[0], gray, clamp)?;
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
        let value = if clamp {
            (*value).clamp(0.0, 1.0)
        } else {
            *value
        };
        output[i] = inverse_curve_checked(&profile.curves[i], value, clamp)?;
    }
    Ok(())
}

fn gray_lab_l_to_xyz(luma: f32) -> [f32; 3] {
    let fy = (luma * 100.0 + 16.0) / 116.0;
    let threshold = 216.0 / 24389.0;
    let k = 24389.0 / 27.0;
    let finv = |value: f32| {
        let cubic = value * value * value;
        if cubic > threshold {
            cubic
        } else {
            (116.0 * value - 16.0) / k
        }
    };
    let y = finv(fy);
    [y * D50[0], y, y * D50[2]]
}

fn gray_lab_l_from_xyz(xyz: [f32; 3]) -> Result<f32, TransformError> {
    if xyz.iter().any(|value| !value.is_finite()) {
        return Err(TransformError::NonFiniteInput);
    }
    // ICC Annex F.2 defines a Gray Lab device by luminance only.  The
    // destination Gray stage consumes L*/100 derived from Y; chromatic XYZ
    // components do not make an RGB source invalid at this boundary.
    let y = xyz[1] / D50[1];
    let f = if y > 216.0 / 24389.0 {
        y.cbrt()
    } else {
        (24389.0 / 27.0 * y + 16.0) / 116.0
    };
    Ok((116.0 * f - 16.0) / 100.0)
}

fn convert_pcs(
    values: &mut [f32],
    from: super::profile::Pcs,
    to: super::profile::Pcs,
) -> Result<(), TransformError> {
    if from == to {
        return Ok(());
    }
    if values.len() != 3 {
        return Err(TransformError::InvalidBufferLength {
            expected: 3,
            actual: values.len(),
        });
    }
    if !values.iter().all(|v| v.is_finite()) {
        return Err(TransformError::NonFiniteInput);
    }
    let d50 = D50;
    if from == super::profile::Pcs::Xyz && to == super::profile::Pcs::Lab {
        let f = |x: f32| {
            if x > 216.0 / 24389.0 {
                x.cbrt()
            } else {
                (24389.0 / 27.0 * x + 16.0) / 116.0
            }
        };
        let x = f(values[0] / d50[0]);
        let y = f(values[1] / d50[1]);
        let z = f(values[2] / d50[2]);
        values[0] = 116.0 * y - 16.0;
        values[1] = 500.0 * (x - y);
        values[2] = 200.0 * (y - z);
    } else {
        let l = values[0];
        let a = values[1];
        let b = values[2];
        let fy = (l + 16.0) / 116.0;
        let fx = fy + a / 500.0;
        let fz = fy - b / 200.0;
        let e = 216.0 / 24389.0;
        let k = 24389.0 / 27.0;
        let finv = |v: f32| {
            let v3 = v * v * v;
            if v3 > e {
                v3
            } else {
                (116.0 * v - 16.0) / k
            }
        };
        values[0] = finv(fx) * d50[0];
        values[1] = finv(fy) * d50[1];
        values[2] = finv(fz) * d50[2];
    }
    Ok(())
}

fn decode_lut_pcs(values: &mut [f32], encoding: PcsEncoding) -> Result<(), TransformError> {
    if values.len() != 3 || values.iter().any(|value| !value.is_finite()) {
        return Err(TransformError::MalformedProfile(
            "invalid encoded PCS value".into(),
        ));
    }
    match encoding {
        PcsEncoding::LegacyXyz16 => {
            for value in &mut *values {
                *value *= 65535.0 / 32768.0;
            }
        }
        PcsEncoding::ModernXyz => {
            for value in &mut *values {
                *value *= 65535.0 / 32768.0;
            }
        }
        PcsEncoding::LegacyLab16 => {
            values[0] *= 65535.0 / 65280.0 * 100.0;
            values[1] = values[1] * 65535.0 / 256.0 - 128.0;
            values[2] = values[2] * 65535.0 / 256.0 - 128.0;
        }
        PcsEncoding::ModernLab => {
            values[0] *= 100.0;
            values[1] = values[1] * 255.0 - 128.0;
            values[2] = values[2] * 255.0 - 128.0;
        }
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(TransformError::MalformedProfile(
            "decoded PCS value is non-finite".into(),
        ));
    }
    Ok(())
}

fn encode_lut_pcs(values: &mut [f32], encoding: PcsEncoding) -> Result<(), TransformError> {
    if values.len() != 3 || values.iter().any(|value| !value.is_finite()) {
        return Err(TransformError::MalformedProfile(
            "invalid physical PCS value".into(),
        ));
    }
    match encoding {
        PcsEncoding::LegacyXyz16 | PcsEncoding::ModernXyz => {
            for value in &mut *values {
                *value *= 32768.0 / 65535.0;
            }
        }
        PcsEncoding::LegacyLab16 => {
            values[0] *= 65280.0 / (100.0 * 65535.0);
            values[1] = (values[1] + 128.0) * 256.0 / 65535.0;
            values[2] = (values[2] + 128.0) * 256.0 / 65535.0;
        }
        PcsEncoding::ModernLab => {
            values[0] /= 100.0;
            values[1] = (values[1] + 128.0) / 255.0;
            values[2] = (values[2] + 128.0) / 255.0;
        }
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(TransformError::MalformedProfile(
            "encoded PCS value is non-finite".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "compile_s3_tests.rs"]
mod compile_s3_tests;

#[cfg(test)]
#[path = "compile_s4_tests.rs"]
mod compile_s4_tests;

#[cfg(test)]
#[path = "reverse_diagnostics_tests.rs"]
mod reverse_diagnostics_tests;
