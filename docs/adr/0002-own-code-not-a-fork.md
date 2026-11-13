# ADR-0002: Own code under MIT; reference projects are never a source of files

**Status:** Accepted · 2026-11-13

## Context

Boxshelf is the closest existing product to the intended experience: games as
physical cases on a shelf, flip the case, pull the disc. It is genuinely open
source under **GPL-3.0-only** (`sassiestrhihno/boxshelf-source`; the
`-beta` repo hosts binaries only and carries no license of its own).

Three things were tangled together in the original intent to "fork it":

1. **Legal** — copying code triggers copyleft.
2. **Creative** — not wanting to look like someone else's product.
3. **Practical** — inheriting maintenance of a 1.4 MB, single-maintainer,
   untested beta.

The decisive fact is that **(1) admits no partial fork**. Under GPL-3.0, lifting
a single file — one stylesheet, one scene module — makes the entire distributed
application GPL-3.0-only. There is no "GPL part" of a binary.

## Decision

Implement the interface from scratch. **No file, stylesheet, shader, image, or
font file from Boxshelf enters this repository.** Licence: MIT.

Boxshelf remains legitimate *reference material* for:

- the general interaction grammar (pull → flip → open → disc),
- the idea that front, back, spine and label are separate artwork slots,
- per-system physical case dimensions (real-world measurements, not authored
  content).

Everything perceptible is original: palette, typography, lighting model,
materials, motion, layout, and sound.

## Consequences

**Positive**

- Licence is MIT; every future option (proprietary, dual-licence, app store)
  stays open.
- No upstream dependency, no divergence debt, no obligation to track a moving
  beta.
- The shelf is ~800 lines of owned TypeScript rather than 366 KB of adapted
  code, so it can be extended without fighting someone else's abstractions.
- No per-release source-publication obligation.

**Negative**

- More work in M2 than a port would have been — roughly break-even once the
  46-channel IPC bridge reimplementation a port would require is counted.
- The result will not look like Boxshelf on day one; it must earn its own
  identity (see the five-of-seven differentiation rule in the blueprint).

## Enforcement

- Every dependency must be MIT / Apache-2.0 / BSD / OFL / Zlib compatible.
  Recorded in `THIRD-PARTY-NOTICES.md`.
- Any Three.js scene code is written here; the vendored engine comes from npm.
- If this decision is ever revisited, it must be revisited **before** any code
  is copied, and it invalidates the MIT licence for the whole application.
