//! Checked accounting for immutable ICC profile and transform owners.
//!
//! The accounting deliberately measures the bytes requested by the contained
//! `Vec` and `Arc` owners.  Allocator bookkeeping is not part of the public
//! contract, since it is platform and allocator dependent.  Every arithmetic
//! operation is checked so callers can use the result as an admission charge.

use std::mem::size_of;
use std::sync::Arc;

use super::compile::{CompiledProfile, Transform};
use super::curve::Curve;
use super::error::TransformError;
use super::lut::{Clut, LutKind, LutTransform, Table};
use super::profile::{MatrixProfile, Profile, ProfileInner};

/// Owned memory associated with an ICC profile or compiled transform.
///
/// `resident_bytes` is the sum of the profile and compiled portions.  The
/// `build_peak_bytes` value is the checked conservative peak needed when both
/// portions are retained by a caller while a transform is built.  No
/// allocator-specific headers are included.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransformMemory {
    profile_bytes: usize,
    compiled_bytes: usize,
    resident_bytes: usize,
    build_peak_bytes: usize,
}

impl TransformMemory {
    pub(crate) fn from_parts(
        profile_bytes: usize,
        compiled_bytes: usize,
    ) -> Result<Self, TransformError> {
        let resident_bytes = profile_bytes
            .checked_add(compiled_bytes)
            .ok_or(TransformError::ResourceLimit("transform memory accounting"))?;
        Ok(Self {
            profile_bytes,
            compiled_bytes,
            resident_bytes,
            // The caller may retain the input profile while compiling and
            // retain the resulting transform afterwards, so this is the
            // same exact ownership sum rather than an unbounded estimate.
            build_peak_bytes: resident_bytes,
        })
    }

    pub fn profile_bytes(self) -> usize {
        self.profile_bytes
    }

    pub fn compiled_bytes(self) -> usize {
        self.compiled_bytes
    }

    pub fn resident_bytes(self) -> usize {
        self.resident_bytes
    }

    pub fn build_peak_bytes(self) -> usize {
        self.build_peak_bytes
    }

    /// Checkedly combine ownership charges from independently retained CMS
    /// objects.  This is useful to callers that retain two profiles and a
    /// transform during construction.
    pub fn checked_add(self, other: Self) -> Result<Self, TransformError> {
        let profile_bytes = self
            .profile_bytes
            .checked_add(other.profile_bytes)
            .ok_or(TransformError::ResourceLimit("transform memory accounting"))?;
        let compiled_bytes = self
            .compiled_bytes
            .checked_add(other.compiled_bytes)
            .ok_or(TransformError::ResourceLimit("transform memory accounting"))?;
        Self::from_parts(profile_bytes, compiled_bytes)
    }
}

impl Profile {
    /// Return the checked resident size of this immutable profile owner.
    pub fn memory_usage(&self) -> Result<TransformMemory, TransformError> {
        TransformMemory::from_parts(profile_memory(&self.0)?, 0)
    }
}

impl CompiledProfile {
    /// Return the checked resident size of the compiled direction.
    pub fn memory_usage(&self) -> Result<TransformMemory, TransformError> {
        TransformMemory::from_parts(0, compiled_direction_memory(&self.0)?)
    }
}

impl Transform {
    /// Return the checked resident size of the source profiles and immutable
    /// CMS stages held by this transform. Shared profile/matrix/LUT `Arc`
    /// allocations are counted once.
    pub fn memory_usage(&self) -> Result<TransformMemory, TransformError> {
        let mut total = 0usize;
        let mut matrices: [*const MatrixProfile; 2] = [std::ptr::null(); 2];
        let mut matrix_count = 0usize;
        for matrix in [self.input.as_ref(), self.output.as_ref()]
            .into_iter()
            .flatten()
        {
            let pointer = Arc::as_ptr(matrix);
            if matrices[..matrix_count].contains(&pointer) {
                continue;
            }
            matrices[matrix_count] = pointer;
            matrix_count += 1;
            total = total
                .checked_add(matrix_memory(matrix)?)
                .ok_or(TransformError::ResourceLimit("transform memory accounting"))?;
        }
        let mut luts: [*const LutTransform; 2] = [std::ptr::null(); 2];
        let mut lut_count = 0usize;
        for lut in [self.lut_input.as_ref(), self.lut_output.as_ref()]
            .into_iter()
            .flatten()
        {
            let pointer = Arc::as_ptr(lut);
            if luts[..lut_count].contains(&pointer) {
                continue;
            }
            luts[lut_count] = pointer;
            lut_count += 1;
            total = total
                .checked_add(lut_memory(lut)?)
                .ok_or(TransformError::ResourceLimit("transform memory accounting"))?;
        }
        let profile_bytes = if Arc::ptr_eq(&self.input_profile.0, &self.output_profile.0) {
            profile_memory(&self.input_profile.0)?
        } else {
            profile_memory(&self.input_profile.0)?
                .checked_add(profile_memory(&self.output_profile.0)?)
                .ok_or(TransformError::ResourceLimit("transform memory accounting"))?
        };
        TransformMemory::from_parts(profile_bytes, total)
    }

