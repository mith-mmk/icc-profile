//! Bounded matrix, CMYK-input and CMYK-output relative black-point compensation.
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

/// Branch used by the bounded LUT destination estimator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmykOutputBpcMethod {
    MidrangeStraight,
    QuadraticFit,
}
/// Immutable CMYK destination diagnostics for the selected interpolation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CmykOutputBpcDiagnostics {
    initial_device_black: [f32; 4],
    initial_lstar: f64,
    destination_lstar: f64,
    source: BpcBlackEstimate,
    method: CmykOutputBpcMethod,
    shadow_points: usize,
    scale: f64,
    offset: [f64; 3],
    interpolation: super::algorithm::ClutInterpolation3D,
}
impl CmykOutputBpcDiagnostics {
    pub fn initial_device_black(self) -> [f32; 4] {
        self.initial_device_black
    }
    pub fn initial_lstar(self) -> f64 {
        self.initial_lstar
    }
    pub fn destination_lstar(self) -> f64 {
        self.destination_lstar
    }
    pub fn source(self) -> BpcBlackEstimate {
        self.source
    }
    pub fn method(self) -> CmykOutputBpcMethod {
        self.method
    }
    pub fn shadow_points(self) -> usize {
        self.shadow_points
    }
    pub fn scale(self) -> f64 {
        self.scale
    }
    pub fn offset(self) -> [f64; 3] {
        self.offset
    }
    pub fn interpolation(self) -> super::algorithm::ClutInterpolation3D {
        self.interpolation
    }
}
#[derive(Debug)]
pub(super) struct CmykOutputBpcStage {
    diagnostics: [CmykOutputBpcDiagnostics; 2],
}
pub(super) const OUTPUT_RAMP_BYTES: usize = 256 * std::mem::size_of::<f64>();
impl CmykOutputBpcStage {
    pub(super) fn new(
        source: &MatrixProfile,
        diagnostics: [CmykOutputBpcDiagnostics; 2],
    ) -> Result<Self, TransformError> {
        let source = estimate(source)?;
        let mut diagnostics = diagnostics;
        for d in &mut diagnostics {
            d.source = source;
            let l = d.destination_lstar;
            let y = if l > 8.0 {
                ((l + 16.0) / 116.0).powi(3)
            } else {
                l * 27.0 / 24389.0
            };
            d.scale = (1.0 - y) / (1.0 - source.effective_y);
            d.offset = [0.9642, 1.0, 0.8249].map(|w| (1.0 - d.scale) * w);
        }
        Ok(Self { diagnostics })
    }
    pub(super) fn diagnostics(
        &self,
        interpolation: super::algorithm::ClutInterpolation3D,
    ) -> CmykOutputBpcDiagnostics {
        self.diagnostics
            [usize::from(interpolation == super::algorithm::ClutInterpolation3D::Trilinear)]
    }
    pub(super) fn apply(
        &self,
        xyz: &mut [f32; 3],
        interpolation: super::algorithm::ClutInterpolation3D,
    ) -> Result<(), TransformError> {
        let d = self.diagnostics(interpolation);
        for (v, o) in xyz.iter_mut().zip(d.offset) {
            *v = (*v as f64 * d.scale + o) as f32;
            if !v.is_finite() {
                return Err(TransformError::InvalidProfile("non-finite output BPC XYZ"));
            }
        }
        Ok(())
    }
}
// ICC-approved 2013 committee draft 4.2.5. Invalid/degenerate ramps are
// explicitly unsupported in this bounded subset, rather than zero-black success.
pub(super) fn output_estimate(
    initial: ([f32; 4], [f64; 3]),
    mut ramp: [f64; 256],
    interpolation: super::algorithm::ClutInterpolation3D,
) -> Result<CmykOutputBpcDiagnostics, TransformError> {
    let invalid =
        || TransformError::UnsupportedProfileFeature("invalid or degenerate CMYK output BPC ramp");
    if ramp
        .iter()
        .any(|v| !v.is_finite() || *v < 0.0 || *v > 100.01)
        || !initial.1[1].is_finite()
        || initial.1[1] < 0.0
    {
        return Err(invalid());
    }
    for i in (0..255).rev() {
        ramp[i] = ramp[i].min(ramp[i + 1]);
    }
    let min = ramp[0];
    let range = ramp[255] - min;
    if range <= 0.0 {
        return Err(invalid());
    }
    let y = initial.1[1];
    let initial_lstar = (if y > 216.0 / 24389.0 {
        116.0 * y.cbrt() - 16.0
    } else {
        24389.0 / 27.0 * y
    })
    .min(50.0);
    let straight = ramp.iter().enumerate().all(|(i, y)| {
        let x = i as f64 * 100.0 / 255.0;
        x <= min + 0.2 * range || (x - y).abs() < 4.0
    });
    let mut points = 0;
    let (l, method) = if straight {
        (initial_lstar, CmykOutputBpcMethod::MidrangeStraight)
    } else {
        // Scaled x improves conditioning; solve the independent 3x3 normal equations
        // with partial pivoting. Convert coefficients back to physical L* units.
        let mut a = [[0.0_f64; 4]; 3];
        for (i, v) in ramp.iter().enumerate() {
            let y = (v - min) / range;
            if !(0.1..0.5).contains(&y) {
                continue;
            }
            points += 1;
            let x = i as f64 / 255.0;
            let row = [x * x, x, 1.0];
            for r in 0..3 {
                for c in 0..3 {
                    a[r][c] += row[r] * row[c];
                }
                a[r][3] += row[r] * y;
            }
        }
        if points < 3 {
            return Err(invalid());
        }
        for c in 0..3 {
            let pivot = (c..3)
                .max_by(|i, j| a[*i][c].abs().total_cmp(&a[*j][c].abs()))
                .unwrap();
            a.swap(c, pivot);
            if a[c][c].abs() < 1e-14 {
                return Err(invalid());
            }
            let divisor = a[c][c];
            for value in &mut a[c][c..] {
                *value /= divisor;
            }
            let pivot_row = a[c];
            for (r, row) in a.iter_mut().enumerate() {
                if r != c {
                    let factor = row[c];
                    for (value, pivot_value) in row[c..].iter_mut().zip(&pivot_row[c..]) {
                        *value -= factor * pivot_value;
                    }
                }
            }
        }
        let (a, b, c) = (a[0][3] / 10000.0, a[1][3] / 100.0, a[2][3]);
        let z = if a.abs() < 1e-10 {
            if b.abs() < 1e-14 {
                return Err(invalid());
            }
            -c / b
        } else {
            let d = b * b - 4.0 * a * c;
            if d <= 0.0 {
                0.0
            } else {
                (-b + d.sqrt()) / (2.0 * a)
            }
        };
        if !z.is_finite() {
            return Err(invalid());
        }
        (z.clamp(0.0, 50.0), CmykOutputBpcMethod::QuadraticFit)
    };
    Ok(CmykOutputBpcDiagnostics {
        initial_device_black: initial.0,
        initial_lstar,
        destination_lstar: l,
        source: BpcBlackEstimate {
            endpoint: [0.0; 3],
            endpoint_xyz: [0.0; 3],
            raw_lstar: 0.0,
            capped_lstar: 0.0,
            effective_y: 0.0,
        },
        method,
        shadow_points: points,
        scale: 1.0,
        offset: [0.0; 3],
        interpolation,
    })
}

