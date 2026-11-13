//! Pure domain types.
//!
//! Nothing here performs I/O, reads the clock, asks the OS anything, or carries
//! a `#[cfg]` attribute. That constraint is what makes the launch resolver and
//! the setup registry unit-testable without a database or a GUI.

pub mod game;
pub mod ids;
pub mod launch;
pub mod runner;
pub mod session;
pub mod system;

pub use game::{Game, LaunchSource, TargetPlatform};
pub use ids::{GameId, LaunchConfigId, PlayerId, RunnerId, SessionId, SystemId};
pub use launch::{LaunchConfig, LaunchPlan, ProtonOptions, Target, Wrapper, WrapperChain};
pub use runner::{Runner, RunnerKind};
pub use session::{PlaySession, SessionStatus};
pub use system::{System, MediaKind};
