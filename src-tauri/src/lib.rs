//! Tauri shell.
//!
//! Intentionally thin: plugins, state, and command registration. Every decision
//! lives in `vitrine-core`, which has no dependency on Tauri and is therefore
//! testable without a GUI toolkit.

mod commands;

use vitrine_core::{HostFacts, HostOs};

/// Facts about this host, resolved once at startup so the pure launch resolver
/// never has to probe the OS.
fn host_facts() -> HostFacts {
    fn on_path(binary: &str) -> bool {
        // Resolved here rather than in core: core stays I/O-free.
        std::env::var_os("PATH")
            .map(|paths| {
                std::env::split_paths(&paths).any(|dir| {
                    dir.join(binary).is_file() || dir.join(format!("{binary}.exe")).is_file()
                })
            })
            .unwrap_or(false)
    }

    let os = HostOs::current();
    HostFacts {
        os,
        has_gamemoderun: os.supports_wrappers() && on_path("gamemoderun"),
        has_mangohud: os.supports_wrappers() && on_path("mangohud"),
        has_gamescope: os.supports_wrappers() && on_path("gamescope"),
        has_umu: os.supports_wrappers() && on_path("umu-run"),
    }
}

pub fn run() {
    let facts = host_facts();
    tracing::info!(
        os = ?facts.os,
        umu = facts.has_umu,
        gamescope = facts.has_gamescope,
        "host facts resolved"
    );

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(facts)
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::host_facts,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Vitrine");
}
