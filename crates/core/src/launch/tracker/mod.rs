//! Process supervision contract (ADR-0003).
//!
//! The rule this module exists to enforce: **the direct child process is not the
//! game.** Bootstrappers, `umu-run`, DRM shims and `wineserver` all exit while the
//! game keeps running, so "is it still running?" must be answered by a
//! containment primitive — a Job Object on Windows, a process group plus
//! descendant closure on Linux.
//!
//! Implementations arrive in M3:
//!   * `windows.rs` — Job Objects with `PROC_THREAD_ATTRIBUTE_JOB_LIST` and
//!     `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, polling `ActiveProcesses`.
//!   * `linux.rs`   — `setsid()` plus `PR_SET_CHILD_SUBREAPER`, membership as the
//!     union of process-group members and the PPID-descendant closure.
//!
//! Both are deliberately absent until they can be tested against a real process
//! tree on both operating systems.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::domain::{LaunchPlan, SessionStatus};

/// A live process belonging to a running game. Display only: used by the UI
/// process panel, never to decide whether the game has exited.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcInfo {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
}

/// Returned once a process exists and is contained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnedProcess {
    /// The process we spawned directly.
    pub launcher_pid: u32,
    /// The process the game actually became, when it differs (post-bootstrap).
    pub root_pid: Option<u32>,
}

/// How a session ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitReport {
    pub status: SessionStatus,
    pub exit_code: Option<i32>,
    /// Unix signal number, when the process was signalled.
    pub exit_signal: Option<i32>,
    pub duration: Duration,
}

/// Contains a launched game and reports when all of it has exited.
#[async_trait]
pub trait ProcessTracker: Send + Sync {
    /// Spawns and contains. Returns once the process exists and is contained,
    /// so a failure here is reported synchronously to the user.
    async fn spawn(&self, plan: &LaunchPlan) -> crate::Result<SpawnedProcess>;

    /// Resolves when the containment is empty, not when the root process exits.
    /// Cancelling `cancel` requests a graceful stop.
    async fn wait_for_exit(&mut self, cancel: CancellationToken) -> crate::Result<ExitReport>;

    /// Current members of the containment, for the UI.
    fn snapshot(&self) -> Vec<ProcInfo>;

    /// Graceful stop, then force after `grace`. Sweeps escapees that left the
    /// process group or job.
    async fn terminate(&mut self, grace: Duration) -> crate::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_report_carries_signal_and_code_separately() {
        // On Unix a signalled process has no exit code; on Windows there is no
        // signal. Conflating them makes "killed by the user" indistinguishable
        // from "exited with 137".
        let report = ExitReport {
            status: SessionStatus::Killed,
            exit_code: None,
            exit_signal: Some(9),
            duration: Duration::from_secs(42),
        };
        assert!(report.exit_code.is_none());
        assert_eq!(report.exit_signal, Some(9));
    }
}
