/**
 * Renderer selection.
 *
 * ADR-0001 notes that Tauri on Linux renders in WebKitGTK, whose WebGL is
 * materially weaker than Chromium's. Rather than hoping, we probe once at boot
 * and pick an engine. This is not a user setting — a user should never be asked
 * to choose a renderer.
 */

export type EngineId = 'three' | 'css3d';

export interface EngineChoice {
  engine: EngineId;
  reason: string;
  /** Renderer string reported by the driver, when available. */
  renderer?: string;
}

const SOFTWARE_RENDERER = /llvmpipe|swrast|softpipe|swiftshader|basic render driver|warp/i;

/** Frame budget for a 60 fps target, with headroom for other work. */
const FRAME_BUDGET_MS = 22;

export function detectSoftwareRendering(): EngineChoice | null {
  if (typeof document === 'undefined') return null;

  const canvas = document.createElement('canvas');
  const gl =
    canvas.getContext('webgl2', { failIfMajorPerformanceCaveat: false }) ??
    canvas.getContext('webgl', { failIfMajorPerformanceCaveat: false });

  if (!gl) {
    return { engine: 'css3d', reason: 'no webgl context', renderer: undefined };
  }

  const debug = gl.getExtension('WEBGL_debug_renderer_info');
  const renderer = debug
    ? String(gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) ?? '')
    : String(gl.getParameter(gl.RENDERER) ?? '');

  // Release the probe context immediately; drivers are stingy with them.
  const lose = gl.getExtension('WEBGL_lose_context');
  lose?.loseContext();

  if (SOFTWARE_RENDERER.test(renderer)) {
    return { engine: 'css3d', reason: 'software rasteriser', renderer };
  }

  return null;
}

/**
 * Measures the first N frames of a trivial animation. A software rasteriser that
 * slips past the string check still fails this.
 */
export function measureFrameBudget(frames = 30): Promise<number | null> {
  if (typeof requestAnimationFrame === 'undefined') return Promise.resolve(null);

  return new Promise((resolve) => {
    const samples: number[] = [];
    let last = performance.now();
    let count = 0;

    const tick = (now: number) => {
      samples.push(now - last);
      last = now;
      if (++count < frames) {
        requestAnimationFrame(tick);
      } else {
        const sorted = [...samples].sort((a, b) => a - b);
        const p95 = sorted[Math.floor(sorted.length * 0.95)] ?? 0;
        resolve(p95);
      }
    };

    requestAnimationFrame(tick);
  });
}

export async function selectEngine(): Promise<EngineChoice> {
  const software = detectSoftwareRendering();
  if (software) return software;

  const p95 = await measureFrameBudget();
  if (p95 !== null && p95 > FRAME_BUDGET_MS) {
    return { engine: 'css3d', reason: `p95 frame time ${p95.toFixed(1)}ms` };
  }

  return { engine: 'three', reason: 'webgl available' };
}
