//! Borrowed structural plans for direction-specific ICC LUT stages.
//!
//! The plan owns no decoded tables.  It validates the selected direction,
//! channel shape, stage combination, and every encoded range before the LUT
//! parser is allowed to allocate any curve or CLUT storage.

use std::mem::size_of;

use super::curve_plan::{curve_encoded_size, plan_curve, validate_parse_limit, CurvePlan};
use super::error::TransformError;
use super::limits::{ParseLimits, TransformLimits};
use super::lut::LutTransform;
use super::profile::Pcs;
use super::reader::{be_i32, be_u16, be_u32, checked_range};

const MFT1: u32 = u32::from_be_bytes(*b"mft1");
const MFT2: u32 = u32::from_be_bytes(*b"mft2");
const MAB: u32 = u32::from_be_bytes(*b"mAB ");
const MBA: u32 = u32::from_be_bytes(*b"mBA ");

#[derive(Clone, Copy, Debug)]
pub(super) struct LutShape<'a> {
    pub(super) input_channels: usize,
    pub(super) output_channels: usize,
    pub(super) curves: CurveSetShape3<'a>,
    pub(super) layout: LutLayout,
}

pub(super) type CurveSetShape3<'a> = [CurveSetShape<'a>; 3];

#[derive(Clone, Copy, Debug)]
pub(super) enum LutLayout {
    Mft {
        wide: bool,
        grid: usize,
        entries_in: usize,
        entries_out: usize,
        matrix: [[f32; 3]; 3],
        input_range: EncodedRange,
        clut_range: EncodedRange,
        output_range: EncodedRange,
    },
    Mab {
        reverse: bool,
        matrix_range: Option<EncodedRange>,
        clut: Option<ClutShape>,
    },
}

#[derive(Clone, Copy, Debug)]
pub(super) struct EncodedRange {
    pub(super) offset: usize,
    pub(super) size: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct CurveSetShape<'a> {
    pub(super) plans: [CurvePlan<'a>; 3],
    pub(super) count: usize,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ClutShape {
    pub(super) offset: usize,
    pub(super) dimensions: usize,
    pub(super) channels: usize,
    pub(super) grid: [usize; 16],
    pub(super) grid_len: usize,
    pub(super) bytes: usize,
    pub(super) values: EncodedRange,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LutPlan<'a> {
    data: &'a [u8],
    shape: LutShape<'a>,
    pcs: Pcs,
    a_to_b: bool,
    limits: TransformLimits,
}

#[derive(Clone, Copy, Debug)]
struct LutOwnerInventory {
    curve_entries: usize,
    storage_bytes: usize,
    clut_entries: usize,
}

/// Build a checked shape without allocating decoded curves or CLUT values.
/// This is the single structural planning entrypoint used by both the
/// bounded route and the compatibility wrapper in `lut.rs`.
pub(super) fn checked_shape<'a>(
    data: &'a [u8],
    limits: TransformLimits,
    pcs: Pcs,
    parse_limits: ParseLimits,
) -> Result<LutShape<'a>, TransformError> {
    let shape = checked_shape_inner(data, limits, pcs, parse_limits)?;
    validate_shape_limits(&shape, limits)?;
    Ok(shape)
}

#[cfg(test)]
pub(super) fn check_encoded_limits(
    data: &[u8],
    limits: TransformLimits,
) -> Result<(), TransformError> {
    let shape = checked_shape_inner(data, limits, Pcs::Xyz, ParseLimits::default())?;
    validate_shape_limits(&shape, limits)
}

fn checked_shape_inner<'a>(
    data: &'a [u8],
    limits: TransformLimits,
    pcs: Pcs,
    parse_limits: ParseLimits,
) -> Result<LutShape<'a>, TransformError> {
    checked_range(data, 0, 4)?;
    match be_u32(data, 0)? {
        MFT1 | MFT2 => checked_mft_shape(data, pcs),
        MAB | MBA => checked_mab_shape(data, limits, parse_limits),
        _ => Err(TransformError::UnsupportedProfileFeature("LUT tag type")),
    }
}

fn validate_shape_limits(
    shape: &LutShape<'_>,
    limits: TransformLimits,
) -> Result<(), TransformError> {
    let inventory = owner_inventory(shape)?;
    if inventory.curve_entries > limits.max_curve_entries {
        return Err(TransformError::ResourceLimit("LUT curve entries"));
    }
    if inventory.clut_entries > limits.max_clut_entries {
        return Err(TransformError::ResourceLimit("LUT CLUT entries"));
    }
    if inventory.storage_bytes > limits.max_compiled_bytes {
        return Err(TransformError::ResourceLimit("decoded transform bytes"));
    }
    Ok(())
}

