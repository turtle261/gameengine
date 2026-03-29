use std::fmt::Debug;
use std::hash::Hash;

use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, Seed, StepOutcome};

pub trait Game {
    type State: Clone + Debug + Eq + Hash + PartialEq;
    type Action: Clone + Debug + Eq + Hash + PartialEq;
    type PlayerObservation: Clone + Debug + Eq + PartialEq;
    type SpectatorObservation: Clone + Debug + Eq + PartialEq;

    fn name(&self) -> &'static str;
    fn player_count(&self) -> usize;
    fn init(&self, seed: Seed) -> Self::State;
    fn is_terminal(&self, state: &Self::State) -> bool;
    fn players_to_act(&self, state: &Self::State, out: &mut Vec<PlayerId>);
    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Vec<Self::Action>);
    fn observe_player(&self, state: &Self::State, player: PlayerId) -> Self::PlayerObservation;
    fn observe_spectator(&self, state: &Self::State) -> Self::SpectatorObservation;
    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &[PlayerAction<Self::Action>],
        rng: &mut DeterministicRng,
        out: &mut StepOutcome,
    );
}
