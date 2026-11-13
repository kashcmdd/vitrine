# ADR-0004: Emulators are runners, not a parallel launch system

**Status:** Accepted · 2026-11-13

## Context

The product covers PC games (Steam, Epic, GOG, Lutris, manual) and emulated
titles (PS2, GameCube, SNES, GBA, Switch, and more). The obvious implementation
gives emulation its own subsystem: separate tables, a separate launch path, a
separate playtime path, a separate config UI.

That is how front-ends become unmaintainable. Every feature then has to be built
twice — logging, error surfacing, kill semantics, session history, wrapper
application — and the two copies drift.

The launch model already treats a game as *a target executed through a
compatibility layer*. An emulator is exactly that: a target (a ROM) executed
through a compatibility layer (RetroArch with a core, or a standalone emulator).

## Decision

Emulators are rows in `runners` with `kind = 'emulator'`, and console systems are
first-class rows in `systems`.

- `games.system_id IS NULL` ⇒ PC game.
- `games.system_id NOT NULL` ⇒ emulated title.
- Which runner handles which system, and with what argv, lives in
  `emulator_systems` (`args_template`, `env_json`, `is_default`, `quit_when`).
- `rom_files` models multi-disc sets; an `.m3u` playlist supersedes its
  constituent discs.
- `systems` carries physical case geometry (`box_aspect`, `box_depth`,
  `media_kind`) which drives both the 3D case and the artwork fetch aspect.

`Target` becomes an enum:

```rust
pub enum Target {
    Process { exe: PathBuf },
    Rom { system: SystemId, file: PathBuf, runner: RunnerId },
}
```

The tracker (ADR-0003) does not care which variant it receives.

## Consequences

**Positive**

- One launch path, one session table, one kill path, one log capture, one
  playtime accounting for both worlds.
- Adding a console is data, not code: a `systems` row, an `emulator_systems` row,
  an accent colour, and a case aspect ratio. Adding the twentieth system costs
  the same as the fifth.
- Playtime for RetroArch titles works on day one, because containment tracking
  was never game-specific.

**Negative**

- `systems` becomes load-bearing, so a bad `box_aspect` distorts every case for
  that console. Values are curated and reviewed, not inferred.
- ROM identification is genuinely messy (multi-disc, `.cue`/`.bin`, `.chd`,
  regional variants, hacks). Handled in `discovery/roms.rs` with explicit
  approval rather than heuristics alone.
- RetroArch can be configured to quit to its own menu rather than exit; the
  `quit_when` column exists for exactly this and must be honoured by the session
  supervisor.

## Explicitly out of scope

Bundling, downloading, or linking ROMs, BIOS/firmware, or keys. The app detects
what is missing and explains how to supply it from the user's own hardware. This
is both a legal and a reputational requirement, and it is stated in the README.
