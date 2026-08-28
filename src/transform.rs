//! Checked ICC parsing and color transformation facade.

mod compile;
mod compile_budget;
mod compile_plan;
mod curve;
mod curve_plan;
mod direction;
mod error;
mod execution;
mod limits;
mod lut;
mod lut_plan;
mod profile;
mod reader;
mod route_plan;
mod worker;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "transform/compile_budget_tests.rs"]
mod compile_budget_tests;

#[cfg(test)]
#[path = "transform/execution_tests.rs"]
mod execution_tests;

pub use compile::{CompiledProfile, Transform, TransformDirection};
pub use error::TransformError;
pub use limits::{
    ExecutionLimits, ExecutionLimitsBuilder, ParseLimits, TransformLimits, TransformLimitsBuilder,
};
pub use profile::TransformOptions;
pub use profile::{ColorSpace, Pcs, Profile, RenderingIntent, RouteInfo, RouteModel};
pub use worker::TransformWorker;
