use super::compile_budget::CompileBudget;
use super::curve::Curve;
use super::error::TransformError;
use super::limits::{ParseLimits, TransformLimits};
use super::reader::{be_i32, be_u16, be_u32, checked_range};

/// Borrowed, allocation-free shape information for one matrix/TRC curve.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct CurvePlan<'a> {
    pub(super) data: &'a [u8],
    pub(super) kind: CurveKind,
    pub(super) entries: usize,
    pub(super) decoded_bytes: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) enum CurveKind {
    #[default]
    Identity,
    Gamma(f32),
    Table,
    Para(u16),
}

impl CurvePlan<'_> {
    /// Evaluate the borrowed curve at the device black endpoint without
    /// creating a materialized `Curve`.
    pub(super) fn zero_value(&self) -> Result<f32, TransformError> {
        let x = 0.0_f32;
        let value = match self.kind {
            CurveKind::Identity => x,
            CurveKind::Gamma(gamma) => x.powf(gamma),
            CurveKind::Table => {
                if self.entries == 0 {
                    x
                } else {
                    f32::from(be_u16(self.data, 12)?) / 65535.0
                }
            }
            CurveKind::Para(function) => {
                let mut values = [0.0_f32; 7];
                for (index, value) in values.iter_mut().take(self.entries).enumerate() {
                    *value = be_i32(self.data, 12 + index * 4)? as f32 / 65536.0;
                }
                super::curve::eval_parametric(function, &values, x)
            }
        };
        if value.is_finite() {
            Ok(value)
        } else {
            Err(TransformError::InvalidProfile(
                "curve endpoint is non-finite",
            ))
        }
    }
}

pub(super) fn plan_curve(
    data: &[u8],
    limits: TransformLimits,
) -> Result<CurvePlan<'_>, TransformError> {
    if data.len() < 12 {
        return Err(TransformError::InvalidProfile("curve tag is truncated"));
    }
    match be_u32(data, 0)? {
        value if value == u32::from_be_bytes(*b"curv") => {
            let entries = usize::try_from(be_u32(data, 8)?)
                .map_err(|_| TransformError::ResourceLimit("curve entries"))?;
            if entries > limits.max_curve_entries {
                return Err(TransformError::ResourceLimit("curve entries"));
            }
            let encoded_bytes = if entries == 0 {
                0
            } else if entries == 1 {
                2
            } else {
                entries
                    .checked_mul(2)
                    .ok_or(TransformError::ResourceLimit("curve arithmetic"))?
            };
            checked_range(data, 12, encoded_bytes)?;
            let decoded_bytes = entries
                .checked_sub(usize::from(entries == 1))
                .and_then(|count| count.checked_mul(std::mem::size_of::<f32>()))
                .ok_or(TransformError::ResourceLimit("curve storage"))?;
            if decoded_bytes > limits.max_compiled_bytes {
                return Err(TransformError::ResourceLimit("compiled transform bytes"));
            }
            let gamma = if entries == 1 {
                let gamma = f32::from(be_u16(data, 12)?) / 256.0;
                if !gamma.is_finite() || gamma <= 0.0 {
                    return Err(TransformError::InvalidProfile("curve gamma"));
                }
                Some(gamma)
            } else {
                None
            };
            Ok(CurvePlan {
                data,
                kind: if entries == 0 {
                    CurveKind::Identity
                } else if entries == 1 {
                    CurveKind::Gamma(gamma.expect("single-entry curve has gamma"))
                } else {
                    CurveKind::Table
                },
                entries,
                decoded_bytes,
            })
        }
        value if value == u32::from_be_bytes(*b"para") => {
            let function = be_u16(data, 8)?;
            let entries: usize = match function {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => {
                    return Err(TransformError::UnsupportedProfileFeature(
                        "parametric curve function",
                    ));
                }
            };
            let encoded_bytes = entries
                .checked_mul(4)
                .ok_or(TransformError::ResourceLimit("curve arithmetic"))?;
            checked_range(data, 12, encoded_bytes)?;
            let decoded_bytes = entries
                .checked_mul(std::mem::size_of::<f32>())
                .ok_or(TransformError::ResourceLimit("curve storage"))?;
            if decoded_bytes > limits.max_compiled_bytes {
                return Err(TransformError::ResourceLimit("compiled transform bytes"));
            }
            Ok(CurvePlan {
                data,
                kind: CurveKind::Para(function),
                entries,
                decoded_bytes,
            })
        }
        _ => Err(TransformError::UnsupportedProfileFeature("curve tag type")),
    }
}