fn owner_inventory(shape: &LutShape<'_>) -> Result<LutOwnerInventory, TransformError> {
    let (curve_entries, decoded_bytes, clut_entries) = match shape.layout {
        LutLayout::Mft {
            wide,
            entries_in,
            entries_out,
            clut_range,
            input_range,
            output_range,
            ..
        } => {
            let curve_entries = shape
                .input_channels
                .checked_mul(entries_in)
                .and_then(|value| {
                    value.checked_add(shape.output_channels.checked_mul(entries_out)?)
                })
                .ok_or(TransformError::ResourceLimit("LUT table entry arithmetic"))?;
            let width = if wide { 2 } else { 1 };
            let clut_entries = clut_range
                .size
                .checked_div(width)
                .ok_or(TransformError::ResourceLimit("LUT CLUT size"))?;
            let decoded_entries = input_range
                .size
                .checked_add(clut_range.size)
                .and_then(|value| value.checked_add(output_range.size))
                .and_then(|value| value.checked_div(width))
                .ok_or(TransformError::ResourceLimit("LUT decoded size"))?;
            let owner_bytes = shape
                .input_channels
                .checked_add(shape.output_channels)
                .and_then(|count| count.checked_mul(size_of::<Vec<f32>>()))
                .and_then(|bytes| {
                    bytes.checked_add(shape.input_channels.checked_mul(size_of::<usize>())?)
                })
                .ok_or(TransformError::ResourceLimit("LUT owner storage"))?;
            (
                curve_entries,
                decoded_entries
                    .checked_mul(size_of::<f32>())
                    .and_then(|bytes| bytes.checked_add(owner_bytes))
                    .ok_or(TransformError::ResourceLimit("LUT decoded size"))?,
                clut_entries,
            )
        }
        LutLayout::Mab { clut, .. } => {
            let mut curve_entries = 0usize;
            let mut decoded_bytes = 0usize;
            for set in shape.curves {
                decoded_bytes = decoded_bytes
                    .checked_add(
                        set.count
                            .checked_mul(size_of::<super::curve::Curve>())
                            .ok_or(TransformError::ResourceLimit("LUT owner storage"))?,
                    )
                    .ok_or(TransformError::ResourceLimit("LUT owner storage"))?;
                for plan in set.plans.iter().take(set.count) {
                    curve_entries = curve_entries
                        .checked_add(plan.entries.max(1))
                        .ok_or(TransformError::ResourceLimit("LUT curve entries"))?;
                    decoded_bytes = decoded_bytes
                        .checked_add(plan.decoded_bytes)
                        .ok_or(TransformError::ResourceLimit("LUT decoded size"))?;
                }
            }
            let clut_entries = clut
                .as_ref()
                .map_or(0, |value| value.values.size / value.bytes);
            decoded_bytes = decoded_bytes
                .checked_add(
                    clut_entries
                        .checked_mul(size_of::<f32>())
                        .ok_or(TransformError::ResourceLimit("LUT decoded size"))?,
                )
                .ok_or(TransformError::ResourceLimit("LUT decoded size"))?;
            if let Some(clut) = clut {
                decoded_bytes = decoded_bytes
                    .checked_add(
                        clut.dimensions
                            .checked_mul(size_of::<usize>())
                            .ok_or(TransformError::ResourceLimit("LUT owner storage"))?,
                    )
                    .ok_or(TransformError::ResourceLimit("LUT owner storage"))?;
            }
            (curve_entries, decoded_bytes, clut_entries)
        }
    };
    Ok(LutOwnerInventory {
        curve_entries,
        storage_bytes: decoded_bytes,
        clut_entries,
    })
}

