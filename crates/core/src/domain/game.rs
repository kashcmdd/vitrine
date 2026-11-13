use serde::{Deserialize, Serialize};

use super::ids::{RunnerId, SystemId};

/// Where a game came from. Drives deduplication and, for Steam, gives us free
/// artwork from the CDN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchSource {
    Manual,
    Steam,
    Epic,
    Gog,
    Lutris,
    Heroic,
    LocalScan,
}

impl LaunchSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Steam => "steam",
            Self::Epic => "epic",
            Self::Gog => "gog",
            Self::Lutris => "lutris",
            Self::Heroic => "heroic",
            Self::LocalScan => "local_scan",
        }
    }

    /// Only Steam exposes a stable app id we can build CDN artwork URLs from.
    pub const fn has_steam_cdn_art(self) -> bool {
        matches!(self, Self::Steam)
    }
}

/// The platform a game's *binary* targets, which is independent of the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetPlatform {
    /// Runs on the host directly (a Linux ELF, or a Windows .exe on Windows).
    Native,
    /// A Windows binary that needs a compatibility layer on Linux.
    Windows,
}

impl TargetPlatform {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Windows => "windows",
        }
    }

    /// A Windows binary on Linux must go through Wine, Proton or UMU.
    pub const fn needs_compatibility_layer(self) -> bool {
        matches!(self, Self::Windows)
    }
}

/// A library entry.
///
/// `system_id` being `Some` makes this an emulated title: its launch target is a
/// ROM rather than an executable. This is the only structural difference between
/// the two kinds of game (ADR-0004) — everything downstream treats them the same.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub id: super::ids::GameId,
    pub title: String,
    pub sort_title: String,
    pub description: Option<String>,

    pub system_id: Option<SystemId>,

    pub executable_path: Option<std::path::PathBuf>,
    pub working_directory: Option<std::path::PathBuf>,
    /// Tokenised at the UI boundary; never a shell string.
    pub arguments: Vec<String>,
    pub env: std::collections::BTreeMap<String, String>,
    pub target_platform: TargetPlatform,

    pub source: LaunchSource,
    pub source_app_id: Option<String>,
    pub install_dir: Option<std::path::PathBuf>,
    pub runner_id: Option<RunnerId>,

    pub playtime_seconds: u64,
    pub last_played_at: Option<chrono::DateTime<chrono::Utc>>,
    pub launch_count: u32,

    pub is_favorite: bool,
    pub is_hidden: bool,

    pub spine_accent: Option<String>,

    /// The primary ROM for an emulated title. Resolved from `rom_files` by the
    /// caller, because a multi-disc set has no single path until the playlist is
    /// considered.
    pub rom: Option<std::path::PathBuf>,
}

impl Game {
    pub const fn is_emulated(&self) -> bool {
        self.system_id.is_some()
    }

    /// Stable identity for deduplication: the same Steam app or the same manual
    /// executable should not appear twice.
    pub fn dedupe_key(&self) -> String {
        match (&self.source_app_id, &self.executable_path) {
            (Some(app_id), _) => format!("{}:{}", self.source.as_str(), app_id),
            (None, Some(path)) => {
                format!("{}:{}", self.source.as_str(), path.to_string_lossy().to_lowercase())
            }
            (None, None) => format!("{}:{}", self.source.as_str(), self.id),
        }
    }
}
