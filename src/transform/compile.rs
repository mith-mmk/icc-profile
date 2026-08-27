use super::curve::inverse_curve;
use super::error::TransformError;
use super::profile::{MatrixProfile, Profile, RenderingIntent, TransformOptions};
use super::reader::D50;

#[derive(Clone, Debug)]
pub struct Transform {
    pub(super) input: MatrixProfile,
    pub(super) output: MatrixProfile,
    pub(super) options: TransformOptions,
}

impl Transform {
    pub fn new(
        input: &Profile,
        output: &Profile,
        options: TransformOptions,
    ) -> Result<Self, TransformError> {
        if options.black_point_compensation {
            return Err(TransformError::UnsupportedProfileFeature(
                "black point compensation requires a profile black-point stage",
            ));
        }
        if options.rendering_intent != RenderingIntent::RelativeColorimetric {
            return Err(TransformError::UnsupportedProfileFeature(
                "matrix/TRC profiles do not provide intent-specific stages",
            ));
        }
        let input = input
            .0
            .matrix
            .clone()
            .ok_or(TransformError::UnsupportedProfileFeature(
                "input profile requires a LUT or CMYK adapter",
            ))?;
        let output = output
            .0
            .matrix
            .clone()
            .ok_or(TransformError::UnsupportedProfileFeature(
                "output profile requires a LUT or CMYK adapter",
            ))?;
        if input.pcs != output.pcs {
            return Err(TransformError::UnsupportedProfileFeature("mixed PCS"));
        }
        if input.curves.len() > 1 && input.inverse.is_none() {
            return Err(TransformError::InvalidProfile("input matrix is singular"));
        }
        if output.curves.len() > 1 && output.inverse.is_none() {
            return Err(TransformError::InvalidProfile("output matrix is singular"));
        }
        Ok(Self {
            input,
            output,
            options,
        })
    }

    pub fn worker(&self) -> super::worker::TransformWorker {
        super::worker::TransformWorker {
            transform: self.clone(),
            scratch: Vec::new(),
            output_scratch: Vec::new(),
        }
    }
    pub fn input_channels(&self) -> usize {
        self.input.curves.len()
    }
    pub fn output_channels(&self) -> usize {
        self.output.curves.len()
    }

    pub fn transform_f32(&self, input: &[f32], output: &mut [f32]) -> Result<(), TransformError> {
        let ic = self.input_channels();
        let oc = self.output_channels();
        if input.len() % ic != 0 {
            return Err(TransformError::InvalidBufferLength {
                expected: ic,
                actual: input.len(),
            });
        }
        let pixels = input.len() / ic;
        let expected = pixels
            .checked_mul(oc)
            .ok_or(TransformError::ResourceLimit("output length"))?;
        if output.len() != expected {
            return Err(TransformError::InvalidBufferLength {
                expected,
                actual: output.len(),
            });
        }
        for (src, dst) in input.chunks_exact(ic).zip(output.chunks_exact_mut(oc)) {
            if src.iter().any(|x| !x.is_finite()) {
                return Err(TransformError::NonFiniteInput);
            }
            let xyz = device_to_xyz(&self.input, src);
            xyz_to_device(&self.output, xyz, dst, self.options.clamp)?;
        }
        Ok(())
    }

    pub fn transform_f32_vec(&self, input: &[f32]) -> Result<Vec<f32>, TransformError> {
        if input.len() % self.input_channels() != 0 {
            return Err(TransformError::InvalidBufferLength {
                expected: self.input_channels(),
                actual: input.len(),
            });
        }
        let mut result = vec![0.0; input.len() / self.input_channels() * self.output_channels()];
        self.transform_f32(input, &mut result)?;
        Ok(result)
    }

    pub fn transform_u8(&self, input: &[u8], output: &mut [u8]) -> Result<(), TransformError> {
        let values: Vec<f32> = input.iter().map(|x| *x as f32 / 255.0).collect();
        let mut converted = vec![0.0; output.len()];
        self.transform_f32(&values, &mut converted)?;
        for (d, x) in output.iter_mut().zip(converted) {
            *d = (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        Ok(())
    }

    pub fn transform_u16(&self, input: &[u16], output: &mut [u16]) -> Result<(), TransformError> {
        let values: Vec<f32> = input.iter().map(|x| *x as f32 / 65535.0).collect();
        let mut converted = vec![0.0; output.len()];
        self.transform_f32(&values, &mut converted)?;
        for (d, x) in output.iter_mut().zip(converted) {
            *d = (x.clamp(0.0, 1.0) * 65535.0).round() as u16;
        }
        Ok(())
    }
}

fn device_to_xyz(profile: &MatrixProfile, input: &[f32]) -> [f32; 3] {
    let mut v = [0.0; 3];
    for i in 0..3.min(input.len()) {
        v[i] = profile.curves[i].eval(input[i]);
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
        output[0] = inverse_curve(&profile.curves[0], (xyz[1] / D50[1]).clamp(0.0, 1.0));
        if clamp {
            output[0] = output[0].clamp(0.0, 1.0);
        }
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
        let value = inverse_curve(&profile.curves[i], value.clamp(0.0, 1.0));
        output[i] = if clamp { value.clamp(0.0, 1.0) } else { value };
    }
    Ok(())
}