pub(super) fn admit_shape(
    shape: &LutShape<'_>,
    limits: TransformLimits,
    budget: &mut super::compile_budget::CompileBudget,
    owned_headers: usize,
) -> Result<(), TransformError> {
    let inventory = owner_inventory(shape)?;
    if inventory.curve_entries > limits.max_curve_entries {
        return Err(TransformError::ResourceLimit("LUT curve entries"));
    }
    if inventory.clut_entries > limits.max_clut_entries {
        return Err(TransformError::ResourceLimit("LUT CLUT entries"));
    }
    let checkpoint = budget.checkpoint();
    let result = (|| {
        budget.admit_curve(inventory.curve_entries, 0)?;
        budget.admit_clut(inventory.clut_entries)?;
        let total = inventory
            .storage_bytes
            .checked_add(owned_headers)
            .ok_or(TransformError::ResourceLimit("LUT owner storage"))?;
        budget.admit_storage(total, "LUT owner storage")
    })();
    if result.is_err() {
        budget.rollback(checkpoint);
    }
    result
}

fn checked_mft_shape(data: &[u8], pcs: Pcs) -> Result<LutShape<'_>, TransformError> {
    let wide = be_u32(data, 0)? == MFT2;
    let header = if wide { 52 } else { 48 };
    checked_range(data, 0, header)?;
    let input_channels = usize::from(data[8]);
    let output_channels = usize::from(data[9]);
    let grid = usize::from(data[10]);
    validate_channels(input_channels)?;
    validate_channels(output_channels)?;
    if grid < 2 {
        return Err(TransformError::InvalidProfile(
            "mft CLUT grid must contain at least two points",
        ));
    }
    let entries_in = if wide {
        usize::from(be_u16(data, 48)?)
    } else {
        256
    };
    let entries_out = if wide {
        usize::from(be_u16(data, 50)?)
    } else {
        256
    };
    let width = if wide { 2 } else { 1 };
    let input_size = input_channels
        .checked_mul(entries_in)
        .and_then(|value| value.checked_mul(width))
        .ok_or(TransformError::ResourceLimit("LUT input table size"))?;
    let points = checked_pow(grid, input_channels)?;
    let clut_count = points
        .checked_mul(output_channels)
        .ok_or(TransformError::ResourceLimit("LUT CLUT size"))?;
    let clut_size = clut_count
        .checked_mul(width)
        .ok_or(TransformError::ResourceLimit("LUT CLUT size"))?;
    let output_size = output_channels
        .checked_mul(entries_out)
        .and_then(|value| value.checked_mul(width))
        .ok_or(TransformError::ResourceLimit("LUT output table size"))?;
    let clut_offset = header
        .checked_add(input_size)
        .ok_or(TransformError::ResourceLimit("LUT offset arithmetic"))?;
    let output_offset = clut_offset
        .checked_add(clut_size)
        .ok_or(TransformError::ResourceLimit("LUT offset arithmetic"))?;
    checked_range(data, header, input_size)?;
    checked_range(data, clut_offset, clut_size)?;
    checked_range(data, output_offset, output_size)?;
    let mut matrix = [[0.0; 3]; 3];
    for index in 0..9 {
        matrix[index / 3][index % 3] = be_i32(data, 12 + index * 4)? as f32 / 65536.0;
    }
    if pcs == Pcs::Lab
        && matrix.iter().enumerate().any(|(row, values)| {
            values
                .iter()
                .enumerate()
                .any(|(column, value)| *value != if row == column { 1.0 } else { 0.0 })
        })
    {
        return Err(TransformError::InvalidProfile(
            "LUT matrix must be identity for Lab PCS",
        ));
    }
    Ok(LutShape {
        input_channels,
        output_channels,
        curves: [CurveSetShape::default(); 3],
        layout: LutLayout::Mft {
            wide,
            grid,
            entries_in,
            entries_out,
            matrix,
            input_range: EncodedRange {
                offset: header,
                size: input_size,
            },
            clut_range: EncodedRange {
                offset: clut_offset,
                size: clut_size,
            },
            output_range: EncodedRange {
                offset: output_offset,
                size: output_size,
            },
        },
    })
}

