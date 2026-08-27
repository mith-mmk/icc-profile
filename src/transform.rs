//! Checked ICC parsing and color transformation facade.

mod compile;
mod curve;
mod error;
mod limits;
mod profile;
mod reader;
mod worker;

#[cfg(test)]
mod tests;

pub use compile::Transform;
pub use error::TransformError;
pub use limits::ParseLimits;
pub use profile::TransformOptions;
pub use profile::{ColorSpace, Pcs, Profile, RenderingIntent};
pub use worker::TransformWorker;
