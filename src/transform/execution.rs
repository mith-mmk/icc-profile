use super::error::TransformError;
use super::limits::ExecutionLimits;

pub(super) struct OutputAllocation {
    pub(super) len: usize,
    pub(super) bytes: usize,
}

pub(super) fn checked_output_len(
    input_len: usize,
    input_channels: usize,
    output_channels: usize,
) -> Result<usize, TransformError> {
    if input_channels == 0 || output_channels == 0 {
        return Err(TransformError::InvalidProfile(
            "compiled transform has no channels",
        ));
    }
    if !input_len.is_multiple_of(input_channels) {
        return Err(TransformError::InvalidBufferLength {
            expected: input_channels,
            actual: input_len,
        });
    }
    (input_len / input_channels)
        .checked_mul(output_channels)
        .ok_or(TransformError::ResourceLimit("output length"))
}

pub(super) fn checked_output_allocation(
    input_len: usize,
    input_channels: usize,
    output_channels: usize,
    limits: ExecutionLimits,
) -> Result<OutputAllocation, TransformError> {
    let len = checked_output_len(input_len, input_channels, output_channels)?;
    let bytes = len
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or(TransformError::ResourceLimit("transform output bytes"))?;
    if bytes > isize::MAX as usize {
        return Err(TransformError::ResourceLimit(
            "transform output addressable size",
        ));
    }
    if bytes > limits.max_output_bytes {
        return Err(TransformError::ResourceLimit("transform output limit"));
    }
    Ok(OutputAllocation { len, bytes })
}

pub(super) fn try_new_f32_output(
    plan: &OutputAllocation,
    limits: ExecutionLimits,
) -> Result<Vec<f32>, TransformError> {
    try_new_f32_output_with(plan, limits, || {
        let mut output = Vec::new();
        output
            .try_reserve_exact(plan.len)
            .map_err(|_| TransformError::ResourceLimit("transform output allocation"))?;
        Ok(output)
    })
}

pub(super) fn try_new_f32_output_with(
    plan: &OutputAllocation,
    limits: ExecutionLimits,
    make_empty_candidate: impl FnOnce() -> Result<Vec<f32>, TransformError>,
) -> Result<Vec<f32>, TransformError> {
    if plan.bytes > limits.max_output_bytes {
        return Err(TransformError::ResourceLimit("transform output limit"));
    }
    let mut output = make_empty_candidate()?;
    let actual_bytes = output
        .capacity()
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or(TransformError::ResourceLimit("transform output capacity"))?;
    if actual_bytes > isize::MAX as usize || actual_bytes > limits.max_output_bytes {
        drop(output);
        return Err(TransformError::ResourceLimit("transform output capacity"));
    }
    if !output.is_empty() || output.capacity() < plan.len {
        drop(output);
        return Err(TransformError::ResourceLimit("transform output candidate"));
    }
    output.resize(plan.len, 0.0);
    Ok(output)
}
