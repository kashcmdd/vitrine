# ADR-0006: AppImage and native packages; no Flatpak

**Status:** Accepted · 2026-11-13

## Context

Linux distribution options for a Tauri app: AppImage, `.deb`/`.rpm`, Flatpak,
Snap.

This application's entire value proposition on Linux is composed of operations a
sandbox interferes with:

- spawning host processes (games, emulators, `umu-run`, `gamemoderun`,
  `mangohud`, `gamescope`) with the user's environment;
- inspecting `/proc` to track process groups and descendants (ADR-0003);
- reading Steam, Epic, Heroic and Lutris libraries from the user's home;
- writing Wine prefixes into the user's filesystem;
- receiving systemd cgroup delegation for the optional tier-2 tracker.

## Decision

Ship **AppImage** and native **`.deb` / `.rpm`**. No Flatpak in v1.

## Consequences

**Positive**

- Process tracking works as designed: no `flatpak-spawn --host` indirection, full
  `/proc` visibility, no PID-namespace translation.
- Wrapper chain (`gamemoderun` → `mangohud` → `gamescope`) runs in the user's own
  session, so GameMode and MangoHud hooks behave exactly as they do from a
  terminal.
- Wine prefixes and emulator config land in normal locations, discoverable by
  other tools.
- `.deb`/`.rpm` install to a fixed path, which keeps `assetProtocol` scopes and
  desktop integration simple.

**Negative**

- An AppImage mounts at a temporary squashfs path (`/tmp/.mount_*`), so **all
  persistent data must be resolved through XDG locations**, never derived from
  the executable path. This is a hard rule in `util/paths.rs`.
- No Flathub reach. Accepted: distribution reach is irrelevant for personal use.
- WebKitGTK's own sandbox interacts with Ubuntu 23.04+ AppArmor user-namespace
  restrictions; AppImage must be tested on a clean 24.04 VM before it is
  promised.
- Snap: same objections as Flatpak, plus stricter confinement. Not considered.

## Revisit trigger

If the application ever needs to reach non-technical users on immutable distros
(Bazzite, Silverblue), a Flatpak build becomes worth the cost — but it would
require a different tracker implementation reading `flatpak-spawn` PID maps, and
honest documentation that playtime may be approximate.