    /// Account for a build in which the two source profiles are retained
    /// alongside this compiled transform.  Passing the same profile twice is
    /// charged once because both handles point at the same immutable `Arc`.
    pub fn memory_usage_with_profiles(
        &self,
        input: &Profile,
        output: &Profile,
    ) -> Result<TransformMemory, TransformError> {
        if !Arc::ptr_eq(&input.0, &self.input_profile.0)
            || !Arc::ptr_eq(&output.0, &self.output_profile.0)
        {
            return Err(TransformError::InvalidProfile(
                "memory profiles do not match the profiles used to build the transform",
            ));
        }
        self.memory_usage()
    }
}

fn vec_bytes<T>(capacity: usize) -> Result<usize, TransformError> {
    capacity
        .checked_mul(size_of::<T>())
        .ok_or(TransformError::ResourceLimit("transform memory accounting"))
}

fn add(total: &mut usize, value: usize) -> Result<(), TransformError> {
    *total = total
        .checked_add(value)
        .ok_or(TransformError::ResourceLimit("transform memory accounting"))?;
    Ok(())
}

fn profile_memory(profile: &ProfileInner) -> Result<usize, TransformError> {
    let mut total = size_of::<ProfileInner>();
    add(&mut total, vec_bytes::<u8>(profile.data.capacity())?)?;
    add(
        &mut total,
        vec_bytes::<(u32, usize, usize)>(profile.tags.capacity())?,
    )?;
    Ok(total)
}

fn compiled_direction_memory(
    direction: &super::compile::CompiledDirection,
) -> Result<usize, TransformError> {
    let mut total = size_of::<super::compile::CompiledDirection>();
    if let Some(matrix) = &direction.matrix {
        add(&mut total, matrix_memory(matrix)?)?;
    }
    if let Some(lut) = &direction.lut {
        add(&mut total, lut_memory(lut)?)?;
    }
    Ok(total)
}

fn curve_memory(curve: &Curve) -> Result<usize, TransformError> {
    match curve {
        Curve::Identity | Curve::Gamma(_) => Ok(0),
        Curve::Table(values) => vec_bytes::<f32>(values.capacity()),
        Curve::Para { values, .. } => vec_bytes::<f32>(values.capacity()),
    }
}

fn matrix_memory(matrix: &MatrixProfile) -> Result<usize, TransformError> {
    let mut total = size_of::<MatrixProfile>();
    add(&mut total, vec_bytes::<Curve>(matrix.curves.capacity())?)?;
    for curve in &matrix.curves {
        add(&mut total, curve_memory(curve)?)?;
    }
    Ok(total)
}

fn lut_memory(lut: &LutTransform) -> Result<usize, TransformError> {
    let mut total = size_of::<LutTransform>();
    match &lut.kind {
        LutKind::Mft {
            input,
            clut,
            output,
            ..
        } => {
            add(&mut total, vec_bytes::<Table>(input.capacity())?)?;
            for table in input {
                add(&mut total, vec_bytes::<f32>(table.0.capacity())?)?;
            }
            add(&mut total, clut_memory(clut)?)?;
            add(&mut total, vec_bytes::<Table>(output.capacity())?)?;
            for table in output {
                add(&mut total, vec_bytes::<f32>(table.0.capacity())?)?;
            }
        }
        LutKind::Mab {
            a,
            clut,
            m,
            matrix: _,
            b,
            ..
        } => {
            add(&mut total, vec_bytes::<Curve>(a.capacity())?)?;
            for curve in a {
                add(&mut total, curve_memory(curve)?)?;
            }
            if let Some(clut) = clut {
                add(&mut total, clut_memory(clut)?)?;
            }
            add(&mut total, vec_bytes::<Curve>(m.capacity())?)?;
            for curve in m {
                add(&mut total, curve_memory(curve)?)?;
            }
            // `Matrix` is an inline `Option` field of `LutKind`; it has no
            // independent allocation to charge.
            add(&mut total, vec_bytes::<Curve>(b.capacity())?)?;
            for curve in b {
                add(&mut total, curve_memory(curve)?)?;
            }
        }
    }
    Ok(total)
}

fn clut_memory(clut: &Clut) -> Result<usize, TransformError> {
    // The `Clut` descriptor is embedded in `LutKind`; only its heap-backed
    // vectors are separate owners.  Counting the descriptor here would
    // diverge from CompileBudget's stage-header inventory.
    let mut total = 0usize;
    add(&mut total, vec_bytes::<usize>(clut.grid.capacity())?)?;
    add(&mut total, vec_bytes::<f32>(clut.values.capacity())?)?;
    Ok(total)
}
