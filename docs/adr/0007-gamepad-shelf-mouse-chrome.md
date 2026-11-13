# ADR-0007: Gamepad-first shelf, mouse-first chrome

**Status:** Accepted · 2026-11-13

## Context

Both input models were requested at equal priority. Taken literally, "equal
priority everywhere" means every dialog, combobox, path picker and text field
needs a focus model and spatial-navigation rules — roughly 1.5 weeks of work
that is invisible when correct and infuriating when absent.

That effort is also misallocated. Two surfaces exist with different input
realities:

- **The shelf and the game detail view** are browsed from a couch, on a TV or a
  Steam Deck, where a controller is the natural device.
- **Settings, runner configuration, discovery review and API-key entry** are
  performed sitting at a keyboard. Nobody types a Wine prefix path with a
  D-pad.

## Decision

Split by surface, not by feature.

- **Gamepad-first:** the shelf, the focus ring, game detail navigation, and the
  gamepad actions on a selected case (launch, inspect, favourite).
- **Mouse-first:** settings, runner and emulator configuration, discovery
  review, API keys, the setup wizard's non-shelf steps.
- **One focus engine** (`src/shelf/focus/useFocusGrid.ts`) owns spatial
  navigation and a single `focus: 'none' | 'hover' | 'selected'` state. Pointer
  input updates the same state, so hover and D-pad selection cannot diverge.
- Focus is visually specified in the token file, not per component: scale 1.06,
  2 px brass outline at 4 px offset, unfocused items at 0.85 opacity (not 0.6 —
  a collection must remain readable as a collection).

## Consequences

**Positive**

- The couch experience is correct where it matters, at roughly a third of the
  cost of universal parity.
- One focus state means the CSS-3D and Three.js engines render identical focus
  behaviour, so the fallback engine needs no separate interaction work.
- Chrome stays ordinary DOM; no custom spatial navigation in text-heavy forms,
  which is where such systems fail.

**Negative**

- A controller user who opens Settings needs a mouse or a keyboard. Mitigated by
  keeping Settings navigable with Tab + Enter (standard DOM focus, which a
  gamepad driver can emulate) and by never putting destructive actions behind
  gamepad-only affordances.
- If parity is later wanted for chrome, it is additive: the focus engine already
  exists and can wrap form controls.

## Non-goals

- Text entry via a gamepad on-screen keyboard. Out of scope; the wizard is
  completable with pointer or keyboard, and the default path (detect my games)
  requires no typing at all.
