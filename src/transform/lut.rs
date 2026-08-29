//! Checked ICC LUT tag parsing and evaluation.
//!
//! This module intentionally has no dependency on the legacy `Data` decoder.
//! A compiled LUT owns only validated numbers and is immutable, so a
//! `Transform` remains `Send + Sync` and worker scratch stays per caller.

use std::mem::size_of;

use super::compile_budget::CompileBudget;
use super::curve::Curve;
use super::curve_plan::materialize_curve;
use super::error::TransformError;
use super::limits::ParseLimits;
#[cfg(test)]
use super::limits::TransformLimits;
use super::lut_plan::{ClutShape, CurveSetShape, LutLayout, LutShape};
use super::reader::{be_i32, be_u16, be_u32, checked_range};

const MFT1: u32 = u32::from_be_bytes(*b"mft1");
const MFT2: u32 = u32::from_be_bytes(*b"mft2");
const MAB: u32 = u32::from_be_bytes(*b"mAB ");
const MBA: u32 = u32::from_be_bytes(*b"mBA ");

#[derive(Clone, Debug)]
pub(super) struct LutTransform {
    pub(super) input_channels: usize,
    pub(super) output_channels: usize,
    pub(super) kind: LutKind,
    pcs_encoding: PcsEncoding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PcsEncoding {
    LegacyXyz16,
    LegacyLab16,
    ModernXyz,
    ModernLab,
}

#[derive(Clone, Debug)]
pub(super) enum LutKind {
    Mft {
        matrix: [[f32; 3]; 3],
        input: Vec<Table>,
        clut: Clut,
        output: Vec<Table>,
    },
    Mab {
        a: Vec<Curve>,
        clut: Option<Clut>,
        m: Vec<Curve>,
        matrix: Option<Matrix>,
        b: Vec<Curve>,
        reverse: bool,
    },
}

#[derive(Clone, Debug)]
pub(super) struct Table(pub(super) Vec<f32>);

#[derive(Clone, Debug)]
pub(super) struct Matrix {
    values: [[f32; 3]; 3],
    offset: [f32; 3],
}

#[derive(Clone, Debug)]
pub(super) struct Clut {
    pub(super) grid: Vec<usize>,
    pub(super) channels: usize,
    pub(super) values: Vec<f32>,
}

/// The allocation-free structural description consumed by the native LUT
/// materializer.  All offsets and stage combinations have already passed the
impl LutTransform {
    pub(super) fn encoded_channels(data: &[u8]) -> Result<(usize, usize), TransformError> {
        checked_range(data, 0, 10)?;
        match be_u32(data, 0)? {
            MFT1 | MFT2 | MAB | MBA => {
                let input = usize::from(data[8]);
                let output = usize::from(data[9]);
                validate_channels(input)?;
                validate_channels(output)?;
                Ok((input, output))
            }
            _ => Err(TransformError::UnsupportedProfileFeature("LUT tag type")),
        }
    }

    /// Compatibility parser retained for the private test/legacy harness.
    /// Production compilation uses the checked planning path below.
    #[cfg(test)]
    pub(super) fn parse(
        data: &[u8],
        limits: ParseLimits,
        pcs: super::profile::Pcs,
        a_to_b: bool,
    ) -> Result<Self, TransformError> {
        if data.len() < 4 {
            return Err(TransformError::InvalidProfile("LUT tag is truncated"));
        }
        super::lut_plan::check_encoded_limits(data, TransformLimits::default())?;
        match be_u32(data, 0)? {
            MFT1 => parse_mft(data, false, limits, pcs),
            MFT2 => parse_mft(data, true, limits, pcs),
            MAB if a_to_b => parse_mab(data, limits, pcs),
            MBA if !a_to_b => parse_mab(data, limits, pcs),
            MAB | MBA => Err(TransformError::InvalidProfile(
                "LUT tag direction does not match A2B/B2A",
            )),
            _ => Err(TransformError::UnsupportedProfileFeature("LUT tag type")),
        }
    }

