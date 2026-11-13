# Vitrine — Technical Blueprint

**Codename:** Vitrine (a display case). Final name TBD — see [open items](#12-open-items).

**One-line:** a cross-platform desktop game launcher for PC games and emulated
systems, built as a private collection rather than a storefront.

**Thesis.** Games rest as spines; you pull one out to see its face. Rust owns
processes, disk and network; the webview owns only pixels. Nothing in the app
requires a network call to be usable — the library is fully legible before a
single image is scraped.

**Status:** pre-M0. Decisions here are recorded as ADRs in [`docs/adr/`](adr/).

---

## Contents

1. [Scope](#1-scope)
2. [Locked decisions](#2-locked-decisions)
3. [Stack](#3-stack)
4. [Repository layout](#4-repository-layout)
5. [Data model](#5-data-model)
6. [Launch and process supervision](#6-launch-and-process-supervision)
7. [Discovery](#7-discovery)
8. [Metadata and artwork](#8-metadata-and-artwork)
9. [Design system](#9-design-system)
10. [First-run setup](#10-first-run-setup)
11. [Milestones](#11-milestones)
12. [Open items](#12-open-items)

---

## 1. Scope

**In scope**

- Windows 10/11 and Linux (X11 + Wayland) desktops.
- PC games from Steam, Epic, GOG, Lutris, Heroic, plus manually added
  executables.
- Emulated titles across console systems, launched through RetroArch,
  EmuDeck / RetroDECK, or standalone emulators.
- Linux compatibility layers: Wine, Proton, GE-Proton, UMU-Launcher.
- Accurate playtime via real process-tree supervision.
- Offline-first artwork and metadata.

**Out of scope (v1)**

- macOS.
- Bundling, downloading, or linking ROMs, BIOS/firmware files, or product keys.
- App stores or proprietary redistribution.
- Save-state / save-file synchronisation beyond playtime accounting.
- Steam Deck Gaming Mode (Desktop Mode only until M3 is stable).

**Distribution:** AppImage, `.deb`, `.rpm`. No Flatpak — its sandbox breaks
process-group tracking and the wrapper chain (ADR-0006).

---

## 2. Locked decisions

Each has a full ADR. They are listed here so a reader can see the whole
posture at once.

| ADR | Decision | Reversal cost |
|---|---|---|
| [0001](adr/0001-tauri-not-electron.md) | Tauri 2 + React/TS, not Electron | high |
| [0002](adr/0002-own-code-not-a-fork.md) | Own code under MIT; Boxshelf is reference material, never a source of files | irreversible once copied |
| [0003](adr/0003-job-objects-and-process-groups.md) | Process containment is the source of truth, not the root PID | medium |
| [0004](adr/0004-emulators-are-runners.md) | Emulators are runners, not a parallel launch system | medium |
| [0005](adr/0005-shelf-view-is-a-rotation.md) | The shelf toggle is a rotation of one scene, not two scenes | cheap now, expensive later |
| [0006](adr/0006-appimage-and-native-packages.md) | AppImage + deb/rpm, no Flatpak | high |
| [0007](adr/0007-gamepad-shelf-mouse-chrome.md) | Gamepad-first shelf, mouse-first chrome | low |
| [0008](adr/0008-setup-step-registry.md) | First-run setup is a backend-owned step registry | low |

**Non-goals that follow from these:** no `tauri-plugin-shell` (all spawning is
Rust-side through the launch-plan layer), no API keys in SQLite (OS keychain
only), no silent library imports (discovery is a staging pipeline with human
approval).

---

## 3. Stack

### Frontend

| Concern | Choice | Why |
|---|---|---|
| Framework | React 19 + TypeScript + Vite | Largest ecosystem for virtualised grids, carousels, focus management |
| Styling | Tailwind 4 + Radix primitives | Menu-heavy UI needs accessible dialogs, context menus, comboboxes |
| Server state | TanStack Query v5 | `invoke` is a promise-based remote; caching and invalidation for free |
| Client state | Zustand (2 stores) | Ephemeral: running games, toasts |
| Virtualisation | `@tanstack/react-virtual` | A 400-game grid must not mount 400 nodes |
| 3D | Three.js, isolated to `src/shelf/engine/three/` | Only one module knows WebGL exists |
| IPC types | `tauri-specta` | Generated bindings; hand-written command strings are how Tauri apps rot |

### Backend

| Concern | Crate | Notes |
|---|---|---|
| Runtime | `tokio` | Tauri already pulls it; use it directly |
| Persistence | `sqlx` 0.8 (`sqlite`, `migrate`, `macros`) | Async, embedded migrations, compile-checked queries |
| Serialisation | `serde`, `serde_json` | |
| IDs / time | `uuid` v7, `chrono` | v7 is time-sortable → good index locality |
| HTTP | `reqwest` (rustls) + `reqwest-retry` | No OpenSSL dependency |
| Errors | `thiserror` (lib), `anyhow` (bin) | |
| Observability | `tracing`, `tracing-subscriber`, `tracing-appender` | Rotating logs |
| Secrets | `keyring` | IGDB / SteamGridDB / ScreenScraper credentials |
| Steam | `steamlocate`, `keyvalues-serde` | Library discovery + VDF parsing |
| Filesystem | `walkdir`, `notify`, `globset` | ROM/exe scanning, library watchers |
| Images | `image`, `sha2` | Content-addressed media cache, thumbnails |
| Process info | `sysinfo` | **UI only** — never the exit signal |
| Windows | `windows` crate | `Win32_System_JobObjects`, `Win32_System_Threading` |
| Linux | `nix`, `libc` | `setsid`, signals, `PR_SET_CHILD_SUBREAPER` |

**Tauri plugins:** `log`, `dialog`, `opener`, `single-instance`, `updater`,
`window-state`. Not `shell`.

### Persistence settings

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous  = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

One writer connection (capacity 1), unbounded readers. Every write goes through
`AppState::write()`; this removes `SQLITE_BUSY` storms without retry loops.

---

## 4. Repository layout

```
Cargo.toml                      # workspace: crates/core + src-tauri
src/                            # frontend
  main.tsx
  app/                          # shell, router, providers
  shelf/                        # the product's core bet
    scene.ts                    # ShelfScene, CaseSlot — renderer-agnostic
    layout/spineRow.ts
    layout/frontGrid.ts
    engine/three/               # renderer A
    engine/css3d/               # renderer B (software-WebGL fallback)
    engine/index.ts             # selectEngine() capability probe
    spine/generate.ts           # synthesised spine art -> webp
    focus/useFocusGrid.ts       # one focus engine, gamepad + pointer
  features/
    setup/                      # wizard shell + step registry client
    library/                    # grid, filters, context menu
    detail/                     # game detail, sessions, launch
    discovery/                  # review/approve staged candidates
    runners/                    # Wine/Proton/UMU + emulator config
    settings/
  components/ui/                # Radix-based primitives, tokens only
  lib/
    bindings.ts                 # GENERATED by tauri-specta
    events.ts
    queryClient.ts
  styles/tokens.css             # day + room primitives, semantic layer
src-tauri/
  src/
    main.rs                     # thin
    lib.rs                      # builder (kept thin for tests)
    app.rs                      # plugins, state, setup hook
    commands/                   # <=5 lines each: deserialize -> service -> map error
    state.rs                    # AppState, RunningRegistry
    events.rs                   # event names + payload structs
crates/core/                    # NO tauri dependency — testable without a GUI stack
  src/
    domain/                     # pure types, no I/O, no cfg
    db/                         # pool, repos
    discovery/
    launch/                     # plan resolution, wrappers, proton, identify
    metadata/
    service/
    util/
  migrations/
docs/
  blueprint.md
  adr/
```

**Boundary rules**

- `#[cfg(target_os = ...)]` appears **only** in `crates/core/src/launch/tracker/`
  and `crates/core/src/launch/identify/`. Everything else is platform-neutral.
- `crates/core` must not depend on `tauri`, so `cargo test -p vitrine-core` runs
  on a machine with no GUI toolkit installed.
- Commands contain no branching logic. If a command body has an `if`, it belongs
  in `service/`.

---

## 5. Data model

Conventions: `STRICT` tables, UUIDv7 text primary keys, UTC ISO-8601 text
timestamps (`...Z`), enums as `TEXT` + `CHECK`, structured values as JSON
columns validated in Rust.

### Entities

```
systems ──< games >── runners
              │         │
              ├──< launch_configs >──┘
              ├──< rom_files
              ├──< artworks
              ├──< game_metadata
              └──< play_sessions
systems >──< emulator_systems >──< runners
```

- `games.system_id IS NULL` ⇒ PC game.
- `games.system_id NOT NULL` ⇒ emulated title.
- Emulators are `runners` with `kind = 'emulator'` (ADR-0004).

### Full DDL

The authoritative schema lives in
[`crates/core/migrations/0001_init.sql`](../crates/core/migrations/0001_init.sql).
Design notes that matter more than the DDL:

**`systems`** carries physical case geometry — `box_aspect` (width ÷ height),
`box_depth`, `media_kind`. These drive the 3D case proportions, so the artwork
pipeline and the renderer read the same numbers. Reference values: CD jewel
≈ 0.95, DVD/Blu-ray ≈ 0.70, Switch ≈ 0.62, Game Boy cart ≈ 1.00.

**`play_sessions.duration_seconds`** is a `STORED` generated column, so the UI
and the aggregate repair job can never disagree about a session's length.
`idx_sessions_one_running` is a partial unique index on `status = 'running'` —
double-launch becomes a database constraint instead of a race in Rust.

**`games.playtime_seconds`** is a denormalised cache, recomputed in a
transaction on session close and again by `service::playtime::recompute_all()`
at boot. Self-healing, no triggers.

**`artworks`** allows many candidates per `(game, kind)` with at most one
`is_selected`, enforced by a partial unique index.

**`discovered_candidates`** is a staging table. Scanners never write to `games`.

---

## 6. Launch and process supervision

This is the part that decides whether the app works.

### 6.1 Plan resolution

```rust
// crates/core/src/launch/plan.rs
pub fn resolve(
    game: &Game,
    config: &LaunchConfig,
    runner: Option<&Runner>,
    host: &HostFacts,
) -> Result<LaunchPlan, PlanError>;
```

**Pure function.** No I/O, no `cfg`. This is where all cross-platform
composition lives, and it is unit-tested exhaustively.

```rust
pub struct LaunchPlan {
    pub game_id: GameId,
    pub config_id: LaunchConfigId,
    pub cwd: Option<PathBuf>,
    pub argv: Vec<String>,          // argv[0] is the program
    pub env: BTreeMap<String, String>,
    pub wrappers: Vec<Wrapper>,     // for display and kill semantics
    pub log_path: PathBuf,
}

pub enum Target {
    Process { exe: PathBuf },
    Rom { system: SystemId, file: PathBuf, runner: RunnerId },
}
```

Composition order on Linux, outermost first:

```
gamemoderun → mangohud → gamescope <args> -- → runner → target
```

Rules encoded in `launch/wrappers.rs`, not guessed at call sites:

- `gamemoderun` wraps `mangohud` (GameMode must be outermost to affect it).
- `gamescope` takes `--` before the wrapped command.
- With a Proton runner, prefer the `MANGOHUD=1` environment variable over
  wrapping the Wine binary — Proton's env path is more reliable.
- On Windows every wrapper is a no-op.

Environment precedence, last wins:

```
runner.default_env
  → games.env_vars
    → launch_config.env_override
      → hard-required runner env (never overridable)
```

Arguments are stored as `Vec<String>`. The UI's raw text field is tokenised by
`util::tokenize` and rendered back as a chip list; nothing is ever passed to a
shell.

### 6.2 Runner matrix

| `runner.kind` | argv prefix | required env |
|---|---|---|
| `native` | `[exe, ...args]` | — |
| `wine` | `[wine, exe, ...args]` | `WINEPREFIX`, `WINEARCH`, `WINEDLLOVERRIDES` |
| `proton` | Proton's run entry, then `waitforexitandrun exe` | `STEAM_COMPAT_DATA_PATH`, `STEAM_COMPAT_CLIENT_INSTALL_PATH`, `PROTON_*` |
| `umu` (**Linux default**) | `[umu-run, exe, ...args]` | `GAMEID`, `PROTONPATH`, `WINEPREFIX` |
| `emulator` | from `emulator_systems.args_template`, e.g. `[retroarch, -L, {core}, {rom}]` | per-system |

UMU is the default for non-Steam Windows games on Linux: it assembles the
`STEAM_COMPAT_*` surface we would otherwise own forever.

### 6.3 Tracker contract

```rust
#[async_trait]
pub trait ProcessTracker: Send + Sync {
    async fn spawn(&self, plan: &LaunchPlan) -> Result<SpawnedProcess>;
    async fn wait_for_exit(&mut self, cancel: CancellationToken) -> Result<ExitReport>;
    fn snapshot(&self) -> Vec<ProcInfo>;          // UI process panel
    async fn terminate(&mut self, grace: Duration) -> Result<()>;
}

pub struct ExitReport {
    pub status: SessionStatus,          // exited | crashed | killed
    pub exit_code: Option<i32>,
    pub exit_signal: Option<i32>,
    pub duration: Duration,
}
```

#### Windows

1. `CreateJobObjectW`, then
   `SetInformationJobObject(JobObjectExtendedLimitInformation)` with
   `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
2. `CreateProcessW` with a `STARTUPINFOEX` carrying
   `PROC_THREAD_ATTRIBUTE_JOB_LIST([job])` — the job is attached **at
   creation**, so a bootstrapper that spawns a child in its first instruction is
   still contained. Flags: `CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT |
   CREATE_NEW_PROCESS_GROUP`, then `ResumeThread`.
3. Fallback for pre-Win10-1809: spawn, assign immediately, then a
   `CreateToolhelp32Snapshot` catch-up pass that adopts already-created
   descendants.
4. Probe `IsProcessInJob(GetCurrentProcess(), NULL)` once at startup to decide
   whether `CREATE_BREAKAWAY_FROM_JOB` is needed (launchers started from a
   terminal or a store sandbox are often already inside a job).

**Exit signal:** poll
`QueryInformationJobObject(JobObjectBasicAccountingInformation)` every 250 ms and
treat `ActiveProcesses == 0` as exit. The root `child.wait()` is collected only
for its exit code, and is *not* the signal — bootstrappers exit in under two
seconds while the job stays populated.

`KILL_ON_JOB_CLOSE` also guarantees no game survives a launcher crash: the
handle closes and the whole tree dies.

**Crash heuristic:** job empties, elapsed < 10 s, and root exit code ≠ 0
⇒ `crashed`. A non-zero code after 10 s is a normal `exited` — games exit
non-zero all the time.

#### Linux

1. `prctl(PR_SET_CHILD_SUBREAPER, 1)` once at startup, so orphaned
   grandchildren re-parent to us and can be reaped.
2. `Command::pre_exec(|| { setsid(); Ok(()) })` — new session, pgid == child pid.
3. **Membership is a union of two sets**, recomputed each tick:
   - group membership — every pid whose `pgrp` equals `pgid`;
   - descendant closure — BFS over the PPID graph from the root, which catches
     anything that called `setsid()`/`setpgid()` and escaped the group.
     This is what makes `wineserver` and Electron-style helper chains visible.
4. After the root exits, keep ticking until the union empties, bounded by a
   grace window (default 30 s, per-game override — `wineserver` legitimately
   lingers).

**Terminate:** `killpg(SIGTERM)` → grace (5 s) → `killpg(SIGKILL)` → sweep the
descendant closure for escapees.

**Optional tier 2:** run the game in a cgroup v2 scope via
`systemd-run --user --scope`, then read `cgroup.procs` for exact membership plus
`cpu.stat` / `memory.peak` for a stats panel. Guarded by a delegation check with
graceful fallback.

**Why `sysinfo` is not the answer:** its refresh granularity and PID-reuse
behaviour make it unusable as an exit signal. It is a display source only.

### 6.4 Session lifecycle

```
launch_game(game_id, config_id?)
  ├─ preflight: target exists, runner resolvable, prefix writable
  ├─ INSERT play_sessions(status='running', last_heartbeat=now)   -- unique index blocks doubles
  ├─ UPDATE games SET last_played_at, launch_count = launch_count + 1
  ├─ tracker.spawn(plan)
  ├─ emit game-started { gameId, sessionId, pid, startedAt, argvPreview }
  └─ return { sessionId }

supervisor task
  ├─ select! {
  │    report = tracker.wait_for_exit(cancel) => close_session(report)
  │    _ = heartbeat.tick(20s) => touch last_heartbeat; emit game-heartbeat { elapsedSeconds }
  │  }
  └─ close_session → recompute aggregates → emit game-exited {
       gameId, sessionId, status, exitCode, durationSeconds, newTotalSeconds }
```

### 6.5 Events

| Event | Payload |
|---|---|
| `game-started` | gameId, sessionId, pid, startedAt, argvPreview, envKeys |
| `game-heartbeat` | gameId, sessionId, elapsedSeconds |
| `game-exited` | gameId, sessionId, status, exitCode, durationSeconds, newTotalSeconds |
| `game-output` | sessionId, line (throttled to ~4/s) |
| `session-recovered` | gameId, sessionId, recoveredSeconds, reason |
| `discovery-changed` | scanner, added, removed |
| `metadata-updated` | gameId, kinds |
| `metadata-progress` | queued, done, failed |

All payload types are exported to TypeScript by `tauri-specta`.

### 6.6 Crash recovery

On boot, `service::recovery::reconcile()`:

1. For every `status = 'running'` row, check liveness using PID **plus process
   start time** (defeats PID reuse).
2. Dead ⇒ close at `last_heartbeat` with `status = 'orphaned'`, recompute
   aggregates, emit `session-recovered`. This is why the heartbeat exists: a
   `kill -9` of the launcher still yields honest playtime.
3. Alive ⇒ reattach to the process group and re-emit `game-started`. On Windows
   this is usually moot because `KILL_ON_JOB_CLOSE` already reaped the tree, and
   that is correct behaviour.

---

## 7. Discovery

```rust
#[async_trait]
pub trait LibraryScanner: Send + Sync {
    fn id(&self) -> ScannerId;
    async fn detect_roots(&self) -> Result<Vec<PathBuf>>;
    async fn scan(&self, roots: &[PathBuf]) -> Result<Vec<DiscoveredCandidate>>;
    fn watch_paths(&self, roots: &[PathBuf]) -> Vec<PathBuf> { Vec::new() }
}
```

Scanners run concurrently with failure isolation (one malformed VDF must not
abort the sweep) and write to `discovered_candidates`. **Nothing is imported
without human approval.**

| Scanner | Source | Notes |
|---|---|---|
| Steam | `steamlocate` → `libraryfolders.vdf` → `appmanifest_*.acf` | `StateFlags & 4` = fully installed; denylist runtimes, Proton, soundtracks, dedicated servers |
| Epic | `%PROGRAMDATA%\Epic\EpicGamesLauncher\Data\Manifests\*.item` | On Linux, Heroic's `legendaryConfig/legendary/installed.json` |
| GOG | `goggame-*.info` adjacent to executables | Heroic's `gog_store/installed.json` on Linux |
| Lutris | `~/.local/share/lutris/pga.db` read-only (`?mode=ro&immutable=1`) | More reliable than parsing YAML |
| Emulators | RetroArch, Flatpak, EmuDeck, RetroDECK, AppImage, standalone | Produces `runners` + `emulator_systems` |
| ROMs | configured roots only | Extension + magic classification; `.m3u` collapses multi-disc sets |
| Local exe | configured roots only, depth ≤ 4 | PE/ELF magic, size ≥ 256 KB, name scoring, suppressed inside any claimed install dir |

### Matching

```
score = 0.55 · title_similarity(normalise(a), normalise(b))
      + 0.25 · (install_dir or exe-parent match)
      + 0.20 · (source_app_id equality)
```

`≥ 0.90` pre-selects "update existing"; `0.45–0.90` shows "possible duplicate";
below that it is a new entry. Approval is always explicit.

### Watchers

`notify` on `libraryfolders.vdf`, each `steamapps/`, Epic's Manifests directory,
and Heroic's config, debounced 2 s, emitting `discovery-changed` and raising a
toast. Never a silent mutation.

---

## 8. Metadata and artwork

### Provider chain

First success per artwork kind wins; all results are retained as alternatives.

| Order | Provider | Covers | Key | Limits |
|---|---|---|---|---|
| 1 | Steam CDN | Steam games | none | none |
| 2 | SteamGridDB | PC | API key | ~1 req/s |
| 3 | IGDB | PC metadata text | Twitch client id/secret | 4 req/s |
| 4 | libretro-thumbnails | consoles | none | be polite, mirror + hash |
| 5 | ScreenScraper | consoles incl. spine + disc label + manual | dev credentials | per-account thread caps |

Steam CDN paths (`source_app_id` is a licence to print artwork):

```
https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/header.jpg            460x215
https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_600x900.jpg   600x900
https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_hero.jpg      3840x1240
https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/logo.png
```

### Pipeline

```
enqueue { game_id, kinds, priority }
  → bounded worker pool Semaphore(3), 1 concurrent request per provider
  → match (title similarity + year + platform), accept >= 0.82 else needs_confirmation
  → download → sha256 → media/blobs/<hash>.<ext>
  → decode/resize → front 600x900, thumb 300x450, hero 1920x620 → webp
  → INSERT artworks (is_selected=1 only if that kind had no selection)
  → emit metadata-updated
```

Backoff: `retry_at = now + min(2^attempt, 3600)s ± 20% jitter`, `attempts ≤ 3`.
Offline errors defer the whole queue by 5 minutes instead of burning attempts.
Jobs are idempotent by `sha256(remote_url)`.

### Spine synthesis (mandatory)

Spine art barely exists in any API, and spine is the default view. So it is
generated locally:

```
spine_art(game) =
    scraped spine image (ScreenScraper, when present)
  | generated: system accent background
             + title in Fraunces, capped at 22 chars, single line
             + small-caps system badge
             + 1px brass rule
             → canvas → webp → media/games/<id>/spine.webp
```

Constraints: legible at 28 px wide; no rotated text below 40 px; deterministic
so re-scrapes do not churn the cache; per-system accent so a shelf reads as
grouped. This is what makes the grid usable with zero network access.

### Serving images

`convertFileSrc()` against an `assetProtocol` scope limited to
`$APPDATA/media/games/**`. Grid uses 300×450 thumbs; detail uses full size.
Fixed `width`/`height` on every `<img>` to prevent grid reflow.

---

## 9. Design system

Direction: **archival collector** — a private collection, not an arcade. Paper
and ink, high-contrast serif display type, catalogue numbers, sharp edges,
restrained motion.

### Tokens

Two primitive palettes over one semantic layer. An archival paper palette is
beautiful on a desk monitor and blinding in a dark room, so the inversion is
built into the token layer rather than retrofitted.

```css
/* day — paper */
--surface-0:#F6F1E8; --surface-1:#EFE8DB; --surface-2:#E4DACA;
--ink-900:#14120F;   --ink-700:#3A352D;   --ink-500:#6B6357; --ink-300:#A79C88;
--brass:#B4763A;     --brass-dim:#8C5A2B; --signal:#C2452D;

/* room — ink */
--surface-0:#14120F; --surface-1:#1C1916; --surface-2:#262119;
--ink-900:#F4EFE5;   --ink-700:#CFC6B6;   --ink-500:#948B7B; --ink-300:#5E574C;
--brass:#C98B4B;     --signal:#E0593C;
```

| Token | Value |
|---|---|
| Display | Fraunces (variable, `opsz` 9–144) — titles, system names, hero |
| UI | Inter Tight 400/500/600 — chrome, buttons, lists |
| Data | Geist Mono, tabular numerals — playtime, catalogue IDs |
| Scale | 12 14 16 20 24 32 44 64 · display `line-height` 1.05 · body 1.55 |
| Motion | 260 ms standard · 420 ms scene · `cubic-bezier(.22,1,.36,1)` · no overshoot |
| Reduced motion | 80 ms fade, rotation disabled |
| Elevation | 3 warm levels: `0 1px 3px` · `0 2px 8px` · `0 12px 32px` |
| Radius | 2 px cases · 0 chrome — sharp edges read as "object", not "app" |
| Materials | plastic `.55` roughness / `.30` clearcoat · paper `.90` · printed spine `.70` flat |
| Lighting | warm 4200 K key + soft fill + contact shadow; room mode = rim light + 8% accent glow |
| Focus | scale 1.06 · 2 px brass outline at 4 px offset · unfocused 0.85 |

No hex literals outside `src/styles/tokens.css`. A CI grep enforces this.

### The shelf

One `ShelfScene`, two layouts, two engines (ADR-0005).

```ts
type ShelfView = 'spine' | 'front';

interface CaseSlot {
  index: number;
  position: [number, number, number];
  rotationY: number;     // 90° = spine, 0° = front — the only thing the toggle changes
  scale: number;
  aspect: number;        // systems.box_aspect
  depth: number;
  faces: { front?: Art; spine?: Art; back?: Art; label?: Art };
  accent: string;
  title: string;
  system?: SystemId;
  focus: 'none' | 'hover' | 'selected';
}

interface ShelfScene {
  view: ShelfView;
  slots: CaseSlot[];
  camera: CameraSpec;
  lighting: 'day' | 'room';
}
```

- `spineRow` — continuous packing like books; cases touch; rows break by shelf
  height. Requires no cover art at all.
- `frontGrid` — rows × columns with gaps and a perspective camera.

Toggling views interpolates every case's `rotationY` with a 60 ms stagger over
420 ms. That transition is the signature motion, and it costs nothing extra
because both views are the same geometry.

**Engine selection** is a boot-time capability probe, not a user setting: create
the GL context, read `WEBGL_debug_renderer_info`; if the renderer matches
`llvmpipe|swrast|softpipe|warp`, or p95 frame time over the first 60 frames
exceeds 22 ms, fall back to the CSS-3D engine with identical layout and
interaction.

---

## 10. First-run setup

A real onboarding flow, implemented as a **step registry** so later milestones
add steps without touching the shell (ADR-0008, see
[`crates/core/src/setup`](../crates/core/src/setup)).

```rust
pub trait SetupStep: Send + Sync {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn availability(&self, ctx: &SetupCtx) -> Availability;
    async fn run(&mut self, ctx: &mut SetupCtx, ui: StepUi) -> Result<StepOutcome>;
}

pub enum Availability { Available, Pending(&'static str), Skipped }
pub enum StepOutcome  { Completed, Skipped, Back }
```

The frontend renders whatever the registry reports. At M1 the wizard ships three
working steps (welcome → library → appearance); M4 adds `detect` by registering
it, with no shell rewrite. `settings.setup_completed_at` records completion and
the whole flow is re-runnable from Settings.

---

## 11. Milestones

Each is independently shippable and demoable.

| Milestone | Scope | ≈ | Exit criteria |
|---|---|---|---|
| **M0** | Tauri shell, MIT + third-party notices, workspace split, migrations, typed IPC round-trip, updater manifest on the tailnet, CI matrix (Windows runner required for Job Object tests), token file, `diagnostics` scaffold | 1 wk | Both OSes build; one typed call round-trips; `cargo test -p vitrine-core` green on a machine with no GUI toolkit |
| **M1** | Manual **PC** game add, native launch, naive exit detection, sessions, events, **wizard shell + step registry + 3 steps** | 1.5 wk | Playtime ±2 s on a single-process game; wizard completes and persists |
| **M2** | **Shelf**: scene model, both layouts, Three.js + CSS-3D engines, spine synthesiser, day/room tokens, focus engine | 2.5 wk | 60 fps @1440p; spine view fully populated **offline**; Linux software-render fallback clean; Deck tested |
| **M3** | Real supervision: Job Objects, pgid + subreaper, heartbeats, orphan recovery, kill-with-grace, log capture | 1.5 wk | Five tracker tests pass on both OSes |
| **M4** | Discovery: Steam → Epic → Lutris/GOG → emulators → ROMs; wizard gains `detect` | 2 wk | 100+ Steam games in <2 s, zero duplicates; a RetroDECK tree imports with correct systems and collapsed multi-disc sets |
| **M5** | Runners: Proton/UMU, wrappers, emulator cores, per-system argv templates, per-system config UI | 2 wk | A manual Windows `.exe` runs via UMU with a fresh prefix; PS2 and Switch launch through their cores |
| **M6** | Artwork pipeline, box/spine/label, manual override, diagnostics bundle, Sentry behind a flag | 2 wk | 100 Steam games fully dressed with no API key; spine art synthesised for every system |

**≈ 12.5 weeks** for one focused developer.

### The fixture that makes M3 trustworthy

`crates/core/tests/fixtures/testgame/` — a ~40-line binary that spawns a
grandchild, prints to stdout, sleeps for `TESTGAME_SLEEP`, and exits with
`TESTGAME_EXIT_CODE`. Every tracker test uses it. It is the highest-leverage
code in the project.

Tracker acceptance suite (both OSes):

1. Game spawns a grandchild that outlives it ⇒ `game-exited` fires only when the
   grandchild is gone.
2. Launcher killed mid-session ⇒ on Windows the tree dies; on Linux the session
   closes as `orphaned` at `last_heartbeat`.
3. Stop button terminates a three-level tree in < 6 s.
4. A grandchild that calls `setsid()` is detected and killed.
5. Ten launch/exit cycles leave zero zombies.

---

## 12. Open items

| Item | Status |
|---|---|
| Final name (avoid "shelf" — the concept is fine, the word invites comparison) | 5 candidates pending |
| Day-one systems: PS2, GameCube, SNES, GBA, Switch — one per media kind | proposed |
| Spine titles capped at 22 chars, single line | proposed |
| Epic on Linux: read Heroic's config only, do not bundle `legendary` | proposed |
| Steam Deck Gaming Mode | post-M3 |
| `theme.mode` default: `auto` | proposed |

---

## Appendix: what this is not

Boxshelf (GPL-3.0) is a direct inspiration for the *interaction concept* — a
physical shelf of cases. No code, CSS, shaders, or assets from it are used here;
see ADR-0002. The per-system case dimensions are real-world physical
measurements. Everything visible — palette, typography, lighting, materials,
motion, layout, sound — is original to this project.
