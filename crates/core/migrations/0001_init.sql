-- Vitrine initial schema.
--
-- Conventions:
--   * STRICT tables (SQLite >= 3.37) — a wrong type is an error, not a surprise.
--   * TEXT primary keys holding UUIDv7 — time-sortable, so index locality is
--     good and rows sort by creation without a second column.
--   * Timestamps are UTC ISO-8601 with a trailing Z, as TEXT.
--   * Enums are TEXT + CHECK, not integer codes, so a database is readable.
--   * Structured values are JSON in TEXT, validated in Rust before insert.
--
-- Connection PRAGMAs (journal_mode=WAL, synchronous=NORMAL, foreign_keys=ON,
-- busy_timeout=5000) are applied per connection in db/pool.rs, not here:
-- journal_mode is persistent but foreign_keys is per-connection and would be a
-- silent bug if it were only set once in a migration.

-- ---------------------------------------------------------------------------
-- systems: console platforms, and the geometry of their retail packaging.
-- ---------------------------------------------------------------------------

CREATE TABLE systems (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    short_name      TEXT NOT NULL,
    manufacturer    TEXT,
    generation      INTEGER,

    -- Physical case proportions. These are real measurements of retail
    -- packaging and they drive both the 3D case and the artwork fetch aspect,
    -- so both must read the same numbers.
    --   CD jewel case  ~0.95
    --   DVD / Blu-ray  ~0.70
    --   Switch case    ~0.62
    --   Game Boy cart  ~1.00
    box_aspect      REAL NOT NULL DEFAULT 0.7 CHECK (box_aspect > 0),
    box_depth       REAL NOT NULL DEFAULT 0.14 CHECK (box_depth > 0),
    media_kind      TEXT NOT NULL DEFAULT 'dvd'
                    CHECK (media_kind IN ('cartridge', 'disc', 'jewel', 'bluray', 'dvd', 'card', 'download')),

    -- Spine colour, so a shelf reads as grouped by system.
    accent          TEXT NOT NULL DEFAULT '#6B6357',

    retroarch_core  TEXT,
    sort_index      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

-- ---------------------------------------------------------------------------
-- runners: compatibility layers AND emulators (ADR-0004).
-- ---------------------------------------------------------------------------

CREATE TABLE runners (
    id                TEXT PRIMARY KEY,
    name              TEXT NOT NULL,
    kind              TEXT NOT NULL
                      CHECK (kind IN ('native', 'wine', 'proton', 'umu', 'emulator', 'custom')),

    -- wine binary / proton directory / umu-run / retroarch
    executable_path   TEXT,
    prefix_path       TEXT,
    umu_game_id       TEXT,
    -- RetroArch core (.so/.dll) or a standalone emulator's core argument
    core_path         TEXT,
    -- org.libretro.RetroArch, etc.
    flatpak_id        TEXT,

    default_env       TEXT NOT NULL DEFAULT '{}',
    is_auto_detected  INTEGER NOT NULL DEFAULT 0 CHECK (is_auto_detected IN (0, 1)),
    is_default        INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

-- A partial unique index rather than a UNIQUE constraint, because NULL
-- prefix_path is meaningful (native and emulator runners have none) and SQLite
-- treats NULLs as distinct in a UNIQUE constraint anyway.
CREATE UNIQUE INDEX idx_runners_identity
    ON runners (kind, executable_path, COALESCE(prefix_path, ''));

-- Which runner handles which system, and with what argv.
CREATE TABLE emulator_systems (
    runner_id     TEXT NOT NULL REFERENCES runners(id) ON DELETE CASCADE,
    system_id     TEXT NOT NULL REFERENCES systems(id) ON DELETE CASCADE,
    -- JSON array, tokenised: ["-L", "{core}", "{rom}"]
    args_template TEXT NOT NULL DEFAULT '[]',
    env_json      TEXT NOT NULL DEFAULT '{}',
    is_default    INTEGER NOT NULL DEFAULT 1 CHECK (is_default IN (0, 1)),
    -- Some front-ends quit to their own menu rather than exiting; the session
    -- supervisor honours this when deciding whether the game is over.
    quit_when     TEXT NOT NULL DEFAULT 'process_exit'
                  CHECK (quit_when IN ('process_exit', 'emulator_exit')),
    PRIMARY KEY (runner_id, system_id)
) STRICT;

-- ---------------------------------------------------------------------------
-- games: PC titles (system_id NULL) and emulated titles (system_id set).
-- ---------------------------------------------------------------------------

CREATE TABLE games (
    id                TEXT PRIMARY KEY,
    title             TEXT NOT NULL,
    sort_title        TEXT NOT NULL,
    description       TEXT,

    system_id         TEXT REFERENCES systems(id) ON DELETE SET NULL,

    -- Launch target. NULL for an emulated title, whose target is in rom_files.
    executable_path   TEXT,
    working_directory TEXT,
    arguments         TEXT NOT NULL DEFAULT '[]',
    env_vars          TEXT NOT NULL DEFAULT '{}',
    target_platform   TEXT NOT NULL DEFAULT 'native'
                      CHECK (target_platform IN ('native', 'windows')),

    source            TEXT NOT NULL DEFAULT 'manual'
                      CHECK (source IN ('manual', 'steam', 'epic', 'gog', 'lutris', 'heroic', 'local_scan')),
    source_app_id     TEXT,
    source_url        TEXT,
    install_dir       TEXT,
    runner_id         TEXT REFERENCES runners(id) ON DELETE SET NULL,

    -- Denormalised cache. Recomputed on session close and again at boot by
    -- service::playtime::recompute_all(), so drift is self-healing.
    playtime_seconds  INTEGER NOT NULL DEFAULT 0 CHECK (playtime_seconds >= 0),
    last_played_at    TEXT,
    launch_count      INTEGER NOT NULL DEFAULT 0 CHECK (launch_count >= 0),

    is_favorite       INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1)),
    is_hidden         INTEGER NOT NULL DEFAULT 0 CHECK (is_hidden IN (0, 1)),

    -- Spine synthesis (ADR-0005): the shelf must be usable with no network,
    -- so every game has a spine colour and knows where its art came from.
    spine_accent      TEXT,
    spine_art         TEXT CHECK (spine_art IN ('scraped', 'generated', 'user')),

    user_sort_index   INTEGER,
    added_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    -- Survives manual games because SQLite treats NULLs as distinct.
    UNIQUE (source, source_app_id)
) STRICT;

