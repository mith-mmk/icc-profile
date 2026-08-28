use super::compile::TransformDirection;
use super::compile_budget::CompileBudget;
use super::curve::Curve;
use super::curve_plan::{materialize_curve, plan_curve, CurvePlan};
use super::direction::find_intent_tag;
use super::error::TransformError;
use super::limits::TransformLimits;
use super::profile::{ColorSpace, MatrixProfile, Pcs, Profile, RenderingIntent};
use super::reader::D50;

/// Borrowed matrix/TRC route shape. The fixed array matches the supported
/// Gray/RGB channel counts and avoids collecting profile data while planning.
#[derive(Clone, Copy, Debug)]
pub(super) struct MatrixPlan<'a> {
    pub(super) channels: usize,
    pub(super) pcs: Pcs,
    pub(super) matrix: [[f32; 3]; 3],
    pub(super) inverse: Option<[[f32; 3]; 3]>,
    pub(super) curves: [CurvePlan<'a>; 3],
}

impl<'a> MatrixPlan<'a> {
    pub(super) fn zero_black_status(&self) -> Result<bool, TransformError> {
        if self.channels != 3 {
            return Ok(false);
        }
        let mut linear = [0.0_f32; 3];
        for (value, curve) in linear
            .iter_mut()
            .zip(self.curves.iter().take(self.channels))
        {
            *value = curve.zero_value()?;
        }
        let physical = [
            self.matrix[0][0] * linear[0]
                + self.matrix[0][1] * linear[1]
                + self.matrix[0][2] * linear[2],
            self.matrix[1][0] * linear[0]
                + self.matrix[1][1] * linear[1]
                + self.matrix[1][2] * linear[2],
            self.matrix[2][0] * linear[0]
                + self.matrix[2][1] * linear[1]
                + self.matrix[2][2] * linear[2],
        ];
        if physical.iter().any(|value| !value.is_finite()) {
            return Err(TransformError::InvalidProfile(
                "matrix black endpoint is non-finite",
            ));
        }
        Ok(physical.iter().all(|value| *value == 0.0))
    }

    #[cfg(test)]
    pub(super) fn inventory(
        &self,
        owned_headers: usize,
    ) -> Result<(usize, usize, usize), TransformError> {
        let mut storage = self
            .channels
            .checked_mul(std::mem::size_of::<Curve>())
            .and_then(|value| value.checked_add(owned_headers))
            .ok_or(TransformError::ResourceLimit("matrix owner storage"))?;
        let mut curve_entries = 0usize;
        for curve in self.curves.iter().take(self.channels) {
            storage = storage
                .checked_add(curve.decoded_bytes)
                .ok_or(TransformError::ResourceLimit("matrix owner storage"))?;
            curve_entries = curve_entries
                .checked_add(curve.entries)
                .ok_or(TransformError::ResourceLimit("matrix curve entries"))?;
        }
        Ok((storage, curve_entries, 0))
    }

    pub(super) fn admit(
        &self,
        budget: &mut CompileBudget,
        owned_headers: usize,
    ) -> Result<(), TransformError> {
        let checkpoint = budget.checkpoint();
        if let Err(error) = (|| {
            budget.admit_matrix_storage(self.channels, owned_headers)?;
            for curve in self.curves.iter().take(self.channels) {
                budget.admit_curve(curve.entries, curve.decoded_bytes)?;
            }
            Ok::<(), TransformError>(())
        })() {
            budget.rollback(checkpoint);
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn materialize(
        &self,
        budget: &mut CompileBudget,
        inverse_direction: bool,
        owned_headers: usize,
    ) -> Result<MatrixProfile, TransformError> {
        let outer_bytes = self
            .channels
            .checked_mul(std::mem::size_of::<Curve>())
            .ok_or(TransformError::ResourceLimit("compiled transform bytes"))?;
        budget.commit_owned(owned_headers, owned_headers, "matrix owner headers")?;
        let mut curves =
            budget.try_new_vec::<Curve>(self.channels, outer_bytes, "matrix curve owner")?;
        for curve in self.curves.iter().take(self.channels) {
            curves.push(materialize_curve(*curve, budget, inverse_direction)?);
        }
        Ok(MatrixProfile {
            pcs: self.pcs,
            matrix: self.matrix,
            inverse: if inverse_direction {
                self.inverse
            } else {
                None
            },
            curves,
        })
    }
}

pub(super) fn plan_matrix(
    profile: &Profile,
    direction: TransformDirection,
    intent: RenderingIntent,
    limits: TransformLimits,
) -> Result<Option<MatrixPlan<'_>>, TransformError> {
    let channels = match profile.color_space() {
        ColorSpace::Gray => 1,
        ColorSpace::Rgb => 3,
        ColorSpace::Cmyk | ColorSpace::NColor(_) => return Ok(None),
    };
    if profile.pcs() != super::profile::Pcs::Xyz
        && !(channels == 1 && profile.pcs() == super::profile::Pcs::Lab)
    {
        return Ok(None);
    }
    let prefix = if direction == TransformDirection::DeviceToPcs {
        b"A2B"
    } else {
        b"B2A"
    };
    if find_intent_tag(profile, prefix, intent as u32).is_some() {
        return Ok(None);
    }
    let names: [&[u8; 4]; 3] = if channels == 1 {
        [b"kTRC", b"kTRC", b"kTRC"]
    } else {
        [b"rTRC", b"gTRC", b"bTRC"]
    };
    let required: &[&[u8; 4]] = &names[..channels];
    if !required
        .iter()
        .all(|name| profile.tag(u32::from_be_bytes(**name)).is_some())
    {
        return Ok(None);
    }
    if channels == 3 {
        for name in [b"rXYZ", b"gXYZ", b"bXYZ"] {
            if profile.tag(u32::from_be_bytes(*name)).is_none() {
                return Ok(None);
            }
        }
    }
    let mut curves = [CurvePlan::default(); 3];
    for (index, name) in required.iter().enumerate() {
        let data = profile
            .tag(u32::from_be_bytes(**name))
            .ok_or(TransformError::UnsupportedProfileFeature("TRC tag"))?;
        let plan = plan_curve(data, limits)?;
        curves[index] = plan;
    }
    let matrix = if channels == 1 {
        [[D50[0], 0.0, 0.0], [D50[1], 0.0, 0.0], [D50[2], 0.0, 0.0]]
    } else {
        let mut matrix = [[0.0; 3]; 3];
        for (column, name) in [b"rXYZ", b"gXYZ", b"bXYZ"].iter().enumerate() {
            let data = profile.tag(u32::from_be_bytes(**name)).ok_or(
                TransformError::UnsupportedProfileFeature("matrix colorant tag"),
            )?;
            let value = super::profile::parse_xyz_tag(data)?;
            matrix[0][column] = value[0];
            matrix[1][column] = value[1];
            matrix[2][column] = value[2];
        }
        matrix
    };
    let inverse = if channels == 1 {
        None
    } else {
        super::profile::invert_matrix(matrix)
    };
    Ok(Some(MatrixPlan {
        channels,
        pcs: profile.pcs(),
        matrix,
        inverse,
        curves,
    }))
}
