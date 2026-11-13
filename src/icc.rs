//! Checked ICC engine: an additive namespace for the existing transform types.
//!
//! These are reexports, not wrappers or a second engine. Existing root imports,
//! signatures, type identity and [`TransformError`] variants are unchanged.
//!
//! Start with [`Profile::parse`] (checked container validation), then
//! [`Transform::new`] for a source/destination pair or [`CompiledProfile::compile`]
//! for one selected direction. Parsing does not promise that every direction or
//! optional feature can be compiled. `Profile::new` and `from_bytes` retain their
//! older eager semantic checks and are not aliases for the new parse workflow.
//!
//! # Samples and output
//! [`Transform`] uses tightly interleaved Gray/RGB/CMYK device channels, no alpha,
//! with normalized `f32` values or full-range `u8`/`u16`. CMYK increases from no
//! ink to full ink; JPEG inverted CMYK and YCCK are caller concerns.
//! [`CompiledProfile`] exchanges physical D50 XYZ on its PCS side even when a
//! profile stores Lab PCS. This XYZ is not confined to a unit cube.
//! Disabling [`TransformOptions::clamp`] does not enable a general HDR path:
//! route, inverse-curve and LUT domains still apply.
//!
//! Ordinary borrowed `f32` execution can partially write output on a late error.
//! Atomic pair/worker execution validates in a first pass and writes in a second,
//! using fixed 768-byte scratch. Integer execution is transactional already.
//! A single compiled pixel uses one evaluation and up to 16 bytes of atomic
//! scratch. These transactions cover returned errors, not panics or concurrency.
//! Vec helpers default to a 64 MiB output cap; [`ExecutionLimits`] allows an
//! explicit bound. Output lengths must match, and input must contain whole pixels.
//!
//! # Routes, algorithms and errors
//! [`RouteInfo::used_fallback`] describes legal *ICC tag/model selection*, never
//! fallback to [`crate::simple`]. Unsupported or malformed selected ICC routes
//! remain errors. [`AlgorithmProfile`] selects built-in numerical methods only;
//! it is not a caller-provided CMS engine or arbitrary callback mechanism.
//!
//! For compatibility, [`TransformError`] is unchanged. Do not assume that every
//! malformed payload is classified as `MalformedProfile`: existing checks also
//! use `InvalidProfile` and other applicable variants. Even with a valid profile,
//! an out-of-range device sample with `clamp=false` can yield `MalformedProfile`;
//! that variant alone does not prove profile corruption. Messages are diagnostics,
//! not a stable machine-readable classification. BPC support is route-specific,
//! not universal; consult the README route table before enabling it.

pub use crate::transform::{
    AlgorithmProfile, BpcBlackEstimate, BpcDiagnostics, BpcMethod, ClutInterpolation3D,
    ClutInterpolation4D, CmykBpcDiagnostics, CmykOutputBpcDiagnostics, CmykOutputBpcMethod,
    ColorSpace, CompiledProfile, ExecutionLimits, ExecutionLimitsBuilder, ParseLimits, Pcs,
    Profile, RenderingIntent, RouteInfo, RouteModel, Transform, TransformDirection, TransformError,
    TransformLimits, TransformLimitsBuilder, TransformMemory, TransformOptions, TransformWorker,
};
