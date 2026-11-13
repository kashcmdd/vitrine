//! Launch plan resolution.
//!
//! [`resolve`] is a **pure function**: no I/O, no clock, no OS queries, no
//! `#[cfg]`. Everything it needs about the host arrives in [`HostFacts`]. That
//! is what makes the trickiest part of the application — composing wrappers,
//! compatibility layers and emulator templates — exhaustively unit-testable.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::domain::{
    Game, LaunchConfig, LaunchPlan, Runner, RunnerKind, Target, WrapperChain,
};
use crate::error::PlanError;
use crate::{HostFacts, HostOs};

/// The runner chosen for this launch, plus the argv template that applies when
/// it is an emulator bound to this game's system.
pub struct RunnerBinding<'a> {
    pub runner: &'a Runner,
    /// Tokens from `emulator_systems.args_template`. Empty for non-emulators.
    pub args_template: &'a [String],
}

impl<'a> RunnerBinding<'a> {
    pub fn emulator(runner: &'a Runner, template: &'a [String]) -> Self {
        Self {
            runner,
            args_template: template,
        }
    }
}

/// Resolves a game, its chosen preset and the selected runner into an exact
/// command line and environment.
pub fn resolve(
    game: &Game,
    config: &LaunchConfig,
    binding: Option<RunnerBinding<'_>>,
    host: &HostFacts,
) -> Result<LaunchPlan, PlanError> {
    let target = resolve_target(game, config)?;

    // A ROM can only be launched by something that knows how to launch ROMs.
    if matches!(target, Target::Rom { .. }) {
        match binding.as_ref().map(|b| b.runner.kind) {
            Some(kind) if kind.launches_roms() => {}
            Some(kind) => return Err(PlanError::RunnerCannotLaunchRom(kind.as_str())),
            None => return Err(PlanError::RunnerCannotLaunchRom("none")),
        }
    }

    let kind = binding.as_ref().map(|b| b.runner.kind);
    let env = resolve_env(game, config, binding.as_ref(), host, kind);
    let mut argv = resolve_argv(&target, game, config, binding.as_ref())?;

    // Wrappers are a Linux concept; on Windows the chain is empty by construction.
    let chain = WrapperChain::build(
        config.use_gamemode && host.has_gamemoderun,
        config.use_mangohud && host.has_mangohud,
        config.use_gamescope && host.has_gamescope,
        &config.gamescope_args,
        kind.is_some_and(RunnerKind::prefers_mangohud_env),
        host.os.supports_wrappers(),
    );
    argv = chain.apply(argv);

    Ok(LaunchPlan {
        game_id: game.id.clone(),
        config_id: config.id.clone(),
        cwd: working_directory(game, &target),
        argv,
        env,
        wrappers: chain,
        target,
        log_name: format!("{}.log", game.id.as_str()),
    })
}

fn resolve_target(game: &Game, config: &LaunchConfig) -> Result<Target, PlanError> {
    if let Some(system_id) = game.system_id.clone() {
        return match &game.rom {
            Some(file) => Ok(Target::Rom {
                system: system_id,
                file: file.clone(),
            }),
            None => Err(PlanError::NoTarget),
        };
    }

    let exe = config
        .executable_override
        .clone()
        .or_else(|| game.executable_path.clone())
        .ok_or(PlanError::NoTarget)?;

    Ok(Target::Process { exe })
}

fn working_directory(game: &Game, target: &Target) -> Option<PathBuf> {
    if let Some(dir) = &game.working_directory {
        return Some(dir.clone());
    }
    match target {
        Target::Process { exe } => exe.parent().map(Path::to_path_buf),
        Target::Rom { .. } => None,
    }
}

