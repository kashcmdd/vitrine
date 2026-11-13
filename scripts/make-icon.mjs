#!/usr/bin/env node
/**
 * Generates the source app icon as a PNG, with no image dependencies.
 *
 * Design: the shelf, in miniature — an ink field, a shelf board, and five spines
 * in the system accents with one brass spine standing proud. It has to read at
 * 32 px, so the motif is four shapes and no text.
 *
 * Usage: node scripts/make-icon.mjs [outfile] [size]
 */

import { deflateSync } from 'node:zlib';
import { writeFileSync } from 'node:fs';

const size = Number(process.argv[3] ?? 1024);
const outfile = process.argv[2] ?? 'src-tauri/icons/source.png';

const INK = [0x14, 0x12, 0x0f];
const BRASS = [0xb4, 0x76, 0x3a];
const BOARD = [0x3a, 0x35, 0x2d];
const PAPER = [0xf6, 0xf1, 0xe8];

// Spine colours, echoing the systems seeded in the schema.
const SPINES = [
  { w: 0.062, h: 0.42, c: [0x2f, 0x48, 0x58] },
  { w: 0.062, h: 0.50, c: [0x3d, 0x3a, 0x6b] },
  { w: 0.070, h: 0.56, c: BRASS },
  { w: 0.062, h: 0.46, c: [0x4a, 0x5b, 0x3a] },
  { w: 0.062, h: 0.38, c: [0x7a, 0x2e, 0x2e] },
];

const px = new Uint8Array(size * size * 4);
const radius = size * 0.22;
const baseline = size * 0.74;
const boardHeight = Math.max(2, size * 0.022);
const shelfLeft = size * 0.18;
const shelfRight = size * 0.82;

function insideRoundedRect(x, y, r) {
  const cx = Math.min(Math.max(x, r), size - r);
  const cy = Math.min(Math.max(y, r), size - r);
  const dx = x - cx;
  const dy = y - cy;
  return dx * dx + dy * dy <= r * r + 0.5;
}

function put(i, [r, g, b], alpha = 255) {
  px[i] = r;
  px[i + 1] = g;
  px[i + 2] = b;
  px[i + 3] = alpha;
}

// Collect spine rectangles first so we can test membership per pixel.
let cursor = shelfLeft;
const rects = SPINES.map((spine) => {
  const w = spine.w * size;
  const h = spine.h * size;
  const rect = { x0: cursor, x1: cursor + w, y0: baseline - h, y1: baseline, c: spine.c };
  cursor += w;
  return rect;
});
// Centre the run of spines on the shelf.
const shift = (size - cursor) / 2 + shelfLeft / 2 - shelfLeft / 2;
const centred = rects.map((r) => ({ ...r, x0: r.x0 + shift, x1: r.x1 + shift }));

const board = { x0: shelfLeft, x1: shelfRight, y0: baseline, y1: baseline + boardHeight };

for (let y = 0; y < size; y++) {
  for (let x = 0; x < size; x++) {
    const i = (y * size + x) * 4;
    if (!insideRoundedRect(x, y, radius)) {
      put(i, [0, 0, 0], 0);
      continue;
    }

    let colour = INK;

    for (const r of centred) {
      if (x >= r.x0 && x < r.x1 && y >= r.y0 && y < r.y1) {
        colour = r.c;
        // A thin highlight down the left edge of each spine, so overlapping
        // cases stay separable at small sizes.
        if (x - r.x0 < Math.max(1, size * 0.004)) {
          colour = colour.map((v) => Math.min(255, v + 26));
        }
        break;
      }
    }

    if (x >= board.x0 && x < board.x1 && y >= board.y0 && y < board.y1) {
      colour = BOARD;
    }

    // Reflection of the collection in the shelf's polish, fading out fast.
    if (y >= board.y1 && y < baseline + boardHeight * 4) {
      const depth = (y - board.y1) / (boardHeight * 4);
      for (const r of centred) {
        if (x >= r.x0 && x < r.x1) {
          colour = r.c.map((v) => v * (1 - depth) + INK[0] * depth).map(Math.round);
        }
      }
    }

    put(i, colour);
  }
}

// A one-pixel paper rule under the board, to give the icon a baseline at 32 px.
for (let x = shelfLeft; x < shelfRight; x++) {
  const y = Math.round(baseline + boardHeight);
  if (y < size) put((y * size + x) * 4, PAPER, 120);
}

// --- PNG encoding -----------------------------------------------------------

const raw = Buffer.alloc((size * 4 + 1) * size);
for (let y = 0; y < size; y++) {
  raw[y * (size * 4 + 1)] = 0; // filter: none
  Buffer.from(px.buffer, y * size * 4, size * 4).copy(raw, y * (size * 4 + 1) + 1);
}

const crcTable = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c;
  }
  return table;
})();

function crc32(buf) {
  let c = -1;
  for (const byte of buf) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ -1) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([length, body, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(size, 0);
ihdr.writeUInt32BE(size, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA
ihdr[10] = 0;
ihdr[11] = 0;
ihdr[12] = 0;

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);

writeFileSync(outfile, png);
console.log(`wrote ${outfile} (${size}x${size}, ${png.length} bytes)`);