fn checked_mab_shape(
    data: &[u8],
    limits: TransformLimits,
    parse_limits: ParseLimits,
) -> Result<LutShape<'_>, TransformError> {
    checked_range(data, 0, 32)?;
    let reverse = be_u32(data, 0)? == MBA;
    let input_channels = usize::from(data[8]);
    let output_channels = usize::from(data[9]);
    validate_channels(input_channels)?;
    validate_channels(output_channels)?;
    let bo = usize::try_from(be_u32(data, 12)?)
        .map_err(|_| TransformError::ResourceLimit("LUT B offset arithmetic"))?;
    let mo = usize::try_from(be_u32(data, 16)?)
        .map_err(|_| TransformError::ResourceLimit("LUT M offset arithmetic"))?;
    let mco = usize::try_from(be_u32(data, 20)?)
        .map_err(|_| TransformError::ResourceLimit("LUT M curve offset arithmetic"))?;
    let co = usize::try_from(be_u32(data, 24)?)
        .map_err(|_| TransformError::ResourceLimit("LUT CLUT offset arithmetic"))?;
    let ao = usize::try_from(be_u32(data, 28)?)
        .map_err(|_| TransformError::ResourceLimit("LUT A offset arithmetic"))?;
    if bo == 0 {
        return Err(TransformError::InvalidProfile("LUT B stage is required"));
    }
    let has_a = ao != 0;
    let has_clut = co != 0;
    let has_m = mco != 0;
    let has_matrix = mo != 0;
    if has_a != has_clut || has_m != has_matrix {
        return Err(TransformError::InvalidProfile(
            "LUT optional stages are not a legal combination",
        ));
    }
    if !has_clut && input_channels != output_channels {
        return Err(TransformError::InvalidProfile(
            "LUT without channel-changing stages has unequal channels",
        ));
    }
    let a = plan_curve_set_shape(
        data,
        ao,
        if reverse {
            output_channels
        } else {
            input_channels
        },
        limits,
        parse_limits,
    )?;
    let b = plan_curve_set_shape(
        data,
        bo,
        if reverse {
            input_channels
        } else {
            output_channels
        },
        limits,
        parse_limits,
    )?;
    let m = plan_curve_set_shape(
        data,
        mco,
        if mco == 0 { 0 } else { 3 },
        limits,
        parse_limits,
    )?;
    let matrix_range = (mo != 0).then_some(EncodedRange {
        offset: mo,
        size: 12 * 4,
    });
    if let Some(range) = matrix_range {
        checked_range(data, range.offset, range.size)?;
    }
    let clut = if co == 0 {
        None
    } else {
        Some(checked_clut_shape(
            data,
            co,
            input_channels,
            output_channels.min(3),
            limits,
        )?)
    };
    Ok(LutShape {
        input_channels,
        output_channels,
        curves: [a, b, m],
        layout: LutLayout::Mab {
            reverse,
            matrix_range,
            clut,
        },
    })
}

fn checked_clut_shape(
    data: &[u8],
    offset: usize,
    dimensions: usize,
    channels: usize,
    limits: TransformLimits,
) -> Result<ClutShape, TransformError> {
    checked_range(data, offset, 20)?;
    let mut grid = [0usize; 16];
    let mut grid_len = 0;
    for index in 0..16 {
        let value = usize::from(data[offset + index]);
        if value == 0 {
            break;
        }
        if value < 2 {
            return Err(TransformError::InvalidProfile("CLUT grid point"));
        }
        grid[index] = value;
        grid_len += 1;
    }
    if grid_len == 0 {
        return Err(TransformError::InvalidProfile("CLUT has no grid"));
    }
    if channels == 0 || channels > 3 || dimensions == 0 || dimensions > grid_len {
        return Err(TransformError::UnsupportedProfileFeature(
            "CLUT channel count",
        ));
    }
    let points = grid[..dimensions]
        .iter()
        .try_fold(1usize, |total, value| total.checked_mul(*value))
        .ok_or(TransformError::ResourceLimit("LUT grid arithmetic"))?;
    let count = points
        .checked_mul(channels)
        .ok_or(TransformError::ResourceLimit("LUT CLUT size"))?;
    if count > limits.max_clut_entries {
        return Err(TransformError::ResourceLimit("LUT CLUT entries"));
    }
    let bytes = match data[offset + 16] {
        1 => 1,
        2 => 2,
        _ => return Err(TransformError::UnsupportedProfileFeature("CLUT precision")),
    };
    let value_offset = offset
        .checked_add(20)
        .ok_or(TransformError::ResourceLimit("LUT CLUT offset arithmetic"))?;
    let value_size = count
        .checked_mul(bytes)
        .ok_or(TransformError::ResourceLimit("LUT CLUT size"))?;
    checked_range(data, value_offset, value_size)?;
    Ok(ClutShape {
        offset,
        dimensions,
        channels,
        grid,
        grid_len,
        bytes,
        values: EncodedRange {
            offset: value_offset,
            size: value_size,
        },
    })
}

