use core::fmt::Debug;
use core::hash::Hash;

use crate::buffer::Buffer;
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome};

pub trait Game {
    type State: Clone + Debug + Default + Eq + Hash + PartialEq;
    type Action: Clone + Copy + Debug + Default + Eq + Hash + PartialEq;
    type PlayerObservation: Clone + Debug + Default + Eq + PartialEq;
    type SpectatorObservation: Clone + Debug + Default + Eq + PartialEq;
    type WorldView: Clone + Debug + Default + Eq + PartialEq;
    type PlayerBuf: Buffer<Item = PlayerId> + Clone + Debug + Default + Eq + Hash + PartialEq;
    type ActionBuf: Buffer<Item = Self::Action> + Clone + Debug + Default + Eq + Hash + PartialEq;
    type JointActionBuf: Buffer<Item = PlayerAction<Self::Action>>
        + Clone
        + Debug
        + Default
        + Eq
        + Hash
        + PartialEq;
    type RewardBuf: Buffer<Item = PlayerReward> + Clone + Debug + Default + Eq + Hash + PartialEq;
    type WordBuf: Buffer<Item = u64> + Clone + Debug + Default + Eq + Hash + PartialEq;

    fn name(&self) -> &'static str;
    fn player_count(&self) -> usize;
    fn init(&self, seed: Seed) -> Self::State;
    fn is_terminal(&self, state: &Self::State) -> bool;
    fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf);
    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf);
    fn observe_player(&self, state: &Self::State, player: PlayerId) -> Self::PlayerObservation;
    fn observe_spectator(&self, state: &Self::State) -> Self::SpectatorObservation;
    fn world_view(&self, state: &Self::State) -> Self::WorldView;
    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<Self::RewardBuf>,
    );

    fn state_invariant(&self, _state: &Self::State) -> bool {
        true
    }

    fn action_invariant(&self, _action: &Self::Action) -> bool {
        true
    }

    fn player_observation_invariant(
        &self,
        _state: &Self::State,
        _player: PlayerId,
        _observation: &Self::PlayerObservation,
    ) -> bool {
        true
    }

    fn spectator_observation_invariant(
        &self,
        _state: &Self::State,
        _observation: &Self::SpectatorObservation,
    ) -> bool {
        true
    }

    fn world_view_invariant(&self, _state: &Self::State, _world: &Self::WorldView) -> bool {
        true
    }

    fn transition_postcondition(
        &self,
        _pre: &Self::State,
        _actions: &Self::JointActionBuf,
        _post: &Self::State,
        _outcome: &StepOutcome<Self::RewardBuf>,
    ) -> bool {
        true
    }

    fn max_players(&self) -> usize {
        <Self::PlayerBuf as Buffer>::CAPACITY
    }

    fn is_action_legal(
        &self,
        state: &Self::State,
        player: PlayerId,
        action: &Self::Action,
    ) -> bool {
        let mut legal = Self::ActionBuf::default();
        self.legal_actions(state, player, &mut legal);
        let actions = legal.as_slice();
        let mut index = 0usize;
        while index < actions.len() {
            if &actions[index] == action {
                return true;
            }
            index += 1;
        }
        false
    }
}