#[cfg(test)]
mod output_tests {
    use super::super::algorithm::ClutInterpolation3D::Trilinear;
    use super::*;
    fn initial(l: f64) -> ([f32; 4], [f64; 3]) {
        ([0.0; 4], [0.0, ((l + 16.0) / 116.0).powi(3), 0.0])
    }
    #[test]
    fn independent_analytic_ramps_cover_straight_linear_quadratic_and_rejection() {
        let straight = std::array::from_fn(|i| (i as f64 * 100.0 / 255.0).max(12.0));
        let d = output_estimate(initial(12.0), straight, Trilinear).unwrap();
        assert_eq!(d.method, CmykOutputBpcMethod::MidrangeStraight);
        assert!((d.destination_lstar - 12.0).abs() < 1e-10);
        // A clipped quadratic ramp with an analytic positive-gradient root of 15.
        let quadratic = std::array::from_fn(|i| {
            let x = i as f64 * 100.0 / 255.0;
            10.0 + 45.0 * (((x - 15.0).max(0.0) / 85.0).powi(2) + (x - 15.0).max(0.0) / 85.0)
        });
        let d = output_estimate(initial(10.0), quadratic, Trilinear).unwrap();
        assert_eq!(d.method, CmykOutputBpcMethod::QuadraticFit);
        assert!((d.destination_lstar - 15.0).abs() < 1e-4);
        let concave = std::array::from_fn(|i| {
            let t = ((i as f64 * 100.0 / 255.0 - 15.0) / 85.0).max(0.0);
            10.0 + 90.0 * (t - 0.3 * t * t) / 0.7
        });
        let d = output_estimate(initial(10.0), concave, Trilinear).unwrap();
        assert_eq!(d.method, CmykOutputBpcMethod::QuadraticFit);
        assert!((d.destination_lstar - 15.0).abs() < 1e-6);
        let mut repaired = straight;
        repaired[10] = 80.0;
        assert_eq!(
            output_estimate(initial(12.0), repaired, Trilinear)
                .unwrap()
                .destination_lstar,
            output_estimate(initial(12.0), straight, Trilinear)
                .unwrap()
                .destination_lstar
        );
        // Repair must not raise the last endpoint: a descending ramp becomes
        // constant at its final value and is rejected, not made increasing.
        let descending = std::array::from_fn(|i| 100.0 - i as f64 * 90.0 / 255.0);
        assert!(output_estimate(initial(10.0), descending, Trilinear).is_err());
        assert_eq!(
            output_estimate(initial(60.0), straight, Trilinear)
                .unwrap()
                .destination_lstar,
            50.0
        );
        let linear = std::array::from_fn(|i| 20.0 + 0.8 * i as f64 * 100.0 / 255.0);
        let d = output_estimate(initial(20.0), linear, Trilinear).unwrap();
        assert!(d.destination_lstar < 1e-10);
        assert!(output_estimate(initial(10.0), [10.0; 256], Trilinear).is_err());
        assert!(output_estimate(initial(10.0), [f64::NAN; 256], Trilinear).is_err());
        let sparse = std::array::from_fn(|i| if i < 254 { 0.0 } else { 100.0 });
        assert!(output_estimate(initial(10.0), sparse, Trilinear).is_err());
    }
}
