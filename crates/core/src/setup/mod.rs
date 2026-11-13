//! First-run setup as a backend-owned step registry (ADR-0008).
//!
//! The point of this module is sequencing: the wizard shell, and the tone it
//! establishes, must exist in M1 — but the most valuable step (find my games)
//! needs discovery, which lands in M4. A registry lets a step declare itself
//! `Pending("M4")` and simply not appear, so later milestones register a step
//! instead of rewriting the shell.
//!
//! The frontend renders whatever [`SetupRegistry`] reports and knows nothing
//! about specific steps.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;

/// Whether a step can run right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    /// Blocked on a milestone that has not shipped. Carries the milestone name so
    /// the placeholder text is accurate rather than vague.
    Pending(&'static str),
    /// Deliberately omitted, e.g. re-running setup with the library already
    /// populated.
    Skipped,
}

/// What a step decided when it finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepOutcome {
    Completed,
    Skipped,
    /// The user pressed back; the registry walks to the previous available step.
    Back,
}

/// Everything a step may read or mutate. Deliberately tiny: a large context is a
/// large contract to keep stable across milestones.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetupCtx {
    /// Set once the flow completes; presence means setup is not re-run on boot.
    pub completed_at: Option<String>,
    /// Roots the user chose to scan. Populated by the M4 detect step.
    pub scan_roots: Vec<PathBuf>,
    /// How many games the library holds, so later steps can adapt their copy.
    pub library_size: usize,
}

/// How a step talks to the UI while it runs.
///
/// Kept to four operations on purpose: every addition here is a contract change
/// that touches every step.
#[async_trait]
pub trait StepUi: Send + Sync {
    /// A determinate or indeterminate progress update.
    async fn progress(&self, fraction: Option<f32>, message: &str);
    /// A line for the scrollback shown during long steps.
    async fn log_line(&self, line: &str);
}

/// One page of the setup flow.
#[async_trait]
pub trait SetupStep: Send + Sync {
    /// Stable identifier, also the routing key in the frontend.
    fn id(&self) -> &'static str;

    /// Heading shown to the user.
    fn title(&self) -> &'static str;

    /// Whether this step should appear in the flow right now.
    fn availability(&self, ctx: &SetupCtx) -> Availability;

    /// Runs the step to completion or until the user moves on.
    async fn run(&mut self, ctx: &mut SetupCtx, ui: &dyn StepUi) -> Result<StepOutcome>;
}

/// The ordered set of steps.
#[derive(Default)]
pub struct SetupRegistry {
    steps: Vec<Box<dyn SetupStep>>,
}

impl SetupRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registration order is presentation order. Later milestones append or
    /// insert; they never edit the shell.
    pub fn register(&mut self, step: impl SetupStep + 'static) {
        self.steps.push(Box::new(step));
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The ids and titles that should be shown, in order.
    pub fn flow(&self, ctx: &SetupCtx) -> Vec<(&'static str, &'static str)> {
        self.steps
            .iter()
            .filter(|step| matches!(step.availability(ctx), Availability::Available))
            .map(|step| (step.id(), step.title()))
            .collect()
    }

    /// Look up a step by id.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut (dyn SetupStep + '_)> {
        self.steps
            .iter_mut()
            .find(|step| step.id() == id)
            .map(|step| step.as_mut())
    }

    /// The next available step after `current`, so the shell can advance without
    /// knowing which steps exist.
    pub fn next_after(&self, ctx: &SetupCtx, current: Option<&str>) -> Option<&'static str> {
        let mut take_next = current.is_none();
        for step in &self.steps {
            if !matches!(step.availability(ctx), Availability::Available) {
                continue;
            }
            if take_next {
                return Some(step.id());
            }
            if Some(step.id()) == current {
                take_next = true;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;

    struct TestStep {
        id: &'static str,
        title: &'static str,
        availability: Availability,
    }

    #[async_trait]
    impl SetupStep for TestStep {
        fn id(&self) -> &'static str {
            self.id
        }
        fn title(&self) -> &'static str {
            self.title
        }
        fn availability(&self, _ctx: &SetupCtx) -> Availability {
            self.availability
        }
        async fn run(&mut self, _ctx: &mut SetupCtx, _ui: &dyn StepUi) -> Result<StepOutcome> {
            Ok(StepOutcome::Completed)
        }
    }

    struct NoopUi;

    #[async_trait]
    impl StepUi for NoopUi {
        async fn progress(&self, _fraction: Option<f32>, _message: &str) {}
        async fn log_line(&self, _line: &str) {}
    }

    fn registry() -> SetupRegistry {
        let mut registry = SetupRegistry::new();
        registry.register(TestStep {
            id: "welcome",
            title: "Welcome",
            availability: Availability::Available,
        });
        registry.register(TestStep {
            id: "detect",
            title: "Find my games",
            availability: Availability::Pending("M4"),
        });
        registry.register(TestStep {
            id: "appearance",
            title: "Appearance",
            availability: Availability::Available,
        });
        registry.register(TestStep {
            id: "legacy",
            title: "Legacy",
            availability: Availability::Skipped,
        });
        registry
    }

    #[test]
    fn pending_and_skipped_steps_are_not_in_the_flow() {
        let flow = registry().flow(&SetupCtx::default());
        assert_eq!(
            flow.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec!["welcome", "appearance"]
        );
    }

    #[test]
    fn a_step_blocked_on_a_milestone_appears_once_available() {
        // This is the whole point of the registry: M4 flips availability and the
        // shell needs no change.
        let mut registry = registry();
        registry.register(TestStep {
            id: "detect",
            title: "Find my games",
            availability: Availability::Available,
        });
        let ctx = SetupCtx::default();
        assert!(registry.flow(&ctx).iter().any(|(id, _)| *id == "detect"));
    }

    #[test]
    fn advancing_skips_unavailable_steps() {
        let registry = registry();
        let ctx = SetupCtx::default();
        assert_eq!(registry.next_after(&ctx, None), Some("welcome"));
        assert_eq!(registry.next_after(&ctx, Some("welcome")), Some("appearance"));
        assert_eq!(registry.next_after(&ctx, Some("appearance")), None);
    }

    #[test]
    fn an_unknown_current_step_still_advances_to_the_first() {
        let registry = registry();
        let ctx = SetupCtx::default();
        // A stale frontend route must not wedge the flow.
        assert_eq!(registry.next_after(&ctx, Some("gone")), None);
    }

    #[tokio::test]
    async fn running_a_step_returns_its_outcome() {
        let mut registry = registry();
        let mut ctx = SetupCtx::default();
        let ui = NoopUi;
        let step = registry.get_mut("welcome").expect("registered");
        let outcome = step.run(&mut ctx, &ui).await.expect("step succeeds");
        assert_eq!(outcome, StepOutcome::Completed);
    }
}
