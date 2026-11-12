use super::compile::Transform;
use super::error::TransformError;

const CHUNK_PIXELS: usize = 64;
const MAX_CHANNELS: usize = 3;

pub struct TransformWorker {
    pub(super) transform: Transform,
}

impl TransformWorker {
    /// Numerical choices inherited when this worker was created.
    pub fn algorithm_profile(&self) -> super::algorithm::AlgorithmProfile {
        self.transform.algorithm_profile()
    }

    /// One-pass execution; a late error may leave output partially updated.
    /// Use `transform_f32_atomic` for unchanged output on every error.
    pub fn transform_f32(
        &mut self,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<(), TransformError> {
        self.transform.transform_f32(input, output)
    }

    /// See [`Transform::transform_f32_atomic`] for the unchanged-on-error,
    /// fixed-scratch, two-pass execution contract.
    pub fn transform_f32_atomic(
        &mut self,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<(), TransformError> {
        self.transform.transform_f32_atomic(input, output)
    }

    /// Already transactional; every returned error leaves output unchanged.
    pub fn transform_u8(&mut self, input: &[u8], output: &mut [u8]) -> Result<(), TransformError> {
        transform_u8(&self.transform, input, output)
    }

    /// Already transactional; every returned error leaves output unchanged.
    pub fn transform_u16(
        &mut self,
        input: &[u16],
        output: &mut [u16],
    ) -> Result<(), TransformError> {
        transform_u16(&self.transform, input, output)
    }
    /// Transactional alias; same fixed scratch and two passes as `transform_u8`.
    pub fn transform_u8_atomic(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<(), TransformError> {
        self.transform_u8(input, output)
    }

    /// Transactional alias; same fixed scratch and two passes as `transform_u16`.
    pub fn transform_u16_atomic(
        &mut self,
        input: &[u16],
        output: &mut [u16],
    ) -> Result<(), TransformError> {
        self.transform_u16(input, output)
    }
}

pub(super) fn transform_u8(
    transform: &Transform,
    input: &[u8],
    output: &mut [u8],
) -> Result<(), TransformError> {
    transform_integer_chunked(
        transform,
        input,
        output,
        |value| value as f32 / 255.0,
        |value| (value.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(super) fn transform_u16(
    transform: &Transform,
    input: &[u16],
    output: &mut [u16],
) -> Result<(), TransformError> {
    transform_integer_chunked(
        transform,
        input,
        output,
        |value| value as f32 / 65535.0,
        |value| (value.clamp(0.0, 1.0) * 65535.0).round() as u16,
    )
}

fn transform_integer_chunked<T, ToFloat, FromFloat>(
    transform: &Transform,
    input: &[T],
    output: &mut [T],
    to_float: ToFloat,
    from_float: FromFloat,
) -> Result<(), TransformError>
where
    T: Copy,
    ToFloat: Fn(T) -> f32 + Copy,
    FromFloat: Fn(f32) -> T + Copy,
{
    transform.validate_buffer_lengths(input.len(), output.len())?;
    let input_channels = transform.input_channels();
    let output_channels = transform.output_channels();
    if input_channels > 4 || output_channels > MAX_CHANNELS {
        return Err(TransformError::UnsupportedProfileFeature(
            "worker supports at most four input and three output channels",
        ));
    }
    let pixels = input.len() / input_channels;
    let chunk_pixels = (CHUNK_PIXELS * MAX_CHANNELS / input_channels).min(CHUNK_PIXELS);
    let mut input_chunk = [0.0f32; CHUNK_PIXELS * MAX_CHANNELS];
    let mut output_chunk = [0.0f32; CHUNK_PIXELS * MAX_CHANNELS];

    // Validate every chunk before publishing any output. This retains the
    // old caller-output-on-error contract without an image-sized temporary.
    // The first pass validates every chunk. The second pass publishes the
    // already-validated results, preserving the caller output on any error.
    for publish in [false, true] {
        for pixel_start in (0..pixels).step_by(chunk_pixels) {
            let count = (pixels - pixel_start).min(chunk_pixels);
            let input_count = count * input_channels;
            let output_count = count * output_channels;
            let input_offset = pixel_start * input_channels;
            for (destination, source) in input_chunk[..input_count]
                .iter_mut()
                .zip(&input[input_offset..input_offset + input_count])
            {
                *destination = to_float(*source);
            }
            transform.transform_f32(
                &input_chunk[..input_count],
                &mut output_chunk[..output_count],
            )?;
            if publish {
                let output_offset = pixel_start * output_channels;
                for (destination, source) in output[output_offset..output_offset + output_count]
                    .iter_mut()
                    .zip(&output_chunk[..output_count])
                {
                    *destination = from_float(*source);
                }
            }
        }
    }
    Ok(())
}