fn resolve_env(
    game: &Game,
    config: &LaunchConfig,
    binding: Option<&RunnerBinding<'_>>,
    host: &HostFacts,
    kind: Option<RunnerKind>,
) -> BTreeMap<String, String> {
    // Precedence, lowest first: runner defaults, game, config, Proton toggles.
    let mut env = BTreeMap::new();
    if let Some(binding) = binding {
        env.extend(binding.runner.default_env.clone());
    }
    env.extend(game.env.clone());
    env.extend(config.env_override.clone());
    env.extend(config.proton_options.to_env());

    // MangoHud: Proton/UMU want the environment variable, not a wrapper.
    if config.use_mangohud && kind.is_some_and(RunnerKind::prefers_mangohud_env) {
        env.insert("MANGOHUD".into(), "1".into());
    }

    // Required environment. Applied last, so a user cannot break a working launch
    // by setting these to something that contradicts the selected runner.
    if let Some(binding) = binding {
        let runner = binding.runner;
        match runner.kind {
            RunnerKind::Wine => {
                if let Some(prefix) = &runner.prefix_path {
                    env.insert("WINEPREFIX".into(), path_str(prefix));
                }
                env.insert("WINEARCH".into(), "win64".into());
            }
            RunnerKind::Proton => {
                if let Some(prefix) = &runner.prefix_path {
                    env.insert("STEAM_COMPAT_DATA_PATH".into(), path_str(prefix));
                }
            }
            RunnerKind::Umu => {
                env.insert("GAMEID".into(), runner.umu_game_id_or_default().to_owned());
                if let Some(program) = &runner.executable_path {
                    env.insert("PROTONPATH".into(), path_str(program));
                }
                if let Some(prefix) = &runner.prefix_path {
                    env.insert("WINEPREFIX".into(), path_str(prefix));
                }
            }
            RunnerKind::Native | RunnerKind::Emulator | RunnerKind::Custom => {}
        }
    }

    // A Windows binary launched natively on Linux is always a mistake, but it is
    // not this function's job to forbid it: the caller decides whether a runner
    // is required.
    let _ = host;

    env
}

fn resolve_argv(
    target: &Target,
    game: &Game,
    config: &LaunchConfig,
    binding: Option<&RunnerBinding<'_>>,
) -> Result<Vec<String>, PlanError> {
    let user_args: &[String] = if config.arguments.is_empty() {
        &game.arguments
    } else {
        &config.arguments
    };

    let mut argv: Vec<String> = Vec::new();

    match target {
        Target::Process { exe } => {
            let exe = path_str(exe);
            match binding.map(|b| b.runner.kind) {
                // No runner, or a runner that just delegates to the binary.
                None | Some(RunnerKind::Native | RunnerKind::Emulator | RunnerKind::Custom) => {
                    argv.push(exe);
                }
                Some(RunnerKind::Wine) => {
                    argv.push(runner_program(binding, "wine")?);
                    argv.push(exe);
                }
                Some(RunnerKind::Proton) => {
                    // Proton's CLI is build-dependent; `run` is the stable entry
                    // point in current builds. UMU remains the recommended path.
                    argv.push(runner_program(binding, "proton")?);
                    argv.push("run".into());
                    argv.push(exe);
                }
                Some(RunnerKind::Umu) => {
                    argv.push(runner_program(binding, "umu")?);
                    argv.push(exe);
                }
            }
            argv.extend(user_args.iter().cloned());
        }

        Target::Rom { file, .. } => {
            let binding = binding.ok_or(PlanError::RunnerCannotLaunchRom("none"))?;
            let rom = path_str(file);
            let core = binding
                .runner
                .core_path
                .as_ref()
                .map(|core| path_str(core));

            let mut tokens = Vec::with_capacity(binding.args_template.len());
            for token in binding.args_template {
                if token.contains("{core}") && core.is_none() {
                    return Err(PlanError::UnresolvedPlaceholder("core"));
                }
                if token.contains("{rom}") && rom.is_empty() {
                    return Err(PlanError::UnresolvedPlaceholder("rom"));
                }
                let token = token.replace("{rom}", &rom);
                let token = match &core {
                    Some(core) => token.replace("{core}", core),
                    None => token,
                };
                tokens.push(token);
            }

            // A Flatpak emulator must be invoked through flatpak, not at its
            // sandboxed binary path.
            match &binding.runner.flatpak_id {
                Some(id) => {
                    argv.push("flatpak".into());
                    argv.push("run".into());
                    argv.push(id.clone());
                }
                None => argv.push(runner_program(Some(binding), "emulator")?),
            }

            argv.extend(tokens);
            argv.extend(user_args.iter().cloned());
        }
    }

    if argv.is_empty() {
        return Err(PlanError::NoTarget);
    }

    Ok(argv)
}

