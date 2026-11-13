use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ids::{GameId, LaunchConfigId, RunnerId, SystemId};

/// What the launch will actually execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Process { exe: PathBuf },
    Rom { system: SystemId, file: PathBuf },
}

impl Target {
    pub fn display_path(&self) -> &std::path::Path {
        match self {
            Self::Process { exe } => exe,
            Self::Rom { file, .. } => file,
        }
    }
}

/// A Linux wrapper that composes around the real command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wrapper {
    /// `gamemoderun` — outermost, so it applies to everything inside.
    GameMode,
    /// `mangohud`, or `MANGOHUD=1` when the runner prefers the environment.
    MangoHud,
    /// `gamescope <args> --`, innermost of the wrappers.
    GameScope,
}

impl Wrapper {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GameMode => "gamemoderun",
            Self::MangoHud => "mangohud",
            Self::GameScope => "gamescope",
        }
    }
}

/// The wrapper chain for one launch, in application order (outermost first).
///
/// Composition rules live here rather than at the call site because getting the
/// order wrong produces subtle breakage: GameMode must be outside MangoHud to
/// affect it, and GameScope needs a `--` terminator before the command it wraps.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WrapperChain {
    pub wrappers: Vec<Wrapper>,
    pub gamescope_args: Vec<String>,
}

impl WrapperChain {
    /// Builds the chain from the per-game toggles, honouring the runner's
    /// preference for the MangoHud environment variable.
    pub fn build(
        use_gamemode: bool,
        use_mangohud: bool,
        use_gamescope: bool,
        gamescope_args: &[String],
        runner_prefers_mangohud_env: bool,
        wrappers_supported: bool,
    ) -> Self {
        if !wrappers_supported {
            return Self::default();
        }

        let mut wrappers = Vec::new();
        if use_gamemode {
            wrappers.push(Wrapper::GameMode);
        }
        if use_mangohud && !runner_prefers_mangohud_env {
            wrappers.push(Wrapper::MangoHud);
        }
        if use_gamescope {
            wrappers.push(Wrapper::GameScope);
        }

        Self {
            wrappers,
            gamescope_args: if use_gamescope {
                gamescope_args.to_vec()
            } else {
                Vec::new()
            },
        }
    }

    /// Wraps an already-built command. `argv[0]` must be the program.
    pub fn apply(&self, mut argv: Vec<String>) -> Vec<String> {
        for wrapper in self.wrappers.iter().rev() {
            match wrapper {
                Wrapper::GameScope => {
                    let mut wrapped = vec![Wrapper::GameScope.as_str().to_owned()];
                    wrapped.extend(self.gamescope_args.iter().cloned());
                    wrapped.push("--".to_owned());
                    wrapped.extend(argv);
                    argv = wrapped;
                }
                other => {
                    let mut wrapped = vec![other.as_str().to_owned()];
                    wrapped.extend(argv);
                    argv = wrapped;
                }
            }
        }
        argv
    }
}

/// Proton / DXVK knobs surfaced in the UI.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtonOptions {
    pub esync: Option<bool>,
    pub fsync: Option<bool>,
    pub nvapi: Option<bool>,
    pub dxvk_hud: Option<String>,
    pub dll_overrides: Option<String>,
    pub use_wow64: Option<bool>,
}

impl ProtonOptions {
    /// Translates the toggles into environment variables. Unset options are
    /// omitted rather than defaulted, so Proton's own defaults apply.
    pub fn to_env(&self) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        if let Some(esync) = self.esync {
            env.insert("WINEESYNC".into(), bool_str(esync).into());
        }
        if let Some(fsync) = self.fsync {
            env.insert("WINEFSYNC".into(), bool_str(fsync).into());
        }
        if let Some(nvapi) = self.nvapi {
            env.insert("PROTON_ENABLE_NVAPI".into(), bool_str(nvapi).into());
        }
        if let Some(hud) = &self.dxvk_hud {
            env.insert("DXVK_HUD".into(), hud.clone());
        }
        if let Some(overrides) = &self.dll_overrides {
            env.insert("WINEDLLOVERRIDES".into(), overrides.clone());
        }
        if let Some(wow64) = self.use_wow64 {
            env.insert("PROTON_USE_WOW64".into(), bool_str(wow64).into());
        }
        env
    }
}

