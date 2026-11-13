/**
 * The shelf scene model.
 *
 * Renderer-agnostic by design (ADR-0005): `spineRow` and `frontGrid` produce the
 * same `CaseSlot[]`, and either the Three.js engine or the CSS-3D engine renders
 * it. Nothing in this file may import a renderer, a React component, or the DOM.
 */

export type GameId = string;
export type SystemId = string;

/** Spine-first or faces-forward. Implemented as a rotation of one scene. */
export type ShelfView = 'spine' | 'front';

export type FocusState = 'none' | 'hover' | 'selected';

export type LightingMode = 'day' | 'room';

/** A case seen end-on shows its spine at this rotation. */
export const SPINE_ROTATION_Y = 90;
export const FRONT_ROTATION_Y = 0;

export interface ArtRef {
  /** Absolute path under the app media directory. */
  path: string;
  width: number;
  height: number;
}

export interface CaseFaces {
  front?: ArtRef;
  spine?: ArtRef;
  back?: ArtRef;
  label?: ArtRef;
}

/** The irreducible facts a layout needs about a game. */
export interface CaseInput {
  gameId: GameId;
  title: string;
  system?: SystemId;
  /** Physical case width / height, from `systems.box_aspect`. */
  aspect: number;
  /** Physical case thickness / height, from `systems.box_depth`. */
  depth: number;
  /** Per-system spine colour, so a shelf reads as grouped. */
  accent: string;
  faces: CaseFaces;
}

export interface CaseSlot {
  gameId: GameId;
  index: number;
  /** Screen-space origin of the case, in CSS px at 1x zoom. */
  position: readonly [number, number, number];
  rotationY: number;
  scale: number;
  aspect: number;
  depth: number;
  faces: CaseFaces;
  accent: string;
  title: string;
  system?: SystemId;
  focus: FocusState;
}

export interface CameraSpec {
  /** CSS `perspective` value in px. */
  perspective: number;
  /** Vertical field of view in degrees. Unused by the CSS-3D engine. */
  fov: number;
  position: readonly [number, number, number];
  target: readonly [number, number, number];
}

export interface Extent {
  width: number;
  height: number;
  /** Number of physical shelves the collection occupies. */
  shelves: number;
}

export interface ShelfScene {
  view: ShelfView;
  slots: CaseSlot[];
  camera: CameraSpec;
  lighting: LightingMode;
  extent: Extent;
}

/** Rotation for a view, in degrees. */
export function rotationFor(view: ShelfView): number {
  return view === 'spine' ? SPINE_ROTATION_Y : FRONT_ROTATION_Y;
}

/**
 * Case geometry in CSS px for a front height of `frontHeight`.
 *
 * `depth` is thickness / height, so the spine width of a case is
 * `frontHeight * depth` and its front width is `frontHeight * aspect`.
 */
export function caseMetrics(
  input: Pick<CaseInput, 'aspect' | 'depth'>,
  frontHeight: number,
): { frontWidth: number; spineWidth: number; height: number } {
  return {
    frontWidth: frontHeight * input.aspect,
    spineWidth: Math.max(6, frontHeight * input.depth),
    height: frontHeight,
  };
}