fn runner_program(
    binding: Option<&RunnerBinding<'_>>,
    label: &'static str,
) -> Result<String, PlanError> {
    binding
        .and_then(|b| b.runner.executable_path.as_ref())
        .map(path_str)
        .ok_or(PlanError::RunnerHasNoExecutable(label))
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        GameId, LaunchConfigId, LaunchSource, ProtonOptions, RunnerId, SystemId, TargetPlatform,
    };

    fn linux_host() -> HostFacts {
        HostFacts {
            os: HostOs::Linux,
            has_gamemoderun: true,
            has_mangohud: true,
            has_gamescope: true,
            has_umu: true,
        }
    }

    fn windows_host() -> HostFacts {
        HostFacts {
            os: HostOs::Windows,
            has_gamemoderun: false,
            has_mangohud: false,
            has_gamescope: false,
            has_umu: false,
        }
    }

    fn game(platform: TargetPlatform) -> Game {
        Game {
            id: GameId::from_db("g1"),
            title: "Hades".into(),
            sort_title: "hades".into(),
            description: None,
            system_id: None,
            executable_path: Some(PathBuf::from("/games/Hades.exe")),
            working_directory: None,
            arguments: vec!["--flag".into()],
            env: BTreeMap::new(),
            target_platform: platform,
            source: LaunchSource::Manual,
            source_app_id: None,
            install_dir: None,
            runner_id: None,
            playtime_seconds: 0,
            last_played_at: None,
            launch_count: 0,
            is_favorite: false,
            is_hidden: false,
            spine_accent: None,
            rom: None,
        }
    }

    fn rom_game() -> Game {
        let mut game = game(TargetPlatform::Native);
        game.system_id = Some(SystemId::from_db("ps2"));
        game.executable_path = None;
        game.arguments.clear();
        game.rom = Some(PathBuf::from("/roms/Shadow of the Colossus.chd"));
        game
    }

    fn config() -> LaunchConfig {
        LaunchConfig {
            id: LaunchConfigId::from_db("c1"),
            game_id: GameId::from_db("g1"),
            label: "Default".into(),
            executable_override: None,
            arguments: Vec::new(),
            env_override: BTreeMap::new(),
            runner_id: None,
            use_gamemode: false,
            use_mangohud: false,
            use_gamescope: false,
            gamescope_args: Vec::new(),
            proton_options: ProtonOptions::default(),
            is_default: true,
        }
    }

    fn runner(id: &str, kind: RunnerKind, program: &str) -> Runner {
        Runner {
            id: RunnerId::from_db(id),
            name: id.into(),
            kind,
            executable_path: Some(PathBuf::from(program)),
            prefix_path: Some(PathBuf::from("/prefixes/hades")),
            umu_game_id: Some("umu-12345".into()),
            core_path: None,
            flatpak_id: None,
            default_env: BTreeMap::new(),
            is_auto_detected: true,
            is_default: false,
        }
    }

    #[test]
    fn native_launch_is_just_the_executable() {
        let plan = resolve(&game(TargetPlatform::Native), &config(), None, &windows_host()).unwrap();
        assert_eq!(plan.argv, vec!["/games/Hades.exe", "--flag"]);
        assert_eq!(plan.program(), "/games/Hades.exe");
        assert_eq!(plan.cwd, Some(PathBuf::from("/games")));
        assert!(plan.wrappers.wrappers.is_empty());
    }

    #[test]
    fn wine_inserts_the_wine_binary_before_the_exe() {
        let wine = runner("wine", RunnerKind::Wine, "/usr/bin/wine");
        let binding = RunnerBinding::emulator(&wine, &[]);
        let plan = resolve(&game(TargetPlatform::Windows), &config(), Some(binding), &linux_host())
            .unwrap();
        assert_eq!(
            plan.argv,
            vec!["/usr/bin/wine", "/games/Hades.exe", "--flag"]
        );
        assert_eq!(plan.env.get("WINEPREFIX").map(String::as_str), Some("/prefixes/hades"));
        assert_eq!(plan.env.get("WINEARCH").map(String::as_str), Some("win64"));
    }

    #[test]
    fn umu_sets_game_id_and_proton_path() {
        let umu = runner("umu", RunnerKind::Umu, "/usr/bin/umu-run");
        let binding = RunnerBinding::emulator(&umu, &[]);
        let plan = resolve(&game(TargetPlatform::Windows), &config(), Some(binding), &linux_host())
            .unwrap();
        assert_eq!(
            plan.argv,
            vec!["/usr/bin/umu-run", "/games/Hades.exe", "--flag"]
        );
        assert_eq!(plan.env.get("GAMEID").map(String::as_str), Some("umu-12345"));
        assert_eq!(plan.env.get("PROTONPATH").map(String::as_str), Some("/usr/bin/umu-run"));
    }

    #[test]
    fn emulator_substitutes_rom_and_core() {
        let mut retroarch = runner("ra", RunnerKind::Emulator, "/usr/bin/retroarch");
        retroarch.core_path = Some(PathBuf::from("/cores/pcsx2.so"));
        retroarch.prefix_path = None;
        let template = vec![
            "-L".to_owned(),
            "{core}".to_owned(),
            "{rom}".to_owned(),
        ];
        let binding = RunnerBinding::emulator(&retroarch, &template);

        let plan = resolve(&rom_game(), &config(), Some(binding), &linux_host()).unwrap();
        assert_eq!(
            plan.argv,
            vec![
                "/usr/bin/retroarch",
                "-L",
                "/cores/pcsx2.so",
                "/roms/Shadow of the Colossus.chd",
            ]
        );
    }

    #[test]
    fn flatpak_emulator_goes_through_flatpak_run() {
        let mut retroarch = runner("ra", RunnerKind::Emulator, "/app/bin/retroarch");
        retroarch.core_path = Some(PathBuf::from("/cores/mgba.so"));
        retroarch.flatpak_id = Some("org.libretro.RetroArch".into());
        let template = vec!["-L".to_owned(), "{core}".to_owned(), "{rom}".to_owned()];
        let binding = RunnerBinding::emulator(&retroarch, &template);

        let plan = resolve(&rom_game(), &config(), Some(binding), &linux_host()).unwrap();
        assert_eq!(plan.argv[0], "flatpak");
        assert_eq!(plan.argv[1], "run");
        assert_eq!(plan.argv[2], "org.libretro.RetroArch");
    }

    #[test]
    fn rom_without_an_emulator_is_refused() {
        let umu = runner("umu", RunnerKind::Umu, "/usr/bin/umu-run");
        let binding = RunnerBinding::emulator(&umu, &[]);
        let error = resolve(&rom_game(), &config(), Some(binding), &linux_host()).unwrap_err();
        assert_eq!(error, PlanError::RunnerCannotLaunchRom("umu"));
    }

    #[test]
    fn rom_without_a_runner_is_refused() {
        let error = resolve(&rom_game(), &config(), None, &linux_host()).unwrap_err();
        assert_eq!(error, PlanError::RunnerCannotLaunchRom("none"));
    }

    #[test]
    fn a_game_with_no_target_is_refused() {
        let mut broken = game(TargetPlatform::Native);
        broken.executable_path = None;
        let error = resolve(&broken, &config(), None, &linux_host()).unwrap_err();
        assert_eq!(error, PlanError::NoTarget);
    }

    #[test]
    fn wrappers_compose_outermost_first() {
        let mut config = config();
        config.use_gamemode = true;
        config.use_mangohud = true;
        config.use_gamescope = true;
        config.gamescope_args = vec!["-W".into(), "2560".into(), "-H".into(), "1440".into(), "-f".into()];

        let plan = resolve(&game(TargetPlatform::Native), &config, None, &linux_host()).unwrap();
        assert_eq!(
            plan.argv,
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
                "/games/Hades.exe",
                "--flag",
            ]
        );
    }

    #[test]
    fn windows_ignores_wrapper_toggles() {
        let mut config = config();
        config.use_gamemode = true;
        config.use_mangohud = true;
        config.use_gamescope = true;

        let plan = resolve(&game(TargetPlatform::Native), &config, None, &windows_host()).unwrap();
        assert_eq!(plan.argv, vec!["/games/Hades.exe", "--flag"]);
    }

    #[test]
    fn missing_wrapper_binaries_are_skipped_rather_than_failing() {
        let mut config = config();
        config.use_gamemode = true;
        config.use_mangohud = true;

        let host = HostFacts {
            has_gamemoderun: false,
            has_mangohud: true,
            ..linux_host()
        };
        let plan = resolve(&game(TargetPlatform::Native), &config, None, &host).unwrap();
        assert_eq!(plan.argv[0], "mangohud");
    }

    #[test]
    fn proton_suppresses_the_mangohud_wrapper_but_sets_the_variable() {
        let mut config = config();
        config.use_mangohud = true;
        let proton = runner("proton", RunnerKind::Proton, "/proton/proton");
        let binding = RunnerBinding::emulator(&proton, &[]);

        let plan = resolve(
            &game(TargetPlatform::Windows),
            &config,
            Some(binding),
            &linux_host(),
        )
        .unwrap();

        assert!(!plan.argv.iter().any(|arg| arg == "mangohud"));
        assert_eq!(plan.env.get("MANGOHUD").map(String::as_str), Some("1"));
        assert_eq!(
            plan.env.get("STEAM_COMPAT_DATA_PATH").map(String::as_str),
            Some("/prefixes/hades")
        );
    }

    #[test]
    fn config_arguments_override_game_arguments() {
        let mut config = config();
        config.arguments = vec!["-windowed".into()];
        let plan = resolve(&game(TargetPlatform::Native), &config, None, &windows_host()).unwrap();
        assert_eq!(plan.argv, vec!["/games/Hades.exe", "-windowed"]);
    }

    #[test]
    fn a_runner_without_a_program_is_reported_clearly() {
        let mut wine = runner("wine", RunnerKind::Wine, "/usr/bin/wine");
        wine.executable_path = None;
        let binding = RunnerBinding::emulator(&wine, &[]);
        let error = resolve(&game(TargetPlatform::Windows), &config(), Some(binding), &linux_host())
            .unwrap_err();
        assert_eq!(error, PlanError::RunnerHasNoExecutable("wine"));
    }
}
