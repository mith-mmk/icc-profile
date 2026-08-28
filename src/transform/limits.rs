/// Limits applied before any allocation derived from an ICC profile.
#[derive(Clone, Copy, Debug)]
pub struct ParseLimits {
    pub max_profile_size: usize,
    pub max_tag_count: usize,
    pub max_tag_size: usize,
    pub max_curve_entries: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_profile_size: 64 * 1024 * 1024,
            max_tag_count: 4096,
            max_tag_size: 64 * 1024 * 1024,
            max_curve_entries: 1 << 20,
        }
    }
}

/// Limits for decoded, immutable transform data.  These are separate from
/// [`ParseLimits`] because encoded tag bounds and compiled curve/CLUT storage
/// are different resources.
#[derive(Clone, Copy, Debug)]
pub struct TransformLimits {
    pub(crate) max_compiled_bytes: usize,
    pub(crate) max_curve_entries: usize,
    pub(crate) max_clut_entries: usize,
}

impl Default for TransformLimits {
    fn default() -> Self {
        Self {
            max_compiled_bytes: 64 * 1024 * 1024,
            max_curve_entries: 1 << 20,
            max_clut_entries: 1 << 24,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TransformLimitsBuilder {
    limits: TransformLimits,
}

impl TransformLimits {
    pub fn builder() -> TransformLimitsBuilder {
        TransformLimitsBuilder {
            limits: Self::default(),
        }
    }
}

impl TransformLimitsBuilder {
    pub fn max_compiled_bytes(mut self, value: usize) -> Self {
        self.limits.max_compiled_bytes = value;
        self
    }

    pub fn max_curve_entries(mut self, value: usize) -> Self {
        self.limits.max_curve_entries = value;
        self
    }

    pub fn max_clut_entries(mut self, value: usize) -> Self {
        self.limits.max_clut_entries = value;
        self
    }

    pub fn build(self) -> Result<TransformLimits, &'static str> {
        if self.limits.max_compiled_bytes == 0
            || self.limits.max_curve_entries < 2
            || self.limits.max_clut_entries < 1
        {
            return Err("transform limits must be non-zero");
        }
        Ok(self.limits)
    }
}

/// Bound for owned output allocation used by an executing compiled transform.
/// The default is 64 MiB. A zero bound is valid only for an empty output;
/// callers that need a different owned-output bound can use the explicit
/// `*_with_limits` API. This is deliberately separate from [`TransformLimits`],
/// which only governs immutable compiled profile data. Integer and borrowed
/// F32 execution uses a fixed stack workspace and this limit does not impose
/// an image-pixel bound on those APIs.
#[derive(Clone, Copy, Debug)]
pub struct ExecutionLimits {
    pub(crate) max_output_bytes: usize,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_output_bytes: 64 * 1024 * 1024,
        }
    }
}

impl ExecutionLimits {
    pub fn builder() -> ExecutionLimitsBuilder {
        ExecutionLimitsBuilder {
            limits: Self::default(),
        }
    }

    pub fn max_output_bytes(self) -> usize {
        self.max_output_bytes
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ExecutionLimitsBuilder {
    limits: ExecutionLimits,
}

impl ExecutionLimitsBuilder {
    pub fn max_output_bytes(mut self, value: usize) -> Self {
        self.limits.max_output_bytes = value;
        self
    }

    pub fn build(self) -> Result<ExecutionLimits, &'static str> {
        Ok(self.limits)
    }
}
