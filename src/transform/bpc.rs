//! Bounded matrix and CMYK-input relative black-point compensation.
use super::error::TransformError;
use super::profile::MatrixProfile;

/// Black estimation method used by the bounded matrix BPC implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BpcMethod {
    /// Evaluate RGB 000 and 111, choose lower Y (000 on a tie), cap L* at 50,
    /// and neutralize using that luminance. No mediaBlackPointTag is consulted.
    MatrixEndpointsLstar50,
    /// Perceptual B2A0 black probe followed by relative A2B1, L* capped at 50.
    CmykPerceptualRoundTripLstar50,
}

/// Immutable diagnostics for one profile's estimated black.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BpcBlackEstimate {
    endpoint: [f32; 3],
    endpoint_xyz: [f64; 3],
    raw_lstar: f64,
    capped_lstar: f64,
    effective_y: f64,
}
impl BpcBlackEstimate {
    pub fn endpoint(self) -> [f32; 3] {
        self.endpoint
    }
    pub fn endpoint_xyz(self) -> [f64; 3] {
        self.endpoint_xyz
    }
    pub fn raw_lstar(self) -> f64 {
        self.raw_lstar
    }
    pub fn capped_lstar(self) -> f64 {
        self.capped_lstar
    }
    pub fn was_capped(self) -> bool {
        self.raw_lstar > self.capped_lstar
    }
    pub fn effective_y(self) -> f64 {
        self.effective_y
    }
}

/// Immutable diagnostics for the relative-PCS affine black-point stage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BpcDiagnostics {
    source: BpcBlackEstimate,
    destination: BpcBlackEstimate,
    scale: f64,
    offset: [f64; 3],
}
impl BpcDiagnostics {
    pub fn method(self) -> BpcMethod {
        BpcMethod::MatrixEndpointsLstar50
    }
    pub fn source(self) -> BpcBlackEstimate {
        self.source
    }
    pub fn destination(self) -> BpcBlackEstimate {
        self.destination
    }
    pub fn scale(self) -> f64 {
        self.scale
    }
    pub fn offset(self) -> [f64; 3] {
        self.offset
    }
}

#[derive(Debug)]
pub(super) struct BpcStage {
    diagnostics: BpcDiagnostics,
}
impl BpcStage {
    pub(super) fn new(
        source: &MatrixProfile,
        destination: &MatrixProfile,
    ) -> Result<Self, TransformError> {
        let source = estimate(source)?;
        let destination = estimate(destination)?;
        let scale = if source.effective_y == destination.effective_y {
            1.0
        } else {
            (1.0 - destination.effective_y) / (1.0 - source.effective_y)
        };
        let offset = [0.9642, 1.0, 0.8249].map(|white| (1.0 - scale) * white);
        if !scale.is_finite() || scale <= 0.0 || offset.iter().any(|v| !v.is_finite()) {
            return Err(TransformError::InvalidProfile(
                "invalid BPC affine coefficients",
            ));
        }
        Ok(Self {
            diagnostics: BpcDiagnostics {
                source,
                destination,
                scale,
                offset,
            },
        })
    }
    pub(super) fn diagnostics(&self) -> BpcDiagnostics {
        self.diagnostics
    }
    pub(super) fn apply(&self, xyz: &mut [f32; 3]) -> Result<(), TransformError> {
        // Preserve bit patterns for the exact identity case.
        if self.diagnostics.scale == 1.0 {
            return Ok(());
        }
        for (value, offset) in xyz.iter_mut().zip(self.diagnostics.offset) {
            let corrected = *value as f64 * self.diagnostics.scale + offset;
            if !corrected.is_finite() || !(corrected as f32).is_finite() {
                return Err(TransformError::InvalidProfile(
                    "BPC produced non-finite XYZ",
                ));
            }
            *value = corrected as f32;
        }
        Ok(())
    }
}

fn estimate(profile: &MatrixProfile) -> Result<BpcBlackEstimate, TransformError> {
    if profile.curves.len() != 3 {
        return Err(TransformError::InvalidProfile(
            "BPC requires three matrix curves",
        ));
    }
    let mut endpoints = [[0.0_f64; 3]; 2];
    for (endpoint, xyz) in endpoints.iter_mut().enumerate() {
        let mut linear = [0.0_f64; 3];
        for (value, curve) in linear.iter_mut().zip(&profile.curves) {
            let evaluated = curve.eval_unclamped(endpoint as f32);
            if !evaluated.is_finite() || !(0.0..=1.0).contains(&evaluated) {
                return Err(TransformError::InvalidProfile(
                    "BPC curve endpoint outside normalized domain",
                ));
            }
            *value = evaluated as f64;
        }
        for (value, row) in xyz.iter_mut().zip(profile.matrix) {
            *value = row.into_iter().zip(linear).map(|(m, v)| m as f64 * v).sum();
        }
        if xyz.iter().any(|v| !v.is_finite()) || xyz[1] < 0.0 {
            return Err(TransformError::InvalidProfile(
                "invalid BPC matrix endpoint XYZ",
            ));
        }
    }
    let endpoint = usize::from(endpoints[1][1] < endpoints[0][1]);
    let xyz = endpoints[endpoint];
    let y = xyz[1];
    let raw_lstar = if y > 216.0 / 24389.0 {
        116.0 * y.cbrt() - 16.0
    } else {
        24389.0 / 27.0 * y
    };
    let capped_lstar = raw_lstar.min(50.0);
    let effective_y = if raw_lstar <= 50.0 {
        y
    } else {
        ((capped_lstar + 16.0) / 116.0).powi(3)
    };
    Ok(BpcBlackEstimate {
        endpoint: [endpoint as f32; 3],
        endpoint_xyz: xyz,
        raw_lstar,
        capped_lstar,
        effective_y,
    })
}

