//! Borrowed, direction-specific route selection for shared compilation.
//!
//! A route plan contains only checked references into an immutable profile.
//! It is admitted before either direction materializes decoded owners so a
//! two-direction transform has one atomic compiled-data budget.

use std::mem::size_of;

use super::compile::{device_channels, CompiledDirection, TransformDirection};
use super::compile_budget::CompileBudget;
use super::compile_plan::{plan_matrix, MatrixPlan};
use super::error::TransformError;
use super::limits::{ParseLimits, TransformLimits};
use super::lut::LutTransform;
use super::lut_plan::{plan_lut, LutPlan};
use super::profile::{Pcs, Profile, RenderingIntent, RouteInfo, RouteModel};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StageKind {
    Matrix,
    Lut,
}

/// Allocation-free selected-stage storage. Both alternatives use fixed
/// fields, so planning never boxes the larger LUT description.
#[derive(Clone, Copy, Debug)]
pub(super) struct SelectedStagePlan<'a> {
    kind: StageKind,
    matrix: Option<MatrixPlan<'a>>,
    lut: Option<LutPlan<'a>>,
}

impl SelectedStagePlan<'_> {
    pub(super) fn matrix(&self) -> Option<&MatrixPlan<'_>> {
        self.matrix.as_ref()
    }

    pub(super) fn lut(&self) -> Option<&LutPlan<'_>> {
        self.lut.as_ref()
    }

    pub(super) fn is_matrix(&self) -> bool {
        self.kind == StageKind::Matrix
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OwnerPolicy {
    CompiledProfile,
    TransformStage,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RoutePlan<'a> {
    pub(super) direction: TransformDirection,
    pub(super) stage: SelectedStagePlan<'a>,
    pub(super) pcs: Pcs,
    pub(super) input_channels: usize,
    pub(super) output_channels: usize,
    stage_headers: usize,
    owner_policy: OwnerPolicy,
    route_info: RouteInfo,
    raw_version: [u8; 4],
    raw_device_class: [u8; 4],
}

pub(super) fn plan_route(
    profile: &Profile,
    direction: TransformDirection,
    intent: RenderingIntent,
    limits: TransformLimits,
) -> Result<RoutePlan<'_>, TransformError> {
    plan_route_with_policy(
        profile,
        direction,
        intent,
        limits,
        OwnerPolicy::CompiledProfile,
    )
}

pub(super) fn plan_route_with_policy(
    profile: &Profile,
    direction: TransformDirection,
    intent: RenderingIntent,
    limits: TransformLimits,
    owner_policy: OwnerPolicy,
) -> Result<RoutePlan<'_>, TransformError> {
    let a_to_b = direction == TransformDirection::DeviceToPcs;
    let prefix = if a_to_b { b"A2B" } else { b"B2A" };
    let selection = super::direction::select_intent_tag(profile, prefix, intent as u32);
    let tag = selection.data;
    let channels = device_channels(profile.color_space());
    if channels == 0 {
        return Err(TransformError::UnsupportedProfileFeature(
            "only Gray/RGB transforms are supported",
        ));
    }
    let expected_channels = if a_to_b { (channels, 3) } else { (3, channels) };
    let model = if tag.is_some() {
        RouteModel::Lut
    } else {
        RouteModel::Matrix
    };
    validate_executable_model(profile, model)?;
    super::compile::selected_media_white(profile, intent)?;
    let lut = tag
        .map(|data| {
            plan_lut(
                data,
                expected_channels,
                profile.pcs(),
                a_to_b,
                limits,
                profile.limits(),
            )
        })
        .transpose()?;
    let matrix = if lut.is_none() {
        plan_matrix(profile, direction, intent, limits)?
    } else {
        None
    };
    let (stage, stage_headers) = match (lut, matrix) {
        (Some(lut), None) => (
            SelectedStagePlan {
                kind: StageKind::Lut,
                matrix: None,
                lut: Some(lut),
            },
            size_of::<LutTransform>(),
        ),
        (None, Some(matrix)) => (
            SelectedStagePlan {
                kind: StageKind::Matrix,
                matrix: Some(matrix),
                lut: None,
            },
            size_of::<super::profile::MatrixProfile>(),
        ),
        (None, None) => {
            return Err(TransformError::UnsupportedProfileFeature(
                "profile has no supported direction-specific stage",
            ));
        }
        (Some(_), Some(_)) => {
            return Err(TransformError::MalformedProfile(
                "profile selected multiple direction-specific stages".into(),
            ));
        }
    };
    Ok(RoutePlan {
        direction,
        stage,
        pcs: profile.pcs(),
        input_channels: expected_channels.0,
        output_channels: expected_channels.1,
        stage_headers,
        owner_policy,
        route_info: RouteInfo::new(
            intent,
            selection.signature,
            model,
            selection.used_fallback || (model == RouteModel::Matrix && tag.is_none()),
        ),
        raw_version: profile.raw_version(),
        raw_device_class: profile.raw_device_class(),
    })
}

