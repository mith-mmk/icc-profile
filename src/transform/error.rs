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