CREATE INDEX idx_games_sort ON games (sort_title);
CREATE INDEX idx_games_last ON games (last_played_at DESC);
CREATE INDEX idx_games_system ON games (system_id);
CREATE UNIQUE INDEX idx_games_exe
    ON games (source, LOWER(executable_path))
    WHERE executable_path IS NOT NULL;

-- ---------------------------------------------------------------------------
-- launch_configs: a game may have several presets; one is default.
-- ---------------------------------------------------------------------------

CREATE TABLE launch_configs (
    id                 TEXT PRIMARY KEY,
    game_id            TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    label              TEXT NOT NULL DEFAULT 'Default',

    executable_override TEXT,
    arguments          TEXT NOT NULL DEFAULT '[]',
    env_override       TEXT NOT NULL DEFAULT '{}',
    runner_id          TEXT REFERENCES runners(id) ON DELETE SET NULL,

    -- Wrapper toggles are columns rather than a join table because the set is
    -- fixed and known to the UI. extra_wrappers is the escape hatch.
    use_gamemode       INTEGER NOT NULL DEFAULT 0 CHECK (use_gamemode IN (0, 1)),
    use_mangohud       INTEGER NOT NULL DEFAULT 0 CHECK (use_mangohud IN (0, 1)),
    use_gamescope      INTEGER NOT NULL DEFAULT 0 CHECK (use_gamescope IN (0, 1)),
    gamescope_args     TEXT,
    extra_wrappers     TEXT NOT NULL DEFAULT '[]',

    proton_options     TEXT NOT NULL DEFAULT '{}',

    is_default         INTEGER NOT NULL DEFAULT 1 CHECK (is_default IN (0, 1)),
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (game_id, label)
) STRICT;