    pub(super) fn parse_planned(
        data: &[u8],
        shape: LutShape<'_>,
        limits: ParseLimits,
        pcs: super::profile::Pcs,
        a_to_b: bool,
        budget: &mut CompileBudget,
    ) -> Result<Self, TransformError> {
        match shape.layout {
            LutLayout::Mft { .. } => parse_mft_planned(data, shape, limits, pcs, budget),
            LutLayout::Mab { reverse, .. } => {
                let expected_reverse = !a_to_b;
                if reverse != expected_reverse {
                    return Err(TransformError::InvalidProfile(
                        "LUT tag direction does not match A2B/B2A",
                    ));
                }
                parse_mab_planned(data, shape, limits, pcs, budget)
            }
        }
    }

    pub(super) fn validate_direction(data: &[u8], a_to_b: bool) -> Result<(), TransformError> {
        if data.len() < 4 {
            return Err(TransformError::InvalidProfile("LUT tag is truncated"));
        }
        match be_u32(data, 0)? {
            MAB if !a_to_b => Err(TransformError::InvalidProfile(
                "mAB tag is only valid for A2B direction",
            )),
            MBA if a_to_b => Err(TransformError::InvalidProfile(
                "mBA tag is only valid for B2A direction",
            )),
            _ => Ok(()),
        }
    }

    pub(super) fn pcs_encoding(&self) -> PcsEncoding {
        self.pcs_encoding
    }

