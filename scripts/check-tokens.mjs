#!/usr/bin/env node
/**
 * Enforces the token rule from docs/blueprint.md §9:
 * no colour literal may appear in UI code outside `src/styles/tokens.css`.
 *
 * Scope is deliberate. It checks components, not data: a `systems.accent` value
 * coming from the database is content, not design. Placeholder mock data is
 * excluded because it is deleted in M1.
 *
 * Usage: node scripts/check-tokens.mjs
 */

import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

/** Where the rule applies: anything that renders. */
const UI_DIRS = ['src/app', 'src/shelf', 'src/components', 'src/features'];

/** Files that legitimately own colour values. */
const ALLOWED = new Set(['src/styles/tokens.css']);

const COLOUR = /(?<![&a-zA-Z0-9_-])#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b/g;

function walk(dir) {
  let out = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) out = out.concat(walk(full));
    else if (/\.(ts|tsx|css)$/.test(entry)) out.push(full);
  }
  return out;
}

const violations = [];

for (const dir of UI_DIRS) {
  let files;
  try {
    files = walk(join(root, dir));
  } catch {
    continue; // directory not created yet
  }

  for (const file of files) {
    const rel = relative(root, file).split('\\').join('/');
    if (ALLOWED.has(rel)) continue;

    const source = readFileSync(file, 'utf8');
    source.split('\n').forEach((line, index) => {
      // Ignore comments: documentation may name a colour.
      const code = line.replace(/\/\/.*$/, '').replace(/\/\*.*?\*\//g, '');
      for (const match of code.matchAll(COLOUR)) {
        violations.push({ file: rel, line: index + 1, value: match[0] });
      }
    });
  }
}

if (violations.length > 0) {
  console.error('colour literals found outside the token file:\n');
  for (const v of violations) {
    console.error(`  ${v.file}:${v.line}  ${v.value}`);
  }
  console.error(
    '\nUse a semantic token (bg-surface-1, text-ink-500, border-brass) or add a token to src/styles/tokens.css.',
  );
  process.exit(1);
}

console.log('token check passed: no colour literals outside src/styles/tokens.css');
