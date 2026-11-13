import { useMemo } from 'react';
import type { CaseSlot, ShelfScene } from '@/shelf/scene';
import { caseMetrics } from '@/shelf/scene';

/**
 * CSS-3D shelf engine.
 *
 * Deliberately the *fallback* engine (see `shelf/engine/index.ts`): it renders
 * each case as a single card that flips into place, rather than a true extruded
 * box. That keeps WebKitGTK's compositor within budget and gives us a rendering
 * path that works when WebGL does not exist at all.
 *
 * The Three.js engine (M2) renders the real geometry — front, spine, back and
 * label faces — from the same `ShelfScene`.
 */

export interface Css3dShelfProps {
  scene: ShelfScene;
  frontHeight: number;
  onSelect: (gameId: string) => void;
  onHover: (gameId: string | null) => void;
}

function caseStyle(slot: CaseSlot, frontHeight: number): React.CSSProperties {
  const { frontWidth, spineWidth, height } = caseMetrics(slot, frontHeight);
  const isSpine = slot.rotationY !== 0;
  const width = isSpine ? spineWidth : frontWidth;
  const [x, y] = slot.position;

  return {
    position: 'absolute',
    left: `${x}px`,
    top: `${y}px`,
    width: `${width}px`,
    height: `${height}px`,
    borderRadius: 'var(--radius-case)',
    // The flip: cases arrive edge-on and settle into place.
    transform: `perspective(1200px) rotateY(${isSpine ? -10 : 0}deg) scale(${
      slot.focus === 'none' ? 0.985 : slot.focus === 'hover' ? 1.03 : 1.06
    })`,
    transformOrigin: 'center bottom',
    opacity: slot.focus === 'none' ? 0.85 : 1,
    transition:
      'width 420ms var(--ease-shelf), transform 260ms var(--ease-shelf), opacity 260ms var(--ease-shelf), left 420ms var(--ease-shelf)',
    boxShadow:
      slot.focus === 'none'
        ? '0 2px 8px rgba(var(--shadow-warm) / 0.14)'
        : '0 12px 32px rgba(var(--shadow-warm) / 0.28)',
    outline: slot.focus === 'selected' ? '2px solid var(--brass)' : 'none',
    outlineOffset: '4px',
    cursor: 'pointer',
    overflow: 'hidden',
    display: 'flex',
    alignItems: 'flex-end',
    justifyContent: 'center',
  };
}

function Cover({ slot, frontHeight }: { slot: CaseSlot; frontHeight: number }) {
  const isSpine = slot.rotationY !== 0;

  if (slot.faces.front && !isSpine) {
    return (
      <img
        src={slot.faces.front.path}
        alt=""
        width={slot.faces.front.width}
        height={slot.faces.front.height}
        loading="lazy"
        decoding="async"
        style={{ width: '100%', height: '100%', objectFit: 'cover' }}
      />
    );
  }

  // Spine, or a case whose artwork has not been fetched yet. The generated
  // spine is the reason the shelf is usable with zero network access
  // (ADR-0005), so it must look intentional rather than like a placeholder.
  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        background: slot.accent,
        color: 'var(--on-accent)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '10px 0',
      }}
    >
      <span
        className="type-display"
        style={{
          writingMode: isSpine ? 'vertical-rl' : 'horizontal-tb',
          textOrientation: 'mixed',
          fontSize: isSpine ? '13px' : '18px',
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          maxHeight: `${Math.max(0, frontHeight - 20)}px`,
          padding: isSpine ? '0' : '0 12px',
        }}
      >
        {slot.title}
      </span>
    </div>
  );
}

export function Css3dShelf({ scene, frontHeight, onSelect, onHover }: Css3dShelfProps) {
  const shelfRows = useMemo(() => {
    const rows = new Set<number>();
    for (const slot of scene.slots) rows.add(slot.position[1]);
    return [...rows].sort((a, b) => a - b);
  }, [scene.slots]);

  return (
    <div
      style={{
        position: 'relative',
        width: `${scene.extent.width}px`,
        height: `${scene.extent.height}px`,
        perspective: `${scene.camera.perspective}px`,
        perspectiveOrigin: '50% 40%',
      }}
    >
      {shelfRows.map((y) => (
        <div
          key={y}
          aria-hidden
          style={{
            position: 'absolute',
            left: 0,
            top: `${y + frontHeight}px`,
            width: '100%',
            height: '6px',
            background: 'var(--surface-2)',
            boxShadow: '0 1px 3px rgba(var(--shadow-warm) / 0.18)',
          }}
        />
      ))}

      {scene.slots.map((slot) => (
        <button
          key={slot.gameId}
          type="button"
          style={caseStyle(slot, frontHeight)}
          onClick={() => onSelect(slot.gameId)}
          onMouseEnter={() => onHover(slot.gameId)}
          onMouseLeave={() => onHover(null)}
          onFocus={() => onHover(slot.gameId)}
          onBlur={() => onHover(null)}
          aria-label={slot.title}
          data-focus={slot.focus}
        >
          <Cover slot={slot} frontHeight={frontHeight} />
        </button>
      ))}
    </div>
  );
}
