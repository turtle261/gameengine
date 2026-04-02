//! Reusable helpers and authoring adapter for deterministic single-player games.

use core::fmt::Debug;
use core::hash::Hash;

use crate::buffer::{Buffer, FixedVec};
use crate::compact::{CompactError, CompactSpec};
use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Reward, Seed, StepOutcome};

/// Canonical acting player id used by single-player environments.
pub const SOLO_PLAYER: PlayerId = 0;

/// Canonical fixed-capacity player buffer for single-player games.
pub type SinglePlayerBuf = FixedVec<PlayerId, 1>;
/// Canonical fixed-capacity joint-action buffer for single-player games.
pub type SinglePlayerJointActionBuf<A> = FixedVec<PlayerAction<A>, 1>;
/// Canonical fixed-capacity reward buffer for single-player games.
pub type SinglePlayerRewardBuf = FixedVec<PlayerReward, 1>;

/// Returns true when `player` can act in a non-terminal single-player state.
pub const fn can_act(player: PlayerId, terminal: bool) -> bool {
    player == SOLO_PLAYER && !terminal
}

/// Clears and emits the single acting player when the state is ongoing.
pub fn write_players_to_act<B>(out: &mut B, terminal: bool)
where
    B: Buffer<Item = PlayerId>,
{
    out.clear();
    if !terminal {
        out.push(SOLO_PLAYER).unwrap();
    }
}

/// Returns the first action assigned to the single acting player.
pub fn first_action<A: Copy>(joint_actions: &[PlayerAction<A>]) -> Option<A> {
    for candidate in joint_actions {
        if candidate.player == SOLO_PLAYER {
            return Some(candidate.action);
        }
    }
    None
}

/// Appends one reward entry for the single acting player.
pub fn push_reward<B>(out: &mut B, reward: Reward)
where
    B: Buffer<Item = PlayerReward>,
{
    out.push(PlayerReward {
        player: SOLO_PLAYER,
        reward,
    })
    .unwrap();
}

/// Ergonomic authoring trait for deterministic single-player games.
///
/// Implement this trait to avoid repeating boilerplate for:
///
/// - player-id dispatch (`player_count = 1`, `players_to_act`, legality gating),
/// - joint-action extraction (`Option<Action>` from one-player action stream),
/// - fixed-capacity reward and joint-action buffer wiring.
pub trait SinglePlayerGame {
    /// Parameter bundle used to initialize/reset game state.
    type Params: Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Concrete game state.
    type State: Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Atomic action type.
    type Action: Clone + Copy + Debug + Default + Eq + Hash + PartialEq;
    /// Canonical observation type.
    type Obs: Clone + Debug + Default + Eq + PartialEq;
    /// Render/debug world view.
    type WorldView: Clone + Debug + Default + Eq + PartialEq;
    /// Buffer type for legal actions.
    type ActionBuf: Buffer<Item = Self::Action> + Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Buffer type for compact observation words.
    type WordBuf: Buffer<Item = u64> + Clone + Debug + Default + Eq + Hash + PartialEq;

    /// Stable machine-readable game name.
    fn name(&self) -> &'static str;
    /// Returns default parameter bundle used by `init` and `SessionKernel::new`.
    fn default_params(&self) -> Self::Params {
        Self::Params::default()
    }
    /// Returns whether a parameter bundle is valid for `init_with_params`.
    fn params_invariant(&self, _params: &Self::Params) -> bool {
        true
    }
    /// Initialize deterministic state from a seed and parameter bundle.
    fn init_with_params(&self, seed: Seed, params: &Self::Params) -> Self::State;
    /// Whether the state is terminal.
    fn is_terminal(&self, state: &Self::State) -> bool;
    /// Emit legal actions for the single acting player in the current state.
    fn legal_actions(&self, state: &Self::State, out: &mut Self::ActionBuf);
    /// Build player observation.
    fn observe_player(&self, state: &Self::State) -> Self::Obs;
    /// Build spectator observation.
    fn observe_spectator(&self, state: &Self::State) -> Self::Obs {
        self.observe_player(state)
    }
    /// Build world/debug view.
    fn world_view(&self, state: &Self::State) -> Self::WorldView;
    /// Apply one transition in-place from an optional single-player action.
    fn step_in_place(
        &self,
        state: &mut Self::State,
        action: Option<Self::Action>,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<SinglePlayerRewardBuf>,
    );

    /// Compact codec descriptor for actions, observations, and rewards.
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

    /// Compact codec descriptor for an explicit parameter bundle.
    fn compact_spec_for_params(&self, _params: &Self::Params) -> CompactSpec {
        self.compact_spec()
    }