    pub(super) fn eval_with_domain(
        &self,
        input: &[f32],
        output: &mut [f32],
        clamp: bool,
    ) -> Result<(), TransformError> {
        if input.len() != self.input_channels || output.len() != self.output_channels {
            return Err(TransformError::InvalidBufferLength {
                expected: self.input_channels,
                actual: input.len(),
            });
        }
        if input.iter().any(|v| !v.is_finite()) {
            return Err(TransformError::NonFiniteInput);
        }
        if !clamp && input.iter().any(|value| !(0.0..=1.0).contains(value)) {
            return Err(TransformError::MalformedProfile(
                "LUT input is outside the normalized domain".into(),
            ));
        }
        let mut normalized = [0.0; 3];
        normalized[..input.len()].copy_from_slice(input);
        if clamp {
            for value in &mut normalized[..input.len()] {
                *value = value.clamp(0.0, 1.0);
            }
        }
        let input = &normalized[..input.len()];
        match &self.kind {
            LutKind::Mft {
                matrix,
                input: it,
                clut,
                output: ot,
            } => {
                let mut v = [0.0; 3];
                if self.input_channels == 1 {
                    v[0] = input[0];
                } else {
                    for r in 0..3 {
                        v[r] = matrix[r][0] * input[0]
                            + matrix[r][1] * input[1]
                            + matrix[r][2] * input[2];
                    }
                }
                let mut values = [0.0; 3];
                for c in 0..self.input_channels {
                    values[c] = table_eval(
                        &it[c],
                        if self.input_channels == 1 { v[0] } else { v[c] },
                        clamp,
                    )?;
                }
                let mut clut_out = [0.0; 3];
                clut.eval(
                    &values[..self.input_channels],
                    &mut clut_out[..clut.channels],
                    clamp,
                )?;
                for c in 0..self.output_channels {
                    output[c] = table_eval(&ot[c], clut_out[c], clamp)?;
                }
            }
            LutKind::Mab {
                a,
                clut,
                m,
                matrix,
                b,
                reverse,
            } => {
                let mut values = [0.0; 3];
                values[..input.len()].copy_from_slice(input);
                if !*reverse {
                    apply_curves(a, &mut values[..input.len()], clamp)?;
                    if let Some(clut) = clut {
                        let mut next = [0.0; 3];
                        clut.eval(&values[..input.len()], &mut next[..clut.channels], clamp)?;
                        values = next;
                    }
                    apply_curves(
                        m,
                        &mut values[..m.len().max(clut.as_ref().map_or(0, |c| c.channels))],
                        clamp,
                    )?;
                    apply_matrix(matrix.as_ref(), &mut values);
                    clip_values(&mut values[..b.len().min(3)], clamp)?;
                    apply_curves(b, &mut values[..b.len()], clamp)?;
                } else {
                    apply_curves(b, &mut values[..input.len()], clamp)?;
                    apply_matrix(matrix.as_ref(), &mut values);
                    clip_values(&mut values[..m.len().min(3)], clamp)?;
                    apply_curves(m, &mut values[..m.len()], clamp)?;
                    if let Some(clut) = clut {
                        let mut next = [0.0; 3];
                        let clut_input = m.len().max(input.len());
                        clut.eval(&values[..clut_input], &mut next[..clut.channels], clamp)?;
                        values = next;
                    }
                    apply_curves(a, &mut values[..a.len()], clamp)?;
                }
                if output.len() != self.output_channels {
                    return Err(TransformError::MalformedProfile(
                        "LUT stage channel mismatch".into(),
                    ));
                }
                if values[..output.len()]
                    .iter()
                    .any(|value| !value.is_finite())
                {
                    return Err(TransformError::MalformedProfile(
                        "LUT produced a non-finite value".into(),
                    ));
                }
                output.copy_from_slice(&values[..output.len()]);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
fn parse_mft(
    data: &[u8],
    wide: bool,
    limits: ParseLimits,
    pcs: super::profile::Pcs,
) -> Result<LutTransform, TransformError> {
    let shape = super::lut_plan::checked_shape(data, TransformLimits::default(), pcs, limits)?;
    let LutShape {
        layout: LutLayout::Mft {
            wide: planned_wide, ..
        },
        ..
    } = shape
    else {
        return Err(TransformError::InvalidProfile("LUT shape kind mismatch"));
    };
    if planned_wide != wide {
        return Err(TransformError::InvalidProfile("LUT shape kind changed"));
    }
    let mut budget = CompileBudget::new(TransformLimits::default());
    super::lut_plan::admit_shape(&shape, TransformLimits::default(), &mut budget, 0)?;
    parse_mft_planned(data, shape, limits, pcs, &mut budget)
}

fn parse_mft_planned(
    data: &[u8],
    shape: LutShape<'_>,
    limits: ParseLimits,
    pcs: super::profile::Pcs,
    budget: &mut CompileBudget,
) -> Result<LutTransform, TransformError> {
    let LutShape {
        input_channels,
        output_channels,
        layout:
            LutLayout::Mft {
                wide,
                grid,
                entries_in,
                entries_out,
                matrix,
                input_range,
                clut_range,
                output_range,
            },
        ..
    } = shape
    else {
        return Err(TransformError::InvalidProfile("LUT shape kind mismatch"));
    };
    checked_range(data, input_range.offset, input_range.size)?;
    checked_range(data, clut_range.offset, clut_range.size)?;
    checked_range(data, output_range.offset, output_range.size)?;
    if usize::from(data[8]) != input_channels
        || usize::from(data[9]) != output_channels
        || usize::from(data[10]) != grid
    {
        return Err(TransformError::InvalidProfile(
            "LUT shape changed after planning",
        ));
    }
    if entries_in < 2
        || entries_out < 2
        || matrix.iter().any(|row| row.iter().any(|v| !v.is_finite()))
    {
        return Err(TransformError::InvalidProfile("LUT shape is invalid"));
    }
    if entries_in > limits.max_curve_entries || entries_out > limits.max_curve_entries {
        return Err(TransformError::ResourceLimit("LUT table entries"));
    }
    let width = if wide { 2 } else { 1 };
    let clut_count = clut_range
        .size
        .checked_div(width)
        .ok_or(TransformError::ResourceLimit("LUT CLUT size"))?;
    if clut_range.size % width != 0
        || input_range
            .size
            .checked_add(clut_range.size)
            .and_then(|size| size.checked_add(output_range.size))
            .ok_or(TransformError::ResourceLimit("LUT size"))?
            > limits.max_tag_size
    {
        return Err(TransformError::ResourceLimit("LUT allocation"));
    }
    let grid_points = grid;
    (|| {
        let scale = if wide { 65535.0 } else { 255.0 };
        let mut input_at = input_range.offset;
        let input = read_tables(
            data,
            &mut input_at,
            input_channels,
            entries_in,
            wide,
            scale,
            budget,
        )?;
        let clut_bytes = clut_count
            .checked_mul(size_of::<f32>())
            .ok_or(TransformError::ResourceLimit("LUT CLUT allocation"))?;
        let mut values =
            budget.try_new_vec::<f32>(clut_count, clut_bytes, "LUT CLUT allocation")?;
        let mut clut_at = clut_range.offset;
        for _ in 0..clut_count {
            values.push(if wide {
                be_u16(data, clut_at)? as f32 / 65535.0
            } else {
                data[clut_at] as f32 / 255.0
            });
            clut_at = clut_at
                .checked_add(width)
                .ok_or(TransformError::ResourceLimit("LUT CLUT offset arithmetic"))?;
        }
        let mut output_at = output_range.offset;
        let output = read_tables(
            data,
            &mut output_at,
            output_channels,
            entries_out,
            wide,
            scale,
            budget,
        )?;
        let grid_bytes = input_channels
            .checked_mul(size_of::<usize>())
            .ok_or(TransformError::ResourceLimit("LUT grid allocation"))?;
        let mut grid =
            budget.try_new_vec::<usize>(input_channels, grid_bytes, "LUT grid allocation")?;
        for _ in 0..input_channels {
            grid.push(grid_points);
        }
        Ok(LutTransform {
            input_channels,
            output_channels,
            kind: LutKind::Mft {
                matrix,
                input,
                clut: Clut {
                    grid,
                    channels: output_channels,
                    values,
                },
                output,
            },
            pcs_encoding: if pcs == super::profile::Pcs::Xyz {
                PcsEncoding::LegacyXyz16
            } else if wide {
                PcsEncoding::LegacyLab16
            } else {
                PcsEncoding::ModernLab
            },
        })
    })()
}

#[cfg(test)]
fn parse_mab(
    data: &[u8],
    limits: ParseLimits,
    pcs: super::profile::Pcs,
) -> Result<LutTransform, TransformError> {
    let limits_for_shape = TransformLimits::default();
    let shape = super::lut_plan::checked_shape(data, limits_for_shape, pcs, limits)?;
    let mut budget = CompileBudget::new(TransformLimits::default());
    super::lut_plan::admit_shape(&shape, TransformLimits::default(), &mut budget, 0)?;
    parse_mab_planned(data, shape, limits, pcs, &mut budget)
}

fn parse_mab_planned(
    data: &[u8],
    shape: LutShape<'_>,
    limits: ParseLimits,
    pcs: super::profile::Pcs,
    budget: &mut CompileBudget,
) -> Result<LutTransform, TransformError> {
    let LutShape {
        input_channels,
        output_channels,
        curves: [a, b, m],
        layout:
            LutLayout::Mab {
                reverse,
                matrix_range,
                clut,
            },
    } = shape
    else {
        return Err(TransformError::InvalidProfile("LUT shape kind mismatch"));
    };
    let a = materialize_curve_set(a, budget)?;
    let b = materialize_curve_set(b, budget)?;
    let matrix = if let Some(matrix_range) = matrix_range {
        checked_range(data, matrix_range.offset, matrix_range.size)?;
        Some(parse_matrix_stage(data, matrix_range.offset)?)
    } else {
        None
    };
    let m = materialize_curve_set(m, budget)?;
    let clut = if let Some(clut_shape) = clut {
        Some(parse_clut_planned(data, clut_shape, limits, budget)?)
    } else {
        None
    };
    Ok(LutTransform {
        input_channels,
        output_channels,
        kind: LutKind::Mab {
            a,
            clut,
            m,
            matrix,
            b,
            reverse,
        },
        pcs_encoding: if pcs == super::profile::Pcs::Xyz {
            PcsEncoding::ModernXyz
        } else {
            PcsEncoding::ModernLab
        },
    })
}

fn materialize_curve_set(
    shape: CurveSetShape<'_>,
    budget: &mut CompileBudget,
) -> Result<Vec<Curve>, TransformError> {
    let outer_bytes = shape
        .count
        .checked_mul(size_of::<Curve>())
        .ok_or(TransformError::ResourceLimit("curve allocation"))?;
    let mut curves = budget.try_new_vec::<Curve>(shape.count, outer_bytes, "curve allocation")?;
    for plan in shape.plans.iter().take(shape.count) {
        curves.push(materialize_curve(*plan, budget, false)?);
    }
    Ok(curves)
}

fn parse_clut_planned(
    data: &[u8],
    shape: ClutShape,
    limits: ParseLimits,
    budget: &mut CompileBudget,
) -> Result<Clut, TransformError> {
    checked_range(data, shape.offset, 20)?;
    if shape.dimensions == 0 || shape.dimensions > shape.grid_len {
        return Err(TransformError::InvalidProfile(
            "CLUT shape changed after planning",
        ));
    }
    let count = shape
        .values
        .size
        .checked_div(shape.bytes)
        .ok_or(TransformError::ResourceLimit("LUT CLUT size"))?;
    if !shape.values.size.is_multiple_of(shape.bytes) || shape.values.size > limits.max_tag_size {
        return Err(TransformError::ResourceLimit("CLUT allocation"));
    }
    let grid_bytes = shape
        .dimensions
        .checked_mul(size_of::<usize>())
        .ok_or(TransformError::ResourceLimit("CLUT grid allocation"))?;
    let value_bytes = count
        .checked_mul(size_of::<f32>())
        .ok_or(TransformError::ResourceLimit("CLUT allocation"))?;
    let mut grid =
        budget.try_new_vec::<usize>(shape.dimensions, grid_bytes, "CLUT grid allocation")?;
    grid.extend_from_slice(&shape.grid[..shape.dimensions]);
    let mut values = budget.try_new_vec::<f32>(count, value_bytes, "CLUT allocation")?;
    let mut at = shape.values.offset;
    for _ in 0..count {
        values.push(if shape.bytes == 1 {
            data[at] as f32 / 255.0
        } else {
            be_u16(data, at)? as f32 / 65535.0
        });
        at = at
            .checked_add(shape.bytes)
            .ok_or(TransformError::ResourceLimit("CLUT offset arithmetic"))?;
    }
    Ok(Clut {
        grid,
        channels: shape.channels,
        values,
    })
}

fn parse_matrix_stage(data: &[u8], offset: usize) -> Result<Matrix, TransformError> {
    checked_range(data, offset, 12 * 4)?;
    let mut n = [0.0; 12];
    for (i, value) in n.iter_mut().enumerate() {
        *value = be_i32(data, offset + i * 4)? as f32 / 65536.0;
    }
    Ok(Matrix {
        values: [[n[0], n[1], n[2]], [n[3], n[4], n[5]], [n[6], n[7], n[8]]],
        offset: [n[9], n[10], n[11]],
    })
}

fn validate_channels(channels: usize) -> Result<(), TransformError> {
    if matches!(channels, 1 | 3) {
        Ok(())
    } else {
        Err(TransformError::UnsupportedProfileFeature(
            "only Gray/RGB LUTs are supported",
        ))
    }
}

fn read_tables(
    data: &[u8],
    p: &mut usize,
    channels: usize,
    entries: usize,
    wide: bool,
    scale: f32,
    budget: &mut CompileBudget,
) -> Result<Vec<Table>, TransformError> {
    let outer_bytes = channels
        .checked_mul(size_of::<Table>())
        .ok_or(TransformError::ResourceLimit("LUT table allocation"))?;
    let mut result = budget.try_new_vec::<Table>(channels, outer_bytes, "LUT table allocation")?;
    for _ in 0..channels {
        let mut table = budget.try_new_vec::<f32>(
            entries,
            entries
                .checked_mul(size_of::<f32>())
                .ok_or(TransformError::ResourceLimit("LUT table allocation"))?,
            "LUT table allocation",
        )?;
        for _ in 0..entries {
            table.push(if wide {
                let v = be_u16(data, *p)? as f32 / scale;
                *p += 2;
                v
            } else {
                let v = data[*p] as f32 / scale;
                *p += 1;
                v
            });
        }
        result.push(Table(table));
    }
    Ok(result)
}

fn table_eval(table: &Table, x: f32, clamp: bool) -> Result<f32, TransformError> {
    if !x.is_finite() {
        return Err(TransformError::NonFiniteInput);
    }
    if !clamp && !(0.0..=1.0).contains(&x) {
        return Err(TransformError::MalformedProfile(
            "LUT table input is outside the normalized domain".into(),
        ));
    }
    let x = if clamp { x.clamp(0.0, 1.0) } else { x };
    if table.0.len() < 2 {
        return Ok(table.0.first().copied().unwrap_or(x));
    }
    let p = x.clamp(0.0, 1.0) * (table.0.len() - 1) as f32;
    let i = p.floor() as usize;
    let j = (i + 1).min(table.0.len() - 1);
    Ok(table.0[i] + (table.0[j] - table.0[i]) * (p - i as f32))
}
fn apply_curves(curves: &[Curve], values: &mut [f32], clamp: bool) -> Result<(), TransformError> {
    for (v, c) in values.iter_mut().zip(curves) {
        if !v.is_finite() {
            return Err(TransformError::NonFiniteInput);
        }
        if !clamp && !(0.0..=1.0).contains(v) {
            return Err(TransformError::MalformedProfile(
                "LUT curve input is outside the normalized domain".into(),
            ));
        }
        let input = if clamp { (*v).clamp(0.0, 1.0) } else { *v };
        *v = c.eval_unclamped(input);
        if !v.is_finite() {
            return Err(TransformError::MalformedProfile(
                "curve produced a non-finite value".into(),
            ));
        }
    }
    Ok(())
}
fn apply_matrix(matrix: Option<&Matrix>, values: &mut [f32]) {
    if let Some(m) = matrix {
        if values.len() >= 3 {
            let v = [values[0], values[1], values[2]];
            for (r, value) in values.iter_mut().enumerate().take(3) {
                *value = m.values[r][0] * v[0]
                    + m.values[r][1] * v[1]
                    + m.values[r][2] * v[2]
                    + m.offset[r];
            }
        }
    }
}
fn clip_values(values: &mut [f32], clamp: bool) -> Result<(), TransformError> {
    for value in values {
        if !value.is_finite() {
            return Err(TransformError::MalformedProfile(
                "LUT stage produced a non-finite value".into(),
            ));
        }
        if clamp {
            *value = value.clamp(0.0, 1.0);
        } else if !(0.0..=1.0).contains(value) {
            return Err(TransformError::MalformedProfile(
                "LUT stage value is outside the normalized domain".into(),
            ));
        }
    }
    Ok(())
}

impl Clut {
    fn eval(&self, input: &[f32], output: &mut [f32], clamp: bool) -> Result<(), TransformError> {
        if input.iter().any(|value| !value.is_finite()) {
            return Err(TransformError::NonFiniteInput);
        }
        if !clamp && input.iter().any(|value| !(0.0..=1.0).contains(value)) {
            return Err(TransformError::MalformedProfile(
                "CLUT input is outside the normalized domain".into(),
            ));
        }
        let mut normalized = [0.0; 3];
        normalized[..input.len()].copy_from_slice(input);
        if clamp {
            for value in &mut normalized[..input.len()] {
                *value = value.clamp(0.0, 1.0);
            }
        }
        let input = &normalized[..input.len()];
        if self.grid.len() == 1 {
            let p =
                input.first().copied().unwrap_or(0.0).clamp(0.0, 1.0) * (self.grid[0] - 1) as f32;
            let i = p.floor() as usize;
            let j = (i + 1).min(self.grid[0] - 1);
            let f = p - i as f32;
            for (c, value) in output.iter_mut().enumerate().take(self.channels) {
                *value = self.values[i * self.channels + c] * (1.0 - f)
                    + self.values[j * self.channels + c] * f;
            }
            return Ok(());
        }
        let x = [
            input[0].clamp(0.0, 1.0) * (self.grid[0] - 1) as f32,
            input[1].clamp(0.0, 1.0) * (self.grid[1] - 1) as f32,
            input[2].clamp(0.0, 1.0) * (self.grid[2] - 1) as f32,
        ];
        let i = [
            x[0].floor() as usize,
            x[1].floor() as usize,
            x[2].floor() as usize,
        ];
        let i1 = [
            i[0].min(self.grid[0] - 2),
            i[1].min(self.grid[1] - 2),
            i[2].min(self.grid[2] - 2),
        ];
        let f = [
            x[0] - i1[0] as f32,
            x[1] - i1[1] as f32,
            x[2] - i1[2] as f32,
        ];
        let (weight, corner) = if f[0] >= f[1] {
            if f[1] >= f[2] {
                (
                    [1.0 - f[0], f[0] - f[1], f[1] - f[2], f[2]],
                    [
                        i1,
                        [i1[0] + 1, i1[1], i1[2]],
                        [i1[0] + 1, i1[1] + 1, i1[2]],
                        [i1[0] + 1, i1[1] + 1, i1[2] + 1],
                    ],
                )
            } else if f[0] >= f[2] {
                (
                    [1.0 - f[0], f[0] - f[2], f[2] - f[1], f[1]],
                    [
                        i1,
                        [i1[0] + 1, i1[1], i1[2]],
                        [i1[0] + 1, i1[1], i1[2] + 1],
                        [i1[0] + 1, i1[1] + 1, i1[2] + 1],
                    ],
                )
            } else {
                (
                    [1.0 - f[2], f[2] - f[0], f[0] - f[1], f[1]],
                    [
                        i1,
                        [i1[0], i1[1], i1[2] + 1],
                        [i1[0] + 1, i1[1], i1[2] + 1],
                        [i1[0] + 1, i1[1] + 1, i1[2] + 1],
                    ],
                )
            }
        } else if f[0] >= f[2] {
            (
                [1.0 - f[1], f[1] - f[0], f[0] - f[2], f[2]],
                [
                    i1,
                    [i1[0], i1[1] + 1, i1[2]],
                    [i1[0] + 1, i1[1] + 1, i1[2]],
                    [i1[0] + 1, i1[1] + 1, i1[2] + 1],
                ],
            )
        } else if f[1] >= f[2] {
            (
                [1.0 - f[1], f[1] - f[2], f[2] - f[0], f[0]],
                [
                    i1,
                    [i1[0], i1[1] + 1, i1[2]],
                    [i1[0], i1[1] + 1, i1[2] + 1],
                    [i1[0] + 1, i1[1] + 1, i1[2] + 1],
                ],
            )
        } else {
            (
                [1.0 - f[2], f[2] - f[1], f[1] - f[0], f[0]],
                [
                    i1,
                    [i1[0], i1[1], i1[2] + 1],
                    [i1[0], i1[1] + 1, i1[2] + 1],
                    [i1[0] + 1, i1[1] + 1, i1[2] + 1],
                ],
            )
        };
        for (c, value) in output.iter_mut().enumerate().take(self.channels) {
            *value = (0..4)
                .map(|n| self.values[self.index(corner[n]) * self.channels + c] * weight[n])
                .sum();
        }
        Ok(())
    }
    fn index(&self, point: [usize; 3]) -> usize {
        point[2] + self.grid[2] * (point[1] + self.grid[1] * point[0])
    }
}
