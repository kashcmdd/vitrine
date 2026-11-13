# ADR-0001: Tauri 2 rather than Electron

**Status:** Accepted · 2026-11-13

## Context

The application is a Windows + Linux desktop launcher whose hardest problem is
not rendering — it is owning process trees, watching filesystems, parsing
manifests and spawning compatibility layers correctly. That work is native work.

Two viable shells exist: Electron (Chromium embedded, Node backend) and Tauri 2
(system webview, Rust backend).

The strongest available reference implementation, Boxshelf, is Electron. Its
rendering quality benefits directly from Chromium being present on every
platform — which is also the strongest argument *against* Tauri on Linux, where
the webview is WebKitGTK.

## Decision

Use Tauri 2 with a React/TypeScript frontend.

- Backend logic lives in Rust, in `crates/core`, with **no Tauri dependency**, so
  it is testable without a GUI toolkit.
- The OS-specific boundary (`#[cfg]`) is confined to two modules:
  `launch/tracker/` and `launch/identify/`.
- The webview contains no business logic.

## Consequences

**Positive**

- Job Objects, process groups, `prctl`, and `waitpid` are directly reachable;
  no N-API addon or shelling out to `taskkill`/`pkill`.
- ~12 MB bundle and one process per window instead of ~150 MB and a Node runtime.
- `tokio` handles the async I/O (manifest scanning, metadata fetches, process
  supervision) in one runtime.
- The core crate's test suite runs on a headless machine with no GTK installed.

**Negative**

- Linux rendering quality is capped by WebKitGTK. WebGL is materially weaker
  than Chromium's, especially on NVIDIA + Wayland and inside containers.
- Mitigation: the shelf is renderer-abstracted (ADR-0005) with a CSS-3D engine
  as a first-class fallback, selected by a boot-time capability probe.
- Escape hatch if Deck performance proves unacceptable: `tauri-runtime-cef`
  replaces the Linux runtime with Chromium. Treated as experimental, evaluated
  during M2, not designed around.

## Alternatives rejected

| Alternative | Why not |
|---|---|
| Electron | Inherits a 150 MB runtime and pushes process supervision into Node, where the Windows primitives needed for correct tracking are not available without a native addon |
| Fork Boxshelf (Electron + GPL) | See ADR-0002 |
| Native per-platform UI (Qt/Slint/egui) | Loses the flexible, CSS-driven visual language the design depends on |
