//! Core game trait defining state transitions, observations, and compact codecs.

use core::fmt::Debug;
use core::hash::Hash;

use crate::buffer::Buffer;
use crate::compact::{CompactError, CompactSpec};
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome};

/// Deterministic game contract used by the session kernel.
///
/// Implementations provide pure state transition logic plus compact codec hooks
/// for actions and observations.
pub trait Game {
    /// Parameter bundle used to initialize/reset game state.
    type Params: Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Concrete game state.
    type State: Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Atomic player action type.
    type Action: Clone + Copy + Debug + Default + Eq + Hash + PartialEq;
    /// Canonical observation type shared across all viewpoints.
    type Obs: Clone + Debug + Default + Eq + PartialEq;
    /// Render/debug world view type.
    type WorldView: Clone + Debug + Default + Eq + PartialEq;
    /// Buffer type for active-player lists.
    type PlayerBuf: Buffer<Item = PlayerId> + Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Buffer type for legal actions.
    type ActionBuf: Buffer<Item = Self::Action> + Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Buffer type for joint actions.
    type JointActionBuf: Buffer<Item = PlayerAction<Self::Action>>
        + Clone
        + Debug
        + Default
        + Eq
        + Hash
        + PartialEq;
    /// Buffer type for per-player rewards.
    type RewardBuf: Buffer<Item = PlayerReward> + Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Buffer type for compact observation words.
    type WordBuf: Buffer<Item = u64> + Clone + Debug + Default + Eq + Hash + PartialEq;

    /// Stable machine-readable game name.
    fn name(&self) -> &'static str;
    /// Total number of players in the game.
    fn player_count(&self) -> usize;
    /// Returns default parameter bundle used by `init` and `SessionKernel::new`.
    fn default_params(&self) -> Self::Params {
        Self::Params::default()
    }

    /// Initialize deterministic state from a seed and parameter bundle.
    fn init_with_params(&self, seed: Seed, params: &Self::Params) -> Self::State;

    /// Initialize the deterministic state from a seed.
    fn init(&self, seed: Seed) -> Self::State {
        let params = self.default_params();
        self.init_with_params(seed, &params)
    }
    /// Whether the state is terminal.
    fn is_terminal(&self, state: &Self::State) -> bool;
    /// Emit active players for the current tick.
    fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf);
    /// Emit legal actions for a player in the current state.
    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf);
    /// Build a player-scoped observation.
    fn observe_player(&self, state: &Self::State, player: PlayerId) -> Self::Obs;
    /// Build a spectator observation.
    fn observe_spectator(&self, state: &Self::State) -> Self::Obs;
    /// Build a world/debug view consumed by render and tooling.
    fn world_view(&self, state: &Self::State) -> Self::WorldView;
    /// Apply one transition in-place.
    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<Self::RewardBuf>,
    );

    /// Compact codec descriptor for actions, observations and rewards.
    fn compact_spec(&self) -> CompactSpec {
        CompactSpec {
            action_count: 0,
            observation_bits: 0,
            observation_stream_len: 0,
            reward_bits: 1,
            min_reward: 0,
            max_reward: 0,
            reward_offset: 0,
        }
    }

    /// Encode an action into its compact integer representation.
    fn encode_action(&self, _action: &Self::Action) -> u64 {
        0
    }

    /// Decode a compact action value.
    fn decode_action(&self, _encoded: u64) -> Option<Self::Action> {
        None
    }

    /// Checked action decoding helper that yields a structured error.
    fn decode_action_checked(&self, encoded: u64) -> Result<Self::Action, CompactError> {
        self.decode_action(encoded)
            .ok_or(CompactError::InvalidActionEncoding { encoded })
    }

    /// Encode a player observation into compact words.
    fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        let _ = observation;
        out.clear();
    }

    /// Encode a spectator observation into compact words.
    fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        let _ = observation;
        out.clear();
    }

    /// Convenience helper that observes a player and encodes the result.
    fn encode_player_view(&self, state: &Self::State, player: PlayerId, out: &mut Self::WordBuf) {
        let observation = self.observe_player(state, player);
        self.encode_player_observation(&observation, out);
    }

    /// Validate compact observation shape against the declared compact spec.
    fn compact_invariant(&self, words: &Self::WordBuf) -> bool {
        self.compact_spec()
            .validate_observation_words(words.as_slice())
            .is_ok()
    }

    /// State invariant used by checked stepping and proof helpers.
    fn state_invariant(&self, _state: &Self::State) -> bool {
        true
    }

    /// Action invariant used by checked stepping and proof helpers.
    fn action_invariant(&self, _action: &Self::Action) -> bool {
        true
    }

    /// Invariant for player observations.
    fn player_observation_invariant(
        &self,
        _state: &Self::State,
        _player: PlayerId,
        _observation: &Self::Obs,
    ) -> bool {
        true
    }

    /// Invariant for spectator observations.
    fn spectator_observation_invariant(
        &self,
        _state: &Self::State,
        _observation: &Self::Obs,
    ) -> bool {
        true
    }

    /// Invariant for world/debug views.
    fn world_view_invariant(&self, _state: &Self::State, _world: &Self::WorldView) -> bool {
        true
    }

    /// Transition postcondition checked in instrumented stepping.
    fn transition_postcondition(
        &self,
        _pre: &Self::State,
        _actions: &Self::JointActionBuf,
        _post: &Self::State,
        _outcome: &StepOutcome<Self::RewardBuf>,
    ) -> bool {
        true
    }

    /// Maximum supported player count from buffer capacity.
    fn max_players(&self) -> usize {
        <Self::PlayerBuf as Buffer>::CAPACITY
    }

    /// Convenience legality query backed by `legal_actions`.
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
