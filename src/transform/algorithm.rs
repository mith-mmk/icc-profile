//! Explicit numerical choices, separate from colorimetric intent and buffers.

/// Supported interpolation algorithms for three-dimensional CLUTs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClutInterpolation3D {
    /// Historical default: four vertices from a tetrahedral subdivision.
    #[default]
    Tetrahedral,
    /// Eight-corner multilinear interpolation.
    Trilinear,
}

/// Supported interpolation algorithms for four-dimensional CLUTs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClutInterpolation4D {
    /// Sixteen-corner multilinear interpolation (the only current option).
    #[default]
    Multilinear,
}

/// A copyable bundle of implemented numerical algorithm choices.
///
/// Default preserves historical 3D tetrahedral and CMYK 4D multilinear behavior.
/// This is not a named CMS compatibility mode or a promise of matching another
/// engine. Choices apply only to CLUTs of the corresponding dimension; matrix
/// and 1D curve paths are unaffected. Intent, BPC and clipping remain in
/// `TransformOptions`. Private fields allow future choices/presets without
/// breaking existing TransformOptions struct literals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlgorithmProfile {
    clut_3d: ClutInterpolation3D,
    clut_4d: ClutInterpolation4D,
}

impl AlgorithmProfile {
    pub fn with_clut_3d(mut self, choice: ClutInterpolation3D) -> Self {
        self.clut_3d = choice;
        self
    }
    pub fn with_clut_4d(mut self, choice: ClutInterpolation4D) -> Self {
        self.clut_4d = choice;
        self
    }
    pub fn clut_3d(self) -> ClutInterpolation3D {
        self.clut_3d
    }
    pub fn clut_4d(self) -> ClutInterpolation4D {
        self.clut_4d
    }
}
