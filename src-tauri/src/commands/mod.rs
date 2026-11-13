//! The Tauri command layer.
//!
//! Rules (see `docs/blueprint.md` §4):
//!   * A command is a thin adapter: deserialize, call a service, map the error.
//!   * No branching logic. If a body contains an `if`, it belongs in a service.
//!   * Every command returns `Result<T, String>`, because Tauri serialises
//!     errors, not `thiserror` types.

use serde::Serialize;
use vitrine_core::{HostFacts, HostOs};

/// Static identity of the running build. The M0 round-trip: if this reaches the
/// frontend with correct types, the IPC contract is wired end to end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub blueprint_version: u32,
    pub os: &'static str,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "Vitrine",
        version: env!("CARGO_PKG_VERSION"),
        blueprint_version: 2,
        os: match HostOs::current() {
            HostOs::Windows => "windows",
            HostOs::Linux => "linux",
        },
    }
}

/// What this host can actually do. The UI uses this to grey out wrapper toggles
/// for binaries that are not installed, rather than offering an option that
/// silently does nothing.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCapabilities {
    pub os: &'static str,
    pub supports_wrappers: bool,
    pub has_gamemoderun: bool,
    pub has_mangohud: bool,
    pub has_gamescope: bool,
    pub has_umu: bool,
}

impl From<&HostFacts> for HostCapabilities {
    fn from(facts: &HostFacts) -> Self {
        Self {
            os: match facts.os {
                HostOs::Windows => "windows",
                HostOs::Linux => "linux",
            },
            supports_wrappers: facts.os.supports_wrappers(),
            has_gamemoderun: facts.has_gamemoderun,
            has_mangohud: facts.has_mangohud,
            has_gamescope: facts.has_gamescope,
            has_umu: facts.has_umu,
        }
    }
}

#[tauri::command]
pub fn host_facts(facts: tauri::State<'_, HostFacts>) -> HostCapabilities {
    HostCapabilities::from(facts.inner())
}
