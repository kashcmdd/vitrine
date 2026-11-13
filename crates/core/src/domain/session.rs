use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ids::{GameId, LaunchConfigId, SessionId};

/// How a session ended. The distinction matters: only `killed` and `orphaned`
/// imply the user or the OS intervened, which is what the UI apologises for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Live. At most one per game, enforced by a partial unique index.
    Running,
    /// Terminated normally. A non-zero exit code is still `Exited`: games exit
    /// non-zero all the time and that is not a crash.
    Exited,
    /// Died quickly and abnormally.
    Crashed,
    /// Terminated by the user from the launcher.
    Killed,
    /// The launcher died while the game was running; closed at the last
    /// heartbeat on next boot.
    Orphaned,
}

impl SessionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Exited => "exited",
            Self::Crashed => "crashed",
            Self::Killed => "killed",
            Self::Orphaned => "orphaned",
        }
    }

    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Running)
    }

    /// Every terminal status contributes to playtime. A crash after three hours
    /// is still three hours played.
    pub const fn counts_toward_playtime(self) -> bool {
        self.is_terminal()
    }

    /// Whether the UI should offer an explanation rather than a silent update.
    pub const fn needs_explanation(self) -> bool {
        matches!(self, Self::Crashed | Self::Killed | Self::Orphaned)
    }
}

/// A contiguous period during which a game was running.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaySession {
    pub id: SessionId,
    pub game_id: GameId,
    pub launch_config_id: Option<LaunchConfigId>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Written every 20 s while running. If the launcher is killed outright,
    /// this is how much of the session survives.
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub exit_code: Option<i32>,
    pub exit_signal: Option<i32>,
    pub status: SessionStatus,
    pub launcher_pid: Option<u32>,
    pub root_pid: Option<u32>,
    pub log_path: Option<PathBuf>,
    pub error_message: Option<String>,
}

impl PlaySession {
    /// Whole seconds between start and end, or up to the last heartbeat for a
    /// session being recovered.
    pub fn duration_seconds(&self) -> Option<i64> {
        let end = self.ended_at.as_ref().unwrap_or(&self.last_heartbeat);
        let delta = end.signed_duration_since(self.started_at).num_seconds();
        Some(delta.max(0))
    }

    /// Classifies a finished run. Kept here rather than in the tracker so both
    /// OS implementations share one definition of "crashed".
    ///
    /// A non-zero exit code within the first ten seconds is a crash: something
    /// refused to start. A non-zero code after that is a normal exit.
    pub fn classify(
        exit_code: Option<i32>,
        elapsed: std::time::Duration,
        killed_by_user: bool,
    ) -> SessionStatus {
        if killed_by_user {
            return SessionStatus::Killed;
        }
        match exit_code {
            Some(0) => SessionStatus::Exited,
            Some(_) if elapsed < std::time::Duration::from_secs(10) => SessionStatus::Crashed,
            Some(_) => SessionStatus::Exited,
            None => SessionStatus::Crashed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn session(start_minute: u32, end_minute: Option<u32>, heartbeat_minute: u32) -> PlaySession {
        let base = chrono::Utc.with_ymd_and_hms(2026, 11, 13, 1, 0, 0).unwrap();
        PlaySession {
            id: SessionId::from_db("s1"),
            game_id: GameId::from_db("g1"),
            launch_config_id: None,
            started_at: base + chrono::Duration::minutes(start_minute as i64),
            ended_at: end_minute.map(|m| base + chrono::Duration::minutes(m as i64)),
            last_heartbeat: base + chrono::Duration::minutes(heartbeat_minute as i64),
            exit_code: Some(0),
            exit_signal: None,
            status: SessionStatus::Exited,
            launcher_pid: None,
            root_pid: None,
            log_path: None,
            error_message: None,
        }
    }

    #[test]
    fn duration_uses_the_end_when_present() {
        assert_eq!(session(0, Some(90), 90).duration_seconds(), Some(5400));
    }

    #[test]
    fn duration_falls_back_to_the_last_heartbeat_when_orphaned() {
        // Launcher died at 20 minutes; the session is recoverable up to there.
        assert_eq!(session(0, None, 20).duration_seconds(), Some(1200));
    }

    #[test]
    fn duration_is_never_negative() {
        // Clock skew or a bad row must not subtract playtime.
        assert_eq!(session(30, None, 10).duration_seconds(), Some(0));
    }

    #[test]
    fn a_fast_non_zero_exit_is_a_crash() {
        assert_eq!(
            PlaySession::classify(Some(3), std::time::Duration::from_secs(4), false),
            SessionStatus::Crashed
        );
    }

    #[test]
    fn a_late_non_zero_exit_is_a_normal_exit() {
        // Most games exit non-zero; that is not a crash.
        assert_eq!(
            PlaySession::classify(Some(1), std::time::Duration::from_secs(600), false),
            SessionStatus::Exited
        );
    }

    #[test]
    fn a_clean_exit_is_exited() {
        assert_eq!(
            PlaySession::classify(Some(0), std::time::Duration::from_secs(2), false),
            SessionStatus::Exited
        );
    }

    #[test]
    fn user_kill_wins_over_the_exit_code() {
        assert_eq!(
            PlaySession::classify(Some(137), std::time::Duration::from_secs(3600), true),
            SessionStatus::Killed
        );
    }

    #[test]
    fn every_terminal_status_counts_toward_playtime() {
        for status in [
            SessionStatus::Exited,
            SessionStatus::Crashed,
            SessionStatus::Killed,
            SessionStatus::Orphaned,
        ] {
            assert!(status.counts_toward_playtime());
            assert!(!status.is_terminal() == false);
        }
        assert!(!SessionStatus::Running.counts_toward_playtime());
    }
}
