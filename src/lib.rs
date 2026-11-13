//! Profile-free color math and checked ICC transforms, with compatible legacy APIs.
//!
//! Choose [`simple`] for explicitly named standalone conversions with no profile,
//! or [`icc`] for checked ICC parsing, direction-specific compilation and execution.
//! ICC errors never select a simple conversion automatically.
//!
//! # ICC quickstart
//! ```
//! use icc_profile::icc::{Profile, RenderingIntent, Transform, TransformError, TransformOptions};
//! fn convert(src: &[u8], dst: &[u8], rgb: &[f32]) -> Result<Vec<f32>, TransformError> {
//!     let src = Profile::parse(src)?;
//!     let dst = Profile::parse(dst)?;
//!     let options = TransformOptions {
//!         rendering_intent: RenderingIntent::RelativeColorimetric,
//!         ..TransformOptions::default()
//!     };
//!     let transform = Transform::new(&src, &dst, options)?;
//!     transform.transform_f32_vec(rgb)
//! }
//! ```
//!
//! Device samples are interleaved channels in `0..=1` (`f32`), without alpha.
//! Standalone compiled stages exchange physical D50 XYZ, not normalized LUT codes.
//! Parsing validates the container; compiling validates the selected direction.
//! See [`icc`] for execution limits, transactions and routing guarantees.
//!
//! # Profile-free quickstart
//! ```
//! use icc_profile::simple::{rgb8_to_ycbcr8_full, YcbcrCoefficients};
//! let ycbcr = rgb8_to_ycbcr8_full([255, 0, 0], YcbcrCoefficients::Bt601);
//! assert_eq!(ycbcr, [76, 85, 255]);
//! ```
//!
//! These byte conversions use full-range digital YCbCr. For floating RGB/XYZ,
//! supply a direction-specific matrix and already linear RGB; for XYZ/Lab,
//! supply a reference white. No transfer curve or adaptation is implicit.
//!
//! # Compatibility
//! Existing root exports, [`transform`], [`cms`] (including `transration`),
//! [`iccprofile`] and [`utils`] remain available without deprecation attributes.
//! Their old units, layouts and failure behavior are not redefined by the new
//! namespaces. Prefer the explicit entrypoints for new work; migrate deliberately.

pub use crate::iccprofile::*;
pub mod cms;
pub mod icc;
pub mod simple;
mod color_diff;
pub mod iccprofile;
pub mod transform;
pub mod utils;

#[cfg(test)]
pub(crate) mod allocation_probe;

#[cfg(test)]
#[global_allocator]
static TEST_ALLOCATOR: allocation_probe::Probe = allocation_probe::Probe;

pub use transform::{
    AlgorithmProfile, BpcBlackEstimate, BpcDiagnostics, BpcMethod, ClutInterpolation3D,
    ClutInterpolation4D, CmykBpcDiagnostics, CmykOutputBpcDiagnostics, CmykOutputBpcMethod,
    ColorSpace, CompiledProfile, ExecutionLimits, ExecutionLimitsBuilder, ParseLimits, Pcs,
    Profile, RenderingIntent, RouteInfo, RouteModel, Transform, TransformDirection, TransformError,
    TransformLimits, TransformLimitsBuilder, TransformMemory, TransformOptions, TransformWorker,
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
