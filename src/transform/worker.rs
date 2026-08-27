use super::compile::Transform;
use super::error::TransformError;

pub struct TransformWorker {
    pub(super) transform: Transform,
    pub(super) scratch: Vec<f32>,
    pub(super) output_scratch: Vec<f32>,
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