/// Diagnostics for the bounded CMYK-input black-point estimate. The device
/// black contains all four normalized C/M/Y/K components; no channel is hidden.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CmykBpcDiagnostics {
    device_black: [f32; 4],
    source_xyz: [f64; 3],
    raw_lstar: f64,
    capped_lstar: f64,
    effective_y: f64,
    destination: BpcBlackEstimate,
    scale: f64,
    offset: [f64; 3],
    auxiliary_route: super::profile::RouteInfo,
    interpolation: super::algorithm::ClutInterpolation3D,
}
impl CmykBpcDiagnostics {
    pub fn device_black(self) -> [f32; 4] {
        self.device_black
    }
    pub fn source_xyz(self) -> [f64; 3] {
        self.source_xyz
    }
    pub fn raw_lstar(self) -> f64 {
        self.raw_lstar
    }
    pub fn capped_lstar(self) -> f64 {
        self.capped_lstar
    }
    pub fn was_capped(self) -> bool {
        self.raw_lstar > self.capped_lstar
    }
    pub fn effective_y(self) -> f64 {
        self.effective_y
    }
    pub fn destination(self) -> BpcBlackEstimate {
        self.destination
    }
    pub fn scale(self) -> f64 {
        self.scale
    }
    pub fn offset(self) -> [f64; 3] {
        self.offset
    }
    pub fn auxiliary_route(self) -> super::profile::RouteInfo {
        self.auxiliary_route
    }
    pub fn interpolation(self) -> super::algorithm::ClutInterpolation3D {
        self.interpolation
    }
    pub fn method(self) -> BpcMethod {
        BpcMethod::CmykPerceptualRoundTripLstar50
    }
}

#[derive(Debug)]
pub(super) struct CmykBpcStage {
    // Both supported reverse-LUT algorithms are compiled up front. Switching
    // AlgorithmProfile therefore cannot leave a stale black estimate behind.
    diagnostics: [CmykBpcDiagnostics; 2],
}
impl CmykBpcStage {
    pub(super) fn new(
        probes: [([f32; 4], [f64; 3]); 2],
        destination: &MatrixProfile,
        auxiliary_route: super::profile::RouteInfo,
    ) -> Result<Self, TransformError> {
        let destination = estimate(destination)?;
        let make = |(device_black, source_xyz): ([f32; 4], [f64; 3]), interpolation| {
            if device_black
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                || source_xyz.iter().any(|v| !v.is_finite())
                || source_xyz[1] < 0.0
            {
                return Err(TransformError::InvalidProfile(
                    "invalid CMYK BPC black probe",
                ));
            }
            let y = source_xyz[1];
            let raw_lstar = if y > 216.0 / 24389.0 {
                116.0 * y.cbrt() - 16.0
            } else {
                24389.0 / 27.0 * y
            };
            let capped_lstar = raw_lstar.min(50.0);
            let effective_y = if raw_lstar <= 50.0 {
                y
            } else {
                ((capped_lstar + 16.0) / 116.0).powi(3)
            };
            let scale = if effective_y == destination.effective_y {
                1.0
            } else {
                (1.0 - destination.effective_y) / (1.0 - effective_y)
            };
            let offset = [0.9642, 1.0, 0.8249].map(|w| (1.0 - scale) * w);
            if !scale.is_finite() || scale <= 0.0 || offset.iter().any(|v| !v.is_finite()) {
                return Err(TransformError::InvalidProfile(
                    "invalid CMYK BPC coefficients",
                ));
            }
            Ok(CmykBpcDiagnostics {
                device_black,
                source_xyz,
                raw_lstar,
                capped_lstar,
                effective_y,
                destination,
                scale,
                offset,
                auxiliary_route,
                interpolation,
            })
        };
        use super::algorithm::ClutInterpolation3D::{Tetrahedral, Trilinear};
        Ok(Self {
            diagnostics: [make(probes[0], Tetrahedral)?, make(probes[1], Trilinear)?],
        })
    }
    pub(super) fn diagnostics(
        &self,
        interpolation: super::algorithm::ClutInterpolation3D,
    ) -> CmykBpcDiagnostics {
        use super::algorithm::ClutInterpolation3D::{Tetrahedral, Trilinear};
        self.diagnostics[match interpolation {
            Tetrahedral => 0,
            Trilinear => 1,
        }]
    }
    pub(super) fn apply(
        &self,
        xyz: &mut [f32; 3],
        interpolation: super::algorithm::ClutInterpolation3D,
    ) -> Result<(), TransformError> {
        let diagnostic = self.diagnostics(interpolation);
        if diagnostic.scale == 1.0 {
            return Ok(());
        }
        for (value, offset) in xyz.iter_mut().zip(diagnostic.offset) {
            let corrected = *value as f64 * diagnostic.scale + offset;
            if !corrected.is_finite() || !(corrected as f32).is_finite() {
                return Err(TransformError::InvalidProfile(
                    "CMYK BPC produced non-finite XYZ",
                ));
            }
            *value = corrected as f32;
        }
        Ok(())
    }
}
