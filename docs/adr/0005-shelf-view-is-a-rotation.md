# ADR-0005: The shelf view toggle is a rotation of one scene

**Status:** Accepted · 2026-11-13

## Context

Two resting states were requested, both first-class:

1. **Spine-first** — cases stand like books; you pull one out to see its face.
2. **Front-facing** — covers face the viewer, as in a conventional grid.

The naive implementation builds two views: two layouts, two renderers, two
component trees, two sets of tests. Every future shelf feature then ships twice.

A case is a box. Its spine and its front are adjacent faces of the same
geometry. At `rotationY = 90°` you see the spine; at `0°` you see the front.

## Decision

One scene, one geometry, one set of materials. The toggle interpolates
`rotationY` across all cases.

```ts
interface CaseSlot {
  rotationY: number;   // 90° spine, 0° front
  // ... position, scale, aspect, depth, faces, focus
}
```

`ShelfScene` is renderer-agnostic and produced by a layout strategy:

```ts
interface ShelfLayout {
  measure(cases: Case[], viewport: Rect): { slots: CaseSlot[]; extent: Extent };
}
```

- `spineRow` — continuous packing like books; cases touch; no gaps; rows break by
  shelf height.
- `frontGrid` — rows × columns with gaps and a perspective camera.

Two engines consume the same scene:

- `engine/three/` — WebGL, the primary renderer.
- `engine/css3d/` — `perspective` + `rotateY` transforms, used when the
  capability probe reports software rendering or a p95 frame time above 22 ms.

## Consequences

**Positive**

- The view toggle is a 420 ms staggered rotation rather than a screen transition.
  It becomes the product's signature motion at zero additional cost.
- Both views are exercised by the same layout and interaction tests.
- Company/user preference is a single `settings` value, not a divergent code
  path.
- Because the spine view requires no cover art, **the library is fully usable
  before any metadata has been fetched**. This decouples M2 from M6 and removes
  the "empty grid while scraping" failure mode.
- The CSS-3D engine keeps WebKitGTK (ADR-0001) from holding the product hostage.

**Negative**

- Spine art barely exists in metadata APIs, so a synthesiser is mandatory:
  system accent + Fraunces title capped at 22 characters + system badge + brass
  rule, rendered to webp and cached deterministically.
- Spine legibility constrains typography: no rotated text below 40 px, and long
  titles must truncate with intent rather than wrap.
- Cases overlap briefly during the layout interpolation; the 60 ms stagger keeps
  this reading as motion rather than collision.

## Rejected alternatives

| Alternative | Why not |
|---|---|
| Two independent views | Doubles every shelf feature and test, forever |
| Spine view as a filtered "list mode" | Loses the physicality that motivates the whole design |
| LOD switching to front art under N px | Reintroduces the artwork dependency the spine view exists to remove |
