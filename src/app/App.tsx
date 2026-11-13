import { useEffect, useMemo, useState } from 'react';
import { DEFAULT_LAYOUT_OPTIONS, buildScene } from '@/shelf/layout';
import { Css3dShelf } from '@/shelf/engine/css3d/Shelf';
import { selectEngine, type EngineChoice } from '@/shelf/engine';
import { MOCK_LIBRARY } from '@/lib/mockLibrary';
import type { FocusState, LightingMode, ShelfView } from '@/shelf/scene';

/**
 * M0 shell. Renders the shelf from mock data so the scene model, both layouts
 * and the CSS-3D engine can be reviewed before any backend exists.
 */
export function App() {
  // Query params make both views reviewable without a backend or clicks.
  const params = new URLSearchParams(window.location.search);
  const [view, setView] = useState<ShelfView>(
    params.get('view') === 'front' ? 'front' : 'spine',
  );
  const [theme, setTheme] = useState<LightingMode>(
    params.get('theme') === 'room' ? 'room' : 'day',
  );
  const [hovered, setHovered] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [viewport, setViewport] = useState({ width: 1280, height: 800 });
  const [engine, setEngine] = useState<EngineChoice | null>(null);

  useEffect(() => {
    void selectEngine().then(setEngine);
  }, []);

  useEffect(() => {
    document.documentElement.dataset['theme'] = theme;
  }, [theme]);

  useEffect(() => {
    const sync = () =>
      setViewport({ width: window.innerWidth, height: window.innerHeight });
    sync();
    window.addEventListener('resize', sync);
    return () => window.removeEventListener('resize', sync);
  }, []);

  const scene = useMemo(
    () =>
      buildScene({
        cases: MOCK_LIBRARY,
        view,
        lighting: theme,
        viewport,
        options: DEFAULT_LAYOUT_OPTIONS,
        focusOf: (id): FocusState =>
          id === selected ? 'selected' : id === hovered ? 'hover' : 'none',
      }),
    [view, theme, viewport, hovered, selected],
  );

  const selectedGame = MOCK_LIBRARY.find((g) => g.gameId === selected);

  return (
    <div className="min-h-screen bg-surface-0 text-ink-900">
      <header className="flex items-baseline justify-between px-8 pt-8 pb-6">
        <div>
          <h1 className="type-display text-4xl">Vitrine</h1>
          <p className="type-badge mt-1 text-ink-500">
            {MOCK_LIBRARY.length} titles · {scene.extent.shelves}{' '}
            {scene.extent.shelves === 1 ? 'shelf' : 'shelves'}
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Toggle
            label="Spines"
            active={view === 'spine'}
            onClick={() => setView('spine')}
          />
          <Toggle
            label="Faces"
            active={view === 'front'}
            onClick={() => setView('front')}
          />
          <span className="mx-2 h-4 w-px bg-ink-300" aria-hidden />
          <Toggle
            label="Day"
            active={theme === 'day'}
            onClick={() => setTheme('day')}
          />
          <Toggle
            label="Room"
            active={theme === 'room'}
            onClick={() => setTheme('room')}
          />
        </div>
      </header>

      <main className="overflow-x-hidden px-0 pb-24">
        <Css3dShelf
          scene={scene}
          frontHeight={DEFAULT_LAYOUT_OPTIONS.frontHeight}
          onSelect={setSelected}
          onHover={setHovered}
        />
      </main>

      <footer className="fixed inset-x-0 bottom-0 border-t border-ink-300/30 bg-surface-0/95 px-8 py-3 backdrop-blur">
        <div className="flex items-baseline justify-between gap-6">
          <p className="type-display truncate text-lg">
            {selectedGame ? selectedGame.title : 'Select a case'}
          </p>
          <p className="type-data shrink-0 text-xs text-ink-500">
            {selectedGame?.system ?? '—'} · engine: {engine?.engine ?? '…'}
            {engine ? ` (${engine.reason})` : ''}
          </p>
        </div>
      </footer>
    </div>
  );
}

function Toggle({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={[
        'type-badge motion-standard rounded-none border px-3 py-1.5 uppercase',
        active
          ? 'border-brass bg-brass text-surface-0'
          : 'border-ink-300/50 text-ink-500 hover:border-brass hover:text-ink-900',
      ].join(' ')}
    >
      {label}
    </button>
  );
}
