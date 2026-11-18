use std::fmt;

#[derive(Debug)]
pub enum TransformError {
    InvalidProfile(&'static str),
    MalformedProfile(String),
    UnsupportedProfileFeature(&'static str),
    InvalidBufferLength { expected: usize, actual: usize },
    NonFiniteInput,
    ResourceLimit(&'static str),
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(s) => write!(f, "invalid ICC profile: {s}"),
            Self::MalformedProfile(s) => write!(f, "malformed ICC profile: {s}"),
            Self::UnsupportedProfileFeature(s) => write!(f, "unsupported ICC profile feature: {s}"),
            Self::InvalidBufferLength { expected, actual } => write!(
                f,
                "invalid buffer length: expected {expected}, got {actual}"
            ),
            Self::NonFiniteInput => f.write_str("transform input contains a non-finite value"),
            Self::ResourceLimit(s) => write!(f, "ICC resource limit exceeded: {s}"),
        }
    }
}

impl std::error::Error for TransformError {}

/// Detailed errors for the opt-in borrowed `f32` execution methods.
///
/// Existing methods still return [`TransformError`] unchanged. Sample
/// positions are zero-based and refer to caller-supplied input pixels.
#[derive(Debug)]
#[non_exhaustive]
pub enum TransformF32Error {
    /// The input has an invalid number of samples. Pair transforms require a
    /// multiple of `expected`; a standalone stage requires exactly that many.
    InputLength { expected: usize, actual: usize },
    /// The output has a different sample count than the transform requires.
    OutputLength { expected: usize, actual: usize },
    /// The input sample is NaN or infinite.
    NonFiniteInput { pixel: usize, channel: usize },
    /// A finite caller-supplied device sample is outside the required 0..=1.
    InputOutOfRange { pixel: usize, channel: usize },
    /// Another error, such as a limit or failure during route evaluation.
    /// This preserves the original error; do not infer an invalid ICC file
    /// from it alone (a supported route can reject derived values).
    Transform(TransformError),
}

impl TransformF32Error {
    pub(super) fn into_legacy(self) -> TransformError {
        match self {
            Self::InputLength { expected, actual } | Self::OutputLength { expected, actual } => {
                TransformError::InvalidBufferLength { expected, actual }
            }
            Self::NonFiniteInput { .. } => TransformError::NonFiniteInput,
            Self::InputOutOfRange { .. } => TransformError::MalformedProfile(
                "device sample is outside the normalized domain".into(),
            ),
            Self::Transform(error) => error,
        }
    }
}

impl From<TransformError> for TransformF32Error {
    fn from(error: TransformError) -> Self {
        Self::Transform(error)
    }
}

impl fmt::Display for TransformF32Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputLength { expected, actual } => {
                write!(
                    f,
                    "invalid input buffer length: {actual} samples for {expected} input channels"
                )
            }
            Self::OutputLength { expected, actual } => {
                write!(
                    f,
                    "invalid output buffer length: expected {expected}, got {actual}"
                )
            }
            Self::NonFiniteInput { pixel, channel } => {
                write!(f, "non-finite input at pixel {pixel}, channel {channel}")
            }
            Self::InputOutOfRange { pixel, channel } => {
                write!(
                    f,
                    "device input outside 0..=1 at pixel {pixel}, channel {channel}"
                )
            }
            Self::Transform(error) => fmt::Display::fmt(error, f),
        }
    }
}

impl std::error::Error for TransformF32Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transform(error) => Some(error),
            _ => None,
        }
    }
}
