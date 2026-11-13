use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ids::RunnerId;

/// How a target is executed. Emulators are runners (ADR-0004), which is why
/// there is no separate emulation concept here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerKind {
    /// Run the target directly.
    Native,
    /// `wine <exe>`
    Wine,
    /// A Proton build, invoked directly. Advanced; prefer `Umu`.
    Proton,
    /// `umu-run <exe>` — the default for Windows games on Linux.
    Umu,
    /// A RetroArch core or a standalone emulator.
    Emulator,
    /// Anything else the user has wired up themselves.
    Custom,
}

impl RunnerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Wine => "wine",
            Self::Proton => "proton",
            Self::Umu => "umu",
            Self::Emulator => "emulator",
            Self::Custom => "custom",
        }
    }

    /// Proton and UMU both want `MANGOHUD=1` in the environment rather than being
    /// wrapped: `mangohud` in front of a Wine binary is less reliable than
    /// Proton's own preload path.
    pub const fn prefers_mangohud_env(self) -> bool {
        matches!(self, Self::Proton | Self::Umu)
    }

    pub const fn launches_roms(self) -> bool {
        matches!(self, Self::Emulator | Self::Custom)
    }

    pub const fn needs_prefix(self) -> bool {
        matches!(self, Self::Wine | Self::Proton | Self::Umu)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Runner {
    pub id: RunnerId,
    pub name: String,
    pub kind: RunnerKind,
    /// wine binary, Proton directory, `umu-run`, or `retroarch`.
    pub executable_path: Option<PathBuf>,
    pub prefix_path: Option<PathBuf>,
    pub umu_game_id: Option<String>,
    pub core_path: Option<PathBuf>,
    pub flatpak_id: Option<String>,
    pub default_env: BTreeMap<String, String>,
    pub is_auto_detected: bool,
    pub is_default: bool,
}

impl Runner {
    /// `GAMEID` for UMU, which is how UMU selects a Proton prefix layout.
    pub fn umu_game_id_or_default(&self) -> &str {
        self.umu_game_id.as_deref().unwrap_or("umu-default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proton_and_umu_prefer_the_env_var() {
        assert!(RunnerKind::Proton.prefers_mangohud_env());
        assert!(RunnerKind::Umu.prefers_mangohud_env());
        assert!(!RunnerKind::Wine.prefers_mangohud_env());
        assert!(!RunnerKind::Native.prefers_mangohud_env());
    }

    #[test]
    fn only_emulators_launch_roms() {
        assert!(RunnerKind::Emulator.launches_roms());
        assert!(!RunnerKind::Umu.launches_roms());
    }

    #[test]
    fn prefixes_belong_to_compatibility_layers() {
        assert!(RunnerKind::Wine.needs_prefix());
        assert!(RunnerKind::Umu.needs_prefix());
        assert!(!RunnerKind::Native.needs_prefix());
        assert!(!RunnerKind::Emulator.needs_prefix());
    }
}