CREATE INDEX idx_launch_configs_game ON launch_configs (game_id);

-- ---------------------------------------------------------------------------
-- play_sessions: the only place playtime is recorded.
-- ---------------------------------------------------------------------------

CREATE TABLE play_sessions (
    id               TEXT PRIMARY KEY,
    game_id          TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    launch_config_id TEXT REFERENCES launch_configs(id) ON DELETE SET NULL,

    started_at       TEXT NOT NULL,
    ended_at         TEXT,
    -- Written every 20s while running. If the launcher is killed outright, this
    -- is how much of the session is recoverable.
    last_heartbeat   TEXT NOT NULL,

    -- Stored, so the UI and the repair job cannot disagree about a session.
    duration_seconds INTEGER GENERATED ALWAYS AS (
        CASE
            WHEN ended_at IS NULL THEN NULL
            ELSE CAST((julianday(ended_at) - julianday(started_at)) * 86400 AS INTEGER)
        END
    ) STORED,

    exit_code        INTEGER,
    exit_signal      INTEGER,
    status           TEXT NOT NULL
                     CHECK (status IN ('running', 'exited', 'crashed', 'killed', 'orphaned')),

    launcher_pid     INTEGER,
    root_pid         INTEGER,
    log_path         TEXT,
    error_message    TEXT
) STRICT;

CREATE INDEX idx_sessions_game ON play_sessions (game_id, started_at DESC);

-- One live session per game, enforced by the database rather than by a race in
-- Rust. A double-click cannot start a second session.
CREATE UNIQUE INDEX idx_sessions_one_running
    ON play_sessions (game_id)
    WHERE status = 'running';

-- ---------------------------------------------------------------------------
-- rom_files: multi-disc sets, cue/bin, chd, m3u playlists.
-- ---------------------------------------------------------------------------

CREATE TABLE rom_files (
    id         TEXT PRIMARY KEY,
    game_id    TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    path       TEXT NOT NULL,
    role       TEXT NOT NULL DEFAULT 'disc'
               CHECK (role IN ('disc', 'track', 'patch', 'bios', 'other')),
    disc_index INTEGER,
    size_bytes INTEGER,
    -- An .m3u that supersedes this file, if any.
    playlist   TEXT,
    UNIQUE (game_id, path)
) STRICT;

CREATE UNIQUE INDEX idx_rom_files_path ON rom_files (path);

-- ---------------------------------------------------------------------------
-- artworks: many candidates per kind, at most one selected.
-- ---------------------------------------------------------------------------

CREATE TABLE artworks (
    id          TEXT PRIMARY KEY,
    game_id     TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL
                CHECK (kind IN ('front', 'back', 'spine', 'label', 'cart', 'manual', 'hero', 'logo', 'icon')),
    blob_hash   TEXT,
    local_path  TEXT,
    remote_url  TEXT,
    source      TEXT NOT NULL,
    width       INTEGER,
    height      INTEGER,
    is_selected INTEGER NOT NULL DEFAULT 0 CHECK (is_selected IN (0, 1)),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE INDEX idx_artworks_game_kind ON artworks (game_id, kind);
CREATE UNIQUE INDEX idx_artworks_selected
    ON artworks (game_id, kind)
    WHERE is_selected = 1;

-- ---------------------------------------------------------------------------
-- game_metadata: cached provider payloads, with retry state.
-- ---------------------------------------------------------------------------

CREATE TABLE game_metadata (
    game_id     TEXT PRIMARY KEY REFERENCES games(id) ON DELETE CASCADE,
    provider    TEXT NOT NULL,
    provider_id TEXT,
    payload     TEXT NOT NULL,
    fetched_at  TEXT NOT NULL,
    state       TEXT NOT NULL DEFAULT 'ready'
                CHECK (state IN ('pending', 'ready', 'failed', 'stale')),
    retry_at    TEXT,
    attempts    INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0)
) STRICT;

-- ---------------------------------------------------------------------------
-- discovered_candidates: a staging area. Scanners never write to games.
-- ---------------------------------------------------------------------------

