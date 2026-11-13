#!/usr/bin/env node
/**
 * Schema conformance check.
 *
 * Applies every migration to a throwaway database and then asserts the
 * invariants that the rest of the application depends on. This runs without a
 * Rust toolchain, so it is cheap enough to run on every commit and it catches
 * the failure mode that matters most: a migration that silently stops enforcing
 * something the Rust code assumes.
 *
 * Usage: node scripts/schema-check.mjs
 */

import { DatabaseSync } from 'node:sqlite';
import { readFileSync, readdirSync, rmSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const migrationsDir = join(root, 'crates/core/migrations');
const cacheDir = join(root, 'node_modules/.cache');
const dbPath = join(cacheDir, 'vitrine-schema-check.db');

let failures = 0;
const pass = (msg) => console.log(`  ok    ${msg}`);
const fail = (msg, detail) => {
  failures += 1;
  console.log(`  FAIL  ${msg}${detail ? ` -> ${detail}` : ''}`);
};

rmSync(dbPath, { force: true });
mkdirSync(cacheDir, { recursive: true });
const db = new DatabaseSync(dbPath);
db.exec('PRAGMA foreign_keys = ON;');

console.log('applying migrations');
for (const file of readdirSync(migrationsDir).filter((f) => f.endsWith('.sql')).sort()) {
  try {
    db.exec(readFileSync(join(migrationsDir, file), 'utf8'));
    pass(file);
  } catch (error) {
    fail(file, error.message);
  }
}

const run = (sql, ...args) => db.prepare(sql).run(...args);
const get = (sql, ...args) => db.prepare(sql).get(...args);
const all = (sql, ...args) => db.prepare(sql).all(...args);

/** Asserts that `fn` throws, i.e. the database refuses the write. */
function rejects(label, fn) {
  try {
    fn();
    fail(label, 'write was accepted');
  } catch {
    pass(`rejects: ${label}`);
  }
}

/** Asserts that `fn` returns a specific value. */
function equals(label, actual, expected) {
  if (actual === expected) pass(`${label} = ${actual}`);
  else fail(label, `expected ${expected}, got ${actual}`);
}

console.log('\nstructure');
const tables = all("select name from sqlite_master where type='table' order by name").map(
  (r) => r.name,
);
for (const expected of [
  'games',
  'systems',
  'runners',
  'emulator_systems',
  'launch_configs',
  'play_sessions',
  'rom_files',
  'artworks',
  'game_metadata',
  'discovered_candidates',
  'settings',
]) {
  if (tables.includes(expected)) pass(`table ${expected}`);
  else fail(`table ${expected}`, 'missing');
}

const seededSystems = get('select count(*) as c from systems').c;
if (seededSystems >= 5) pass(`systems seeded (${seededSystems})`);
else fail('systems seeded', `only ${seededSystems}`);

// Every system must be renderable: geometry drives the 3D case (ADR-0004).
const badGeometry = all(
  'select id from systems where box_aspect <= 0 or box_depth <= 0 or accent is null',
);
if (badGeometry.length === 0) pass('every system has usable box geometry');
else fail('system geometry', badGeometry.map((r) => r.id).join(', '));

console.log('\ninvariants');

run(
  `insert into games (id, title, sort_title, source, source_app_id, executable_path)
   values ('g1', 'Hades', 'hades', 'steam', '1145360', '/games/hades.exe')`,
);
run(`insert into games (id, title, sort_title, source) values ('g2', 'Manual', 'manual', 'manual')`);
pass('two manual games coexist despite NULL source_app_id');

rejects('duplicate (source, source_app_id)', () =>
  run(
    `insert into games (id, title, sort_title, source, source_app_id)
     values ('g3', 'Dup', 'dup', 'steam', '1145360')`,
  ),
);

rejects('STRICT rejects text in an integer column', () =>
  run(`insert into games (id, title, sort_title, playtime_seconds) values ('g4', 'T', 't', 'lots')`),
);

rejects('negative playtime', () =>
  run(`update games set playtime_seconds = -1 where id = 'g1'`),
);

rejects('unknown launch source', () =>
  run(
    `insert into games (id, title, sort_title, source) values ('g5', 'T', 't', 'battlenet')`,
  ),
);

// One live session per game: a double-click cannot create a second one.
run(
  `insert into play_sessions (id, game_id, started_at, last_heartbeat, status)
   values ('s1', 'g1', '2026-11-13T01:00:00Z', '2026-11-13T01:00:00Z', 'running')`,
);
rejects('second concurrent running session for one game', () =>
  run(
    `insert into play_sessions (id, game_id, started_at, last_heartbeat, status)
     values ('s2', 'g1', '2026-11-13T01:10:00Z', '2026-11-13T01:10:00Z', 'running')`,
  ),
);

run(`update play_sessions set ended_at = '2026-11-13T02:30:00Z', status = 'exited' where id = 's1'`);
equals(
  'generated duration_seconds (90 min)',
  get(`select duration_seconds as d from play_sessions where id = 's1'`).d,
  5400,
);

// Closing a session frees the slot for the next launch.
run(
  `insert into play_sessions (id, game_id, started_at, last_heartbeat, status)
   values ('s3', 'g1', '2026-11-13T03:00:00Z', '2026-11-13T03:00:00Z', 'running')`,
);
pass('a new session is allowed once the previous one closed');

rejects('session status typo', () =>
  run(
    `insert into play_sessions (id, game_id, started_at, last_heartbeat, status)
     values ('s4', 'g2', 'x', 'x', 'runing')`,
  ),
);

rejects('orphan session (foreign key)', () =>
  run(
    `insert into play_sessions (id, game_id, started_at, last_heartbeat, status)
     values ('s5', 'missing', 'x', 'x', 'exited')`,
  ),
);

run(`insert into artworks (id, game_id, kind, source, is_selected) values ('a1', 'g1', 'front', 'steam_cdn', 1)`);
pass('first artwork for a kind is selectable');
rejects('two selected artworks of the same kind', () =>
  run(`insert into artworks (id, game_id, kind, source, is_selected) values ('a2', 'g1', 'front', 'steamgriddb', 1)`),
);

// Emulators are runners (ADR-0004): same table, same session accounting.
run(
  `insert into runners (id, name, kind, executable_path) values ('r1', 'RetroArch', 'emulator', '/usr/bin/retroarch')`,
);
run(
  `insert into emulator_systems (runner_id, system_id, args_template)
   values ('r1', 'ps2', '["-L","{core}","{rom}"]')`,
);
pass('an emulator runner can own a system');
rejects('emulator_systems requires a real system', () =>
  run(
    `insert into emulator_systems (runner_id, system_id) values ('r1', 'nonexistent')`,
  ),
);

// Cascades: deleting a game must not leave sessions or roms behind.
run(
  `insert into rom_files (id, game_id, path, role) values ('f1', 'g2', '/roms/x.chd', 'disc')`,
);
run(`delete from games where id = 'g2'`);
equals(
  'deleting a game cascades to rom_files',
  get(`select count(*) as c from rom_files where game_id = 'g2'`).c,
  0,
);

db.close();
rmSync(dbPath, { force: true });

console.log('');
if (failures > 0) {
  console.error(`${failures} schema check(s) failed`);
  process.exit(1);
}
console.log('all schema checks passed');
