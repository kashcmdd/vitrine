# Vitrine

![Rust](https://img.shields.io/badge/rust-stable-000000?logo=rust)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![License: MIT](https://img.shields.io/badge/license-MIT-green)

A desktop game launcher for PC games and emulated systems, for Windows and
Linux. A private collection, not a storefront: your cases sit on a shelf as
spines, and you pull one out to see its face.

> **Status:** pre-alpha. The architecture is settled and documented; the app is
> being built milestone by milestone. Nothing here is usable yet.

## What it will do

- **Library** — Steam, Epic, GOG, Lutris, Heroic, and hand-picked executables,
  alongside emulated titles for consoles.
- **Honest playtime** — the launcher tracks the whole process tree, so
  bootstrappers, `wineserver`, and emulator front-ends don't break the clock.
- **Linux compatibility** — Wine, Proton, GE-Proton and UMU-Launcher, with
  GameMode, MangoHud and GameScope wrappers.
- **Artwork** — Steam, SteamGridDB, IGDB, libretro-thumbnails and ScreenScraper,
  cached locally and usable offline.
- **Two shelf views** — spines or facing covers, which are the same scene at two
  rotations.

## What it will not do

It does not include, download, or link to games, ROMs, BIOS/firmware files, or
product keys. You supply what you already own. Emulated systems require firmware
you must dump from your own hardware.

## Documentation

- [Technical blueprint](docs/blueprint.md) — architecture, schema, milestones.
- [Decision records](docs/adr/) — why each load-bearing choice was made.

## Development

Nothing is wired up yet; `docs/blueprint.md` §11 describes the build order.

Planned toolchain: Rust (stable) + Tauri 2, Node 20+, and on Linux the usual
Tauri system packages (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`,
`libayatana-appindicator3-dev`, `librsvg2-dev`, `build-essential`).

## Licence

MIT — see [LICENSE](LICENSE). Third-party notices in
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).