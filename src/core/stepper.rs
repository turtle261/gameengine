//! Session stepping adapters for checked and unchecked execution paths.

use crate::game::Game;
use crate::session::{HistoryStore, SessionKernel};
use crate::types::StepOutcome;

/// Minimal wrapper that executes unchecked kernel steps.
pub struct KernelStepper<'a, G: Game, H: HistoryStore<G>> {
    session: &'a mut SessionKernel<G, H>,
}

impl<'a, G: Game, H: HistoryStore<G>> KernelStepper<'a, G, H> {
    /// Creates an unchecked stepper over a session kernel.
    pub fn new(session: &'a mut SessionKernel<G, H>) -> Self {
        Self { session }
    }

    /// Applies one joint-action step.
    pub fn step(&mut self, actions: &G::JointActionBuf) -> &StepOutcome<G::RewardBuf> {
        self.session.step_with_joint_actions(actions)
    }
}

/// Wrapper that executes checked kernel steps with contract assertions.
pub struct CheckedStepper<'a, G: Game, H: HistoryStore<G>> {
    session: &'a mut SessionKernel<G, H>,
}

impl<'a, G: Game, H: HistoryStore<G>> CheckedStepper<'a, G, H> {
    /// Creates a checked stepper over a session kernel.
    pub fn new(session: &'a mut SessionKernel<G, H>) -> Self {
        Self { session }
    }

    /// Applies one checked joint-action step.
    pub fn step(&mut self, actions: &G::JointActionBuf) -> &StepOutcome<G::RewardBuf> {
        self.session.step_with_joint_actions_checked(actions)
    }
}