    /// Encode an action into compact integer representation.
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
    fn encode_player_observation(&self, _observation: &Self::Obs, out: &mut Self::WordBuf) {
        out.clear();
    }

    /// Encode a spectator observation into compact words.
    fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        self.encode_player_observation(observation, out);
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
    fn player_observation_invariant(&self, _state: &Self::State, _observation: &Self::Obs) -> bool {
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
        _action: Option<Self::Action>,
        _post: &Self::State,
        _outcome: &StepOutcome<SinglePlayerRewardBuf>,
    ) -> bool {
        true
    }
}

impl<T> Game for T
where
    T: SinglePlayerGame,
{
    type Params = T::Params;
    type State = T::State;
    type Action = T::Action;
    type Obs = T::Obs;
    type WorldView = T::WorldView;
    type PlayerBuf = SinglePlayerBuf;
    type ActionBuf = T::ActionBuf;
    type JointActionBuf = SinglePlayerJointActionBuf<Self::Action>;
    type RewardBuf = SinglePlayerRewardBuf;
    type WordBuf = T::WordBuf;

    fn name(&self) -> &'static str {
        <T as SinglePlayerGame>::name(self)
    }

    fn player_count(&self) -> usize {
        1
    }

    fn default_params(&self) -> Self::Params {
        <T as SinglePlayerGame>::default_params(self)
    }

    fn params_invariant(&self, params: &Self::Params) -> bool {
        <T as SinglePlayerGame>::params_invariant(self, params)
    }

    fn init_with_params(&self, seed: Seed, params: &Self::Params) -> Self::State {
        <T as SinglePlayerGame>::init_with_params(self, seed, params)
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        <T as SinglePlayerGame>::is_terminal(self, state)
    }

    fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf) {
        write_players_to_act(out, self.is_terminal(state));
    }

    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf) {
        out.clear();
        if !can_act(player, self.is_terminal(state)) {
            return;
        }
        <T as SinglePlayerGame>::legal_actions(self, state, out);
    }

    fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::Obs {
        <T as SinglePlayerGame>::observe_player(self, state)
    }

    fn observe_spectator(&self, state: &Self::State) -> Self::Obs {
        <T as SinglePlayerGame>::observe_spectator(self, state)
    }

    fn world_view(&self, state: &Self::State) -> Self::WorldView {
        <T as SinglePlayerGame>::world_view(self, state)
    }

    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<Self::RewardBuf>,
    ) {
        <T as SinglePlayerGame>::step_in_place(
            self,
            state,
            first_action(joint_actions.as_slice()),
            rng,
            out,
        );
    }

    fn compact_spec(&self) -> CompactSpec {
        <T as SinglePlayerGame>::compact_spec(self)
    }

    fn compact_spec_for_params(&self, params: &Self::Params) -> CompactSpec {
        <T as SinglePlayerGame>::compact_spec_for_params(self, params)
    }

    fn encode_action(&self, action: &Self::Action) -> u64 {
        <T as SinglePlayerGame>::encode_action(self, action)
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        <T as SinglePlayerGame>::decode_action(self, encoded)
    }

    fn decode_action_checked(&self, encoded: u64) -> Result<Self::Action, CompactError> {
        <T as SinglePlayerGame>::decode_action_checked(self, encoded)
    }

    fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        <T as SinglePlayerGame>::encode_player_observation(self, observation, out)
    }

    fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        <T as SinglePlayerGame>::encode_spectator_observation(self, observation, out)
    }

    fn state_invariant(&self, state: &Self::State) -> bool {
        <T as SinglePlayerGame>::state_invariant(self, state)
    }

    fn action_invariant(&self, action: &Self::Action) -> bool {
        <T as SinglePlayerGame>::action_invariant(self, action)
    }

    fn player_observation_invariant(
        &self,
        state: &Self::State,
        _player: PlayerId,
        observation: &Self::Obs,
    ) -> bool {
        <T as SinglePlayerGame>::player_observation_invariant(self, state, observation)
    }

    fn spectator_observation_invariant(
        &self,
        state: &Self::State,
        observation: &Self::Obs,
    ) -> bool {
        <T as SinglePlayerGame>::spectator_observation_invariant(self, state, observation)
    }

    fn world_view_invariant(&self, state: &Self::State, world: &Self::WorldView) -> bool {
        <T as SinglePlayerGame>::world_view_invariant(self, state, world)
    }

    fn transition_postcondition(
        &self,
        pre: &Self::State,
        actions: &Self::JointActionBuf,
        post: &Self::State,
        outcome: &StepOutcome<Self::RewardBuf>,
    ) -> bool {
        <T as SinglePlayerGame>::transition_postcondition(
            self,
            pre,
            first_action(actions.as_slice()),
            post,
            outcome,
        )
    }
}