fn validate_executable_model(profile: &Profile, model: RouteModel) -> Result<(), TransformError> {
    let version = profile.raw_version();
    if !matches!(version[0], 2 | 4) {
        return Err(TransformError::UnsupportedProfileFeature(
            "ICC version is not supported for transform compilation",
        ));
    }
    let class = profile.raw_device_class();
    let scnr = *b"scnr";
    let mntr = *b"mntr";
    let prtr = *b"prtr";
    let spac = *b"spac";
    let matrix_allowed = match class {
        value if value == scnr || value == mntr => true,
        value if value == prtr => profile.color_space() == super::profile::ColorSpace::Gray,
        _ => false,
    };
    let class_allowed = match model {
        RouteModel::Lut => class == scnr || class == mntr || class == prtr || class == spac,
        RouteModel::Matrix => matrix_allowed && class != spac,
    };
    if !class_allowed {
        return Err(TransformError::UnsupportedProfileFeature(
            "ICC device class/model is not supported for transform compilation",
        ));
    }
    if model == RouteModel::Matrix
        && profile.color_space() == super::profile::ColorSpace::Rgb
        && profile.pcs() != Pcs::Xyz
    {
        return Err(TransformError::UnsupportedProfileFeature(
            "RGB matrix/TRC requires XYZ PCS",
        ));
    }
    Ok(())
}

impl RoutePlan<'_> {
    pub(super) fn admit(&self, budget: &mut CompileBudget) -> Result<(), TransformError> {
        let checkpoint = budget.checkpoint();
        let result = (|| {
            if let Some(matrix) = self.stage.matrix() {
                matrix.admit(budget, self.stage_headers)?;
            } else if let Some(lut) = self.stage.lut() {
                lut.admit(budget, self.stage_headers)?;
            } else {
                return Err(TransformError::InvalidProfile("empty selected stage"));
            }
            if self.owner_policy == OwnerPolicy::CompiledProfile {
                budget
                    .admit_storage(size_of::<CompiledDirection>(), "compiled direction header")?;
            }
            Ok(())
        })();
        if result.is_err() {
            budget.rollback(checkpoint);
        }
        result
    }

    pub(super) fn materialize(
        self,
        budget: &mut CompileBudget,
        parse_limits: ParseLimits,
    ) -> Result<CompiledDirection, TransformError> {
        let checkpoint = budget.checkpoint();
        let stage = match self.materialize_stage(budget, parse_limits) {
            Ok(stage) => stage,
            Err(error) => {
                budget.rollback(checkpoint);
                return Err(error);
            }
        };
        if self.owner_policy == OwnerPolicy::CompiledProfile {
            if let Err(error) = budget.commit_owned(
                size_of::<CompiledDirection>(),
                size_of::<CompiledDirection>(),
                "compiled direction header",
            ) {
                drop(stage);
                budget.rollback(checkpoint);
                return Err(error);
            }
        }
        Ok(stage)
    }

    fn materialize_stage(
        self,
        budget: &mut CompileBudget,
        parse_limits: ParseLimits,
    ) -> Result<CompiledDirection, TransformError> {
        match self.stage {
            SelectedStagePlan {
                matrix: Some(matrix),
                ..
            } => {
                let direction = self.direction;
                let compiled = matrix.materialize(
                    budget,
                    direction == TransformDirection::PcsToDevice,
                    self.stage_headers,
                )?;
                Ok(CompiledDirection {
                    direction,
                    matrix: Some(std::sync::Arc::new(compiled)),
                    lut: None,
                    pcs: self.pcs,
                    input_channels: self.input_channels,
                    output_channels: self.output_channels,
                    route_info: self.route_info,
                    raw_version: self.raw_version,
                    raw_device_class: self.raw_device_class,
                })
            }
            SelectedStagePlan { lut: Some(lut), .. } => {
                let direction = self.direction;
                let compiled =
                    lut.materialize_with_budget(budget, parse_limits, self.stage_headers)?;
                Ok(CompiledDirection {
                    direction,
                    matrix: None,
                    lut: Some(std::sync::Arc::new(compiled)),
                    pcs: self.pcs,
                    input_channels: self.input_channels,
                    output_channels: self.output_channels,
                    route_info: self.route_info,
                    raw_version: self.raw_version,
                    raw_device_class: self.raw_device_class,
                })
            }
            SelectedStagePlan {
                matrix: None,
                lut: None,
                ..
            } => Err(TransformError::InvalidProfile("empty selected stage")),
        }
    }

    #[cfg(test)]
    pub(super) fn inventory(&self) -> Result<(usize, usize, usize), TransformError> {
        let (storage, curves, clut) = if let Some(matrix) = self.stage.matrix() {
            matrix.inventory(self.stage_headers)?
        } else if let Some(lut) = self.stage.lut() {
            lut.inventory(self.stage_headers)?
        } else {
            return Err(TransformError::InvalidProfile("empty selected stage"));
        };
        let storage = if self.owner_policy == OwnerPolicy::CompiledProfile {
            storage
                .checked_add(size_of::<CompiledDirection>())
                .ok_or(TransformError::ResourceLimit("compiled direction header"))?
        } else {
            storage
        };
        Ok((storage, curves, clut))
    }
}

pub(super) fn admit_pair(
    input: &RoutePlan<'_>,
    output: &RoutePlan<'_>,
    budget: &mut CompileBudget,
) -> Result<(), TransformError> {
    let checkpoint = budget.checkpoint();
    if let Err(error) = (|| {
        input.admit(budget)?;
        output.admit(budget)
    })() {
        budget.rollback(checkpoint);
        return Err(error);
    }
    Ok(())
}