CREATE TABLE discovered_candidates (
    id                TEXT PRIMARY KEY,
    scanner           TEXT NOT NULL,
    external_id       TEXT NOT NULL,
    title             TEXT NOT NULL,
    install_dir       TEXT,
    executable        TEXT,
    payload           TEXT NOT NULL DEFAULT '{}',

    suggested_game_id TEXT REFERENCES games(id) ON DELETE SET NULL,
    match_score       REAL,

    state             TEXT NOT NULL DEFAULT 'new'
                      CHECK (state IN ('new', 'approved', 'rejected', 'ignored')),
    discovered_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (scanner, external_id)
) STRICT;

CREATE INDEX idx_candidates_state ON discovered_candidates (state);

-- ---------------------------------------------------------------------------
-- key/value.
-- ---------------------------------------------------------------------------

CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE schema_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

-- ---------------------------------------------------------------------------
-- Seed: systems.
--
-- Five systems ship at M1, one per media kind, which is enough to exercise every
-- code path. The rest are data, not code — adding one is an INSERT.
-- ---------------------------------------------------------------------------

INSERT INTO systems (id, name, short_name, manufacturer, generation, box_aspect, box_depth, media_kind, accent, retroarch_core, sort_index) VALUES
    ('ps2',       'PlayStation 2',      'PS2',    'Sony',    6, 0.70, 0.14, 'dvd',      '#2F4858', 'pcsx2',          10),
    ('gamecube',  'GameCube',           'GCN',    'Nintendo', 6, 0.95, 0.16, 'jewel',    '#3D3A6B', 'dolphin',        20),
    ('snes',      'Super Nintendo',     'SNES',   'Nintendo', 4, 1.00, 0.18, 'cartridge','#6B4A3A', 'snes9x',         30),
    ('gba',       'Game Boy Advance',   'GBA',    'Nintendo', 6, 1.00, 0.12, 'cartridge','#4A5B3A', 'mgba',           40),
    ('switch',    'Nintendo Switch',    'NSW',    'Nintendo', 9, 0.62, 0.12, 'card',     '#7A2E2E', NULL,             50),
    -- Secondary systems: present as data so the UI and importers can be built
    -- against them, without promising launch support yet.
    ('ps1',       'PlayStation',        'PS1',    'Sony',    5, 0.95, 0.14, 'jewel',    '#4A4A55', 'pcsx_rearmed',   60),
    ('n64',       'Nintendo 64',        'N64',    'Nintendo', 5, 1.05, 0.22, 'cartridge','#7A6A3A', 'mupen64plus_next', 70),
    ('nes',       'Nintendo Entertainment System', 'NES', 'Nintendo', 3, 1.15, 0.16, 'cartridge', '#6B6B6B', 'mesen',   80),
    ('genesis',   'Mega Drive / Genesis','MD',    'Sega',    4, 1.10, 0.18, 'cartridge','#2E3E52', 'genesis_plus_gx', 90),
    ('dreamcast', 'Dreamcast',          'DC',     'Sega',    6, 0.95, 0.14, 'jewel',    '#5A3A2E', 'flycast',       100),
    ('gb',        'Game Boy',           'GB',     'Nintendo', 4, 1.00, 0.10, 'cartridge','#5E6B4A', 'gambatte',      110),
    ('gba_v2',    'Game Boy Color',     'GBC',    'Nintendo', 5, 1.00, 0.10, 'cartridge','#7A5E3A', 'gambatte',      120),
    ('psp',       'PlayStation Portable','PSP',   'Sony',    7, 0.95, 0.14, 'jewel',    '#3A4A5E', 'ppsspp',        130),
    ('nds',       'Nintendo DS',        'NDS',    'Nintendo', 7, 1.02, 0.10, 'card',     '#4A4A6B', 'melonds',       140),
    ('wii',       'Wii',                'Wii',    'Nintendo', 7, 0.70, 0.14, 'dvd',      '#5E6B7A', 'dolphin',       150),
    ('saturn',    'Sega Saturn',        'SAT',    'Sega',    5, 0.95, 0.14, 'jewel',    '#5A4A5E', 'mednafen_saturn', 160);

INSERT INTO schema_meta (key, value) VALUES ('blueprint_version', '2');