pub(super) fn materialize_curve(
    plan: CurvePlan<'_>,
    budget: &mut CompileBudget,
    inverse_direction: bool,
) -> Result<Curve, TransformError> {
    match plan.kind {
        CurveKind::Identity => Ok(Curve::Identity),
        CurveKind::Gamma(gamma) => Ok(Curve::Gamma(gamma)),
        CurveKind::Table => {
            let mut table = budget.try_new_vec::<f32>(
                plan.entries,
                plan.decoded_bytes,
                "matrix curve table",
            )?;
            for index in 0..plan.entries {
                table.push(be_u16(plan.data, 12 + index * 2)? as f32 / 65535.0);
            }
            let increasing = table.windows(2).all(|pair| pair[0] <= pair[1]);
            let decreasing = table.windows(2).all(|pair| pair[0] >= pair[1]);
            if inverse_direction && (table.first() == table.last() || (!increasing && !decreasing))
            {
                return Err(TransformError::InvalidProfile(
                    "sampled curve must be monotonic and non-constant",
                ));
            }
            Ok(Curve::Table(table))
        }
        CurveKind::Para(function) => {
            let mut values = budget.try_new_vec::<f32>(
                plan.entries,
                plan.decoded_bytes,
                "matrix parametric curve",
            )?;
            for index in 0..plan.entries {
                values.push(super::reader::be_i32(plan.data, 12 + index * 4)? as f32 / 65536.0);
            }
            if values.iter().any(|value| !value.is_finite()) || values[0] <= 0.0 {
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
            let direction = if inverse_direction {
                super::curve::validate_parametric_curve_values(function, &values)?
            } else {
                1
            };
            let curve = Curve::Para {
                function,
                values,
                direction,
            };
            super::curve::validate_parametric_domain_curve(&curve)?;
            Ok(curve)
        }
    }
}

/// Apply the profile parser's per-tag bound without applying it to para
/// functions. The legacy parser bounds `curv` tags, while parametric curves
/// are bounded by their fixed function shape.
pub(super) fn validate_parse_limit(
    plan: CurvePlan<'_>,
    limits: ParseLimits,
) -> Result<(), TransformError> {
    if !matches!(plan.kind, CurveKind::Para(_)) && plan.entries > limits.max_curve_entries {
        return Err(TransformError::ResourceLimit("curve entries"));
    }
    Ok(())
}

pub(super) fn curve_encoded_size(data: &[u8]) -> Result<usize, TransformError> {
    match be_u32(data, 0)? {
        signature if signature == u32::from_be_bytes(*b"curv") => {
            let count = usize::try_from(be_u32(data, 8)?)
                .map_err(|_| TransformError::ResourceLimit("curve entries"))?;
            let size = 12usize
                .checked_add(
                    count
                        .checked_mul(2)
                        .ok_or(TransformError::ResourceLimit("curve size"))?,
                )
                .ok_or(TransformError::ResourceLimit("curve size"))?
                .checked_add(3)
                .map(|value| value & !3)
                .ok_or(TransformError::ResourceLimit("curve alignment"))?;
            checked_range(data, 0, size)?;
            Ok(size)
        }
        signature if signature == u32::from_be_bytes(*b"para") => {
            let entries: usize = match be_u16(data, 8)? {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => {
                    return Err(TransformError::UnsupportedProfileFeature(
                        "parametric curve function",
                    ));
                }
            };
            let size = 12usize
                .checked_add(
                    entries
                        .checked_mul(4)
                        .ok_or(TransformError::ResourceLimit("curve size"))?,
                )
                .ok_or(TransformError::ResourceLimit("curve size"))?
                .checked_add(3)
                .map(|value| value & !3)
                .ok_or(TransformError::ResourceLimit("curve alignment"))?;
            checked_range(data, 0, size)?;
            Ok(size)
        }
        _ => Err(TransformError::UnsupportedProfileFeature("curve tag type")),
    }
}