fn validate_channels(channels: usize) -> Result<(), TransformError> {
    if channels == 0 || channels > 16 {
        return Err(TransformError::UnsupportedProfileFeature(
            "LUT channel count",
        ));
    }
    Ok(())
}

fn checked_pow(base: usize, exponent: usize) -> Result<usize, TransformError> {
    (0..exponent).try_fold(1usize, |value, _| {
        value
            .checked_mul(base)
            .ok_or(TransformError::ResourceLimit("LUT grid arithmetic"))
    })
}

pub(super) fn plan_lut<'a>(
    data: &'a [u8],
    expected_channels: (usize, usize),
    pcs: Pcs,
    a_to_b: bool,
    limits: TransformLimits,
    parse_limits: ParseLimits,
) -> Result<LutPlan<'a>, TransformError> {
    // Keep this first: callers and the legacy route rely on direction errors
    // taking precedence over channel-shape diagnostics.
    LutTransform::validate_direction(data, a_to_b)?;
    let channels = LutTransform::encoded_channels(data)?;
    if channels != expected_channels {
        return Err(TransformError::MalformedProfile(
            "LUT channels do not match the profile header".into(),
        ));
    }
    let shape = checked_shape(data, limits, pcs, parse_limits)?;
    Ok(LutPlan {
        data,
        shape,
        pcs,
        a_to_b,
        limits,
    })
}

pub(super) fn plan_curve_set_shape<'a>(
    data: &'a [u8],
    offset: usize,
    count: usize,
    limits: TransformLimits,
    parse_limits: ParseLimits,
) -> Result<CurveSetShape<'a>, TransformError> {
    if offset == 0 {
        return Ok(CurveSetShape {
            plans: [CurvePlan::default(); 3],
            count: 0,
        });
    }
    if count > 3 {
        return Err(TransformError::UnsupportedProfileFeature(
            "only Gray/RGB LUTs are supported",
        ));
    }
    let mut plans = [CurvePlan::default(); 3];
    let mut at = offset;
    for plan in plans.iter_mut().take(count) {
        let curve_data = data
            .get(at..)
            .ok_or(TransformError::InvalidProfile("LUT curve range"))?;
        *plan = plan_curve(curve_data, limits)?;
        validate_parse_limit(*plan, parse_limits)?;
        at = at
            .checked_add(curve_encoded_size(curve_data)?)
            .ok_or(TransformError::ResourceLimit("LUT curve offset arithmetic"))?;
    }
    Ok(CurveSetShape { plans, count })
}

impl LutPlan<'_> {
    #[cfg(test)]
    pub(super) fn inventory(
        &self,
        owned_headers: usize,
    ) -> Result<(usize, usize, usize), TransformError> {
        let inventory = owner_inventory(&self.shape)?;
        Ok((
            inventory
                .storage_bytes
                .checked_add(owned_headers)
                .ok_or(TransformError::ResourceLimit("LUT owner storage"))?,
            inventory.curve_entries,
            inventory.clut_entries,
        ))
    }

    pub(super) fn admit(
        &self,
        budget: &mut super::compile_budget::CompileBudget,
        owned_headers: usize,
    ) -> Result<(), TransformError> {
        admit_shape(&self.shape, self.limits, budget, owned_headers)
    }

    #[cfg(test)]
    pub(super) fn materialize(
        &self,
        parse_limits: ParseLimits,
    ) -> Result<LutTransform, TransformError> {
        let mut budget = super::compile_budget::CompileBudget::new(self.limits);
        self.admit(&mut budget, 0)?;
        self.materialize_with_budget(&mut budget, parse_limits, 0)
    }

    pub(super) fn materialize_with_budget(
        &self,
        budget: &mut super::compile_budget::CompileBudget,
        parse_limits: ParseLimits,
        owned_headers: usize,
    ) -> Result<LutTransform, TransformError> {
        let checkpoint = budget.checkpoint();
        let result = LutTransform::parse_planned(
            self.data,
            self.shape,
            parse_limits,
            self.pcs,
            self.a_to_b,
            budget,
        );
        let result = match result {
            Ok(lut) => match budget.commit_owned(owned_headers, owned_headers, "LUT owner headers")
            {
                Ok(()) => Ok(lut),
                Err(error) => {
                    drop(lut);
                    Err(error)
                }
            },
            Err(error) => Err(error),
        };
        if result.is_err() {
            budget.rollback(checkpoint);
        }
        result
    }
}
