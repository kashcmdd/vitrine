import {
  type CaseInput,
  type CaseSlot,
  type Extent,
  type FocusState,
  type ShelfScene,
  type ShelfView,
  caseMetrics,
  rotationFor,
} from '../scene';

export interface Rect {
  width: number;
  height: number;
}

export interface LayoutOptions {
  /** Height of a facing case in CSS px. Everything else derives from it. */
  frontHeight: number;
  gapX: number;
  gapY: number;
  /** Visible thickness of the shelf board under each row. */
  shelfThickness: number;
  /** Inset from the left edge of the viewport. */
  insetX: number;
}

export interface LayoutResult {
  slots: CaseSlot[];
  extent: Extent;
}

export interface LayoutInput {
  cases: readonly CaseInput[];
  viewport: Rect;
  options: LayoutOptions;
  focusOf: (gameId: string) => FocusState;
}

export interface ShelfLayout {
  readonly name: 'spineRow' | 'frontGrid';
  measure(input: LayoutInput): LayoutResult;
}

export const DEFAULT_LAYOUT_OPTIONS: LayoutOptions = {
  frontHeight: 260,
  gapX: 18,
  gapY: 22,
  shelfThickness: 6,
  insetX: 32,
};

/**
 * Packs `items` into rows. Each item reports its own width; `gapX` is applied
 * between items, which the spine layout sets to 0 so cases touch like books.
 */
function packRows(
  widths: readonly number[],
  maxWidth: number,
  gapX: number,
): number[][] {
  const rows: number[][] = [];
  let current: number[] = [];
  let cursor = 0;

  widths.forEach((width, index) => {
    const advance = current.length === 0 ? width : width + gapX;
    if (current.length > 0 && cursor + advance > maxWidth) {
      rows.push(current);
      current = [index];
      cursor = width;
      return;
    }
    current.push(index);
    cursor += advance;
  });

  if (current.length > 0) rows.push(current);
  return rows;
}

function offsetFor(row: number, options: LayoutOptions): number {
  return row * (options.frontHeight + options.gapY + options.shelfThickness);
}

export function createLayout(name: ShelfLayout['name']): ShelfLayout {
  return name === 'spineRow' ? spineRow : frontGrid;
}

/** Cases stand like books: continuous packing, no gaps, spines to camera. */
export const spineRow: ShelfLayout = {
  name: 'spineRow',
  measure({ cases, viewport, options, focusOf }) {
    const widths = cases.map((c) => caseMetrics(c, options.frontHeight).spineWidth);
    const available = Math.max(1, viewport.width - options.insetX * 2);
    const rows = packRows(widths, available, 0);

    const slots: CaseSlot[] = [];
    rows.forEach((row, rowIndex) => {
      let x = options.insetX;
      const y = offsetFor(rowIndex, options);
      for (const index of row) {
        const item = cases[index];
        if (!item) continue;
        const { spineWidth } = caseMetrics(item, options.frontHeight);
        slots.push({
          gameId: item.gameId,
          index,
          position: [x, y, 0],
          rotationY: rotationFor('spine'),
          scale: 1,
          aspect: item.aspect,
          depth: item.depth,
          faces: item.faces,
          accent: item.accent,
          title: item.title,
          ...(item.system ? { system: item.system } : {}),
          focus: focusOf(item.gameId),
        });
        x += spineWidth;
      }
    });

    const rowsUsed = Math.max(1, rows.length);
    return {
      slots,
      extent: {
        width: viewport.width,
        height: offsetFor(rowsUsed - 1, options) + options.frontHeight + options.shelfThickness,
        shelves: rowsUsed,
      },
    };
  },
};

/** Faces forward: a gapped grid with a perspective camera. */
export const frontGrid: ShelfLayout = {
  name: 'frontGrid',
  measure({ cases, viewport, options, focusOf }) {
    const widths = cases.map((c) => caseMetrics(c, options.frontHeight).frontWidth);
    const available = Math.max(1, viewport.width - options.insetX * 2);
    const rows = packRows(widths, available, options.gapX);

    const slots: CaseSlot[] = [];
    rows.forEach((row, rowIndex) => {
      let x = options.insetX;
      const y = offsetFor(rowIndex, options);
      for (const index of row) {
        const item = cases[index];
        if (!item) continue;
        const { frontWidth } = caseMetrics(item, options.frontHeight);
        slots.push({
          gameId: item.gameId,
          index,
          position: [x, y, 0],
          rotationY: rotationFor('front'),
          scale: 1,
          aspect: item.aspect,
          depth: item.depth,
          faces: item.faces,
          accent: item.accent,
          title: item.title,
          ...(item.system ? { system: item.system } : {}),
          focus: focusOf(item.gameId),
        });
        x += frontWidth + options.gapX;
      }
    });

    const rowsUsed = Math.max(1, rows.length);
    return {
      slots,
      extent: {
        width: viewport.width,
        height: offsetFor(rowsUsed - 1, options) + options.frontHeight + options.shelfThickness,
        shelves: rowsUsed,
      },
    };
  },
};

export interface BuildSceneInput extends Omit<LayoutInput, 'focusOf'> {
  view: ShelfView;
  lighting: ShelfScene['lighting'];
  focusOf: (gameId: string) => FocusState;
}

/** Single entry point: cases + view + viewport -> a renderable scene. */
export function buildScene(input: BuildSceneInput): ShelfScene {
  const layout = createLayout(input.view === 'spine' ? 'spineRow' : 'frontGrid');
  const { slots, extent } = layout.measure(input);

  return {
    view: input.view,
    slots,
    lighting: input.lighting,
    extent,
    camera: {
      perspective: 2400,
      fov: 32,
      position: [extent.width / 2, extent.height / 2, 1600],
      target: [extent.width / 2, extent.height / 2, 0],
    },
  };
}