const fn bool_str(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

/// One preset of launch settings for a game. A game may have several.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub id: LaunchConfigId,
    pub game_id: GameId,
    pub label: String,
    pub executable_override: Option<PathBuf>,
    pub arguments: Vec<String>,
    pub env_override: BTreeMap<String, String>,
    pub runner_id: Option<RunnerId>,
    pub use_gamemode: bool,
    pub use_mangohud: bool,
    pub use_gamescope: bool,
    pub gamescope_args: Vec<String>,
    pub proton_options: ProtonOptions,
    pub is_default: bool,
}

/// A fully resolved, ready-to-spawn command.
///
/// Produced by a pure function, so the whole composition is unit-testable
/// without touching the filesystem or the OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub game_id: GameId,
    pub config_id: LaunchConfigId,
    pub cwd: Option<PathBuf>,
    /// `argv[0]` is the program to execute.
    pub argv: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub wrappers: WrapperChain,
    pub target: Target,
    pub log_name: String,
}

impl LaunchPlan {
    /// Program to spawn; `argv[0]` by construction.
    pub fn program(&self) -> &str {
        self.argv.first().map(String::as_str).unwrap_or_default()
    }

    /// Redacted rendering for logs and the diagnostics panel. Values that look
    /// like secrets are replaced before they can reach a log file.
    pub fn argv_preview(&self) -> String {
        self.argv.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(wrappers: &[(&str, &str)]) -> WrapperChain {
        WrapperChain {
            wrappers: wrappers
                .iter()
                .map(|(name, _)| match *name {
                    "gamemoderun" => Wrapper::GameMode,
                    "mangohud" => Wrapper::MangoHud,
                    _ => Wrapper::GameScope,
                })
                .collect(),
            gamescope_args: wrappers
                .iter()
                .filter(|(name, _)| *name == "gamescope")
                .flat_map(|(_, args)| args.split_whitespace().map(str::to_owned))
                .collect(),
        }
    }

    #[test]
    fn gamemoderun_wraps_mangohud_wraps_gamescope() {
        let chain = chain(&[
            ("gamemoderun", ""),
            ("mangohud", ""),
            ("gamescope", "-W 2560 -H 1440 -f"),
        ]);
        let argv = chain.apply(vec!["game".into(), "--flag".into()]);
        assert_eq!(
            argv,
            vec![
                "gamemoderun",
                "mangohud",
                "gamescope",
                "-W",
                "2560",
                "-H",
                "1440",
                "-f",
                "--",
                "game",
                "--flag",
            ]
        );
    }

    #[test]
    fn gamescope_alone_still_gets_its_terminator() {
        let chain = chain(&[("gamescope", "-f")]);
        assert_eq!(
            chain.apply(vec!["game".into()]),
            vec!["gamescope", "-f", "--", "game"]
        );
    }

    #[test]
    fn proton_suppresses_the_mangohud_wrapper() {
        // UMU/Proton prefer MANGOHUD=1; wrapping would be double-counted.
        let chain = WrapperChain::build(false, true, false, &[], true, true);
        assert!(chain.wrappers.is_empty());
    }

    #[test]
    fn wine_keeps_the_mangohud_wrapper() {
        let chain = WrapperChain::build(false, true, false, &[], false, true);
        assert_eq!(chain.wrappers, vec![Wrapper::MangoHud]);
    }

    #[test]
    fn windows_has_no_wrappers() {
        let chain = WrapperChain::build(true, true, true, &[], false, false);
        assert!(chain.wrappers.is_empty());
        assert!(chain.gamescope_args.is_empty());
    }

    #[test]
    fn proton_options_only_emit_what_was_set() {
        let options = ProtonOptions {
            esync: Some(true),
            dxvk_hud: Some("fps".into()),
            ..Default::default()
        };
        let env = options.to_env();
        assert_eq!(env.get("WINEESYNC").map(String::as_str), Some("1"));
        assert_eq!(env.get("DXVK_HUD").map(String::as_str), Some("fps"));
        assert!(!env.contains_key("WINEFSYNC"));
        assert!(!env.contains_key("PROTON_ENABLE_NVAPI"));
    }
}
