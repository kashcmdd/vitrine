# ADR-0008: First-run setup is a step registry

**Status:** Accepted · 2026-11-13

## Context

First run should be a real onboarding flow — welcome, find games, choose
appearance, done — not an empty grid with a button. It is where the product's
voice is established, and it is the difference between "a tool" and "an app".

The natural flow is scan → approve → shelf. But discovery lands at M4, and
putting the first impression at week eight is unacceptable: the wizard shell and
its tone need to exist while the app is still small, and they need to survive
every later milestone without a rewrite.

## Decision

Implement setup as a **backend-owned step registry**.

```rust
pub trait SetupStep: Send + Sync {
    fn id(&self) -> &'static str;                 // "welcome" | "library" | "appearance" | "detect"
    fn title(&self) -> &'static str;
    fn availability(&self, ctx: &SetupCtx) -> Availability;
    async fn run(&mut self, ctx: &mut SetupCtx, ui: StepUi) -> Result<StepOutcome>;
}

pub enum Availability { Available, Pending(&'static str), Skipped }
pub enum StepOutcome  { Completed, Skipped, Back }
```

- The frontend renders whatever the registry reports, in registry order. It
  knows nothing about specific steps.
- A step whose milestone has not landed returns `Pending("M4")` and is omitted
  from the flow, without the shell changing.
- `run()` communicates through a `StepUi` handle (progress, prompt, choice) so a
  step can drive a scan without the frontend hardcoding scan UI.
- Completion is recorded in `settings.setup_completed_at`; the whole flow is
  re-runnable from Settings.
- Adding the `detect` step at M4 is a registration, not a UI change.

## Consequences

**Positive**

- M1 ships a working wizard with three real steps (welcome → library →
  appearance) and no dependency on discovery.
- The M4 scan step plugs in without touching `features/setup/`.
- Individual steps are unit-testable in isolation, with no webview.
- Steps can be reordered, skipped, or conditionally hidden without editing the
  shell.

**Negative**

- The indirection is real overhead for a three-step flow; a hardcoded sequence
  would be faster to write initially.
- The `StepUi` protocol is a stable contract that must be designed carefully —
  changing it later touches every step. It is therefore intentionally small
  (progress, prompt, choice, log line) and versioned with the bindings.

## Rejected alternatives

| Alternative | Why not |
|---|---|
| Hardcoded wizard in the frontend | Puts the first impression behind M4, and forces UI changes for each new step |
| No wizard (empty shelf + add button) | Cheapest, but wastes the strongest moment for establishing the design's point of view |
| Wizard as a separate window | Two window lifecycles, two state machines, and it cannot inherit the shelf's motion language |
