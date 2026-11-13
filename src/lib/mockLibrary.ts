import type { CaseInput } from '@/shelf/scene';

/**
 * Placeholder library so the shelf can be built and reviewed before any backend
 * exists. Deleted in M1 when `list_games` lands.
 */

export const SYSTEMS = {
  ps2: { id: 'ps2', label: 'PS2', aspect: 0.7, accent: '#2F4858' },
  gamecube: { id: 'gamecube', label: 'GCN', aspect: 0.95, accent: '#3D3A6B' },
  snes: { id: 'snes', label: 'SNES', aspect: 1.0, accent: '#6B4A3A' },
  gba: { id: 'gba', label: 'GBA', aspect: 1.0, accent: '#4A5B3A' },
  switch: { id: 'switch', label: 'NSW', aspect: 0.62, accent: '#7A2E2E' },
  pc: { id: 'pc', label: 'PC', aspect: 0.7, accent: '#4B423A' },
} as const;

const TITLES: ReadonlyArray<[keyof typeof SYSTEMS, string]> = [
  ['ps2', 'Silent Hill 2'],
  ['pc', 'Disco Elysium'],
  ['gamecube', 'Metroid Prime'],
  ['snes', 'Chrono Trigger'],
  ['switch', 'Hollow Knight'],
  ['gba', 'Golden Sun'],
  ['ps2', 'Shadow of the Colossus'],
  ['pc', 'Hades'],
  ['gamecube', 'F-Zero GX'],
  ['snes', 'Super Metroid'],
  ['switch', 'Outer Wilds'],
  ['gba', 'Advance Wars'],
  ['ps2', 'Katamari Damacy'],
  ['pc', 'Signalis'],
  ['gamecube', 'Paper Mario'],
  ['snes', 'Earthbound'],
  ['switch', 'Celeste'],
  ['gba', 'Fire Emblem'],
  ['ps2', 'Ico'],
  ['pc', 'Pentiment'],
  ['gamecube', 'Wind Waker'],
  ['snes', 'Secret of Mana'],
  ['switch', 'Tunic'],
  ['gba', 'Mario Kart'],
];

export const MOCK_LIBRARY: CaseInput[] = TITLES.map(([systemKey, title], index) => {
  const system = SYSTEMS[systemKey];
  return {
    gameId: `mock-${index}`,
    title,
    system: system.id,
    aspect: system.aspect,
    depth: systemKey === 'gamecube' ? 0.16 : 0.14,
    accent: system.accent,
    faces: {},
  };
});
