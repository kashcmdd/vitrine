//! Launch planning and process supervision.
//!
//! [`plan`] is pure and complete. [`tracker`] defines the supervision contract;
//! its two implementations land in M3, because they are the only code in the
//! project that cannot be validated without both operating systems.

pub mod plan;
pub mod tracker;

pub use plan::{resolve, RunnerBinding};
