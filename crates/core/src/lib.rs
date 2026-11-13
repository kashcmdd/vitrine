//! Vitrine core.
//!
//! Everything that is not the webview and not the Tauri shell lives here. The
//! crate deliberately has **no dependency on Tauri**, so the whole domain and
//! every service is testable on a machine with no GUI toolkit installed
//! (ADR-0001).
//!
//! Structure:
//!
//! * [`domain`] — pure types. No I/O, no `cfg`, no SQL.
//! * [`launch`] — plan resolution, wrappers, and (from M3) process trackers.
//! * [`setup`] — the first-run step registry (ADR-0008).
//! * `db` — pool setup and repositories, behind the `db` feature.
//!
//! The `db` feature exists so the pure layers can be typechecked and tested on a
//! machine without a C toolchain, which `libsqlite3-sys` requires. Platform
//! specific code appears only in [`launch::tracker`].

pub mod domain;
pub mod error;
pub mod launch;
pub mod setup;

#[cfg(feature = "db")]
pub mod db;

pub use error::{Error, PlanError, Result};

/// Application-level facts about the host that the pure launch resolver needs,
/// so it never has to ask the OS itself. Resolved once at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFacts {
    pub os: HostOs,
    /// Whether each wrapper is actually on PATH. A toggle for a missing binary is
    /// silently ignored rather than failing the launch.
    pub has_gamemoderun: bool,
    pub has_mangohud: bool,
    pub has_gamescope: bool,
    pub has_umu: bool,
}

impl HostFacts {
    /// The conservative default: no optional tools, nothing assumed.
    pub fn minimal(os: HostOs) -> Self {
        Self {
            os,
            has_gamemoderun: false,
            has_mangohud: false,
            has_gamescope: false,
            has_umu: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    Windows,
    Linux,
}

impl HostOs {
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }

    /// Wrappers are a Linux concept; on Windows a wrapper chain is always empty.
    pub const fn supports_wrappers(self) -> bool {
        matches!(self, Self::Linux)
    }

    /// A Windows binary cannot run natively on Linux; it needs a compatibility
    /// layer (Wine, Proton or UMU).
    pub const fn needs_compatibility_layer_for(self, platform: domain::TargetPlatform) -> bool {
        matches!(self, Self::Linux) && platform.needs_compatibility_layer()
    }
}
