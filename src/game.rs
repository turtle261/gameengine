//! Core game traits split into kernel, observation, compact, contract, and optional oracle layers.

use core::fmt::Debug;
use core::hash::Hash;

use crate::buffer::Buffer;
use crate::compact::{CompactError, CompactSpec};
use crate::rng::DeterministicRng;
use crate::types::{KernelOutcome, PlayerAction, PlayerId, PlayerReward, Seed};

/// Compatibility authoring trait that carries the full historical monolithic game surface.
///
/// New code should reason in terms of the split traits:
/// `GameKernel`, `ObservationModel`, `CompactCodec`, `ContractSurface`, and `OracleProjection`.
pub trait GameAuthoring {
    /// Parameter bundle used to initialize/reset game state.
    type Params: Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Concrete game state.
    type State: Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Atomic player action type.
    type Action: Clone + Copy + Debug + Default + Eq + Hash + PartialEq;
    /// Canonical observation type shared across viewpoints.
    type Obs: Clone + Debug + Default + Eq + PartialEq;
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

    /// Returns whether a parameter bundle is valid for `init_with_params`.
    fn params_invariant(&self, _params: &Self::Params) -> bool {
        true
    }

    /// Initialize deterministic state from a seed and parameter bundle.
    fn init_with_params(&self, seed: Seed, params: &Self::Params) -> Self::State;

    /// Initialize deterministic state from a seed and default params.
    fn init(&self, seed: Seed) -> Self::State {
        let params = self.default_params();
        self.init_with_params(seed, &params)
    }
    /// Whether the state is terminal.
    fn is_terminal(&self, state: &Self::State) -> bool;
    /// Emit active players for the current tick.
    fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf);
    /// Emit legal actions for one player in the current state.
    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf);
    /// Build a player-scoped observation.
    fn observe_player(&self, state: &Self::State, player: PlayerId) -> Self::Obs;
    /// Build a spectator observation.
    fn observe_spectator(&self, state: &Self::State) -> Self::Obs;
    /// Apply one transition in-place.
    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut KernelOutcome<Self::RewardBuf>,
    );

    /// Compact codec descriptor for an explicit parameter bundle.
    ///
    /// Per specification §6.5 this is the primary method and defaults to
    /// the parameter-independent zero-valued spec
    /// `(action_count=0, observation_bits=0, observation_stream_len=0,
    /// reward_bits=1, min_reward=0, max_reward=0, reward_offset=0)`.
    fn compact_spec_for(&self, _params: &Self::Params) -> CompactSpec {
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

    /// Compact codec descriptor for default parameters.
    ///
    /// Per specification §6.5 `compact_spec()` means
    /// `compact_spec_for(default_params())`. Session-level uses of the
    /// compact spec instead refer to the current session params.
    fn compact_spec(&self) -> CompactSpec {
        self.compact_spec_for(&self.default_params())
    }

    /// Encode an action into compact representation.
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
    fn encode_spectator_observation(&self, _observation: &Self::Obs, out: &mut Self::WordBuf) {
        out.clear();
    }

    /// Convenience helper that observes a player and encodes the result.
    fn encode_player_view(&self, state: &Self::State, player: PlayerId, out: &mut Self::WordBuf) {
        let observation = self.observe_player(state, player);
        self.encode_player_observation(&observation, out);
    }

    /// Validate compact observation shape against an explicit parameter bundle.
    fn compact_invariant_for(&self, params: &Self::Params, words: &Self::WordBuf) -> bool {
        self.compact_spec_for(params)
            .validate_observation_words(words.as_slice())
            .is_ok()
    }

    /// Validate compact observation shape against default params.
    fn compact_invariant(&self, words: &Self::WordBuf) -> bool {
        let params = self.default_params();
        self.compact_invariant_for(&params, words)
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

    /// Oracle-world invariant hook used by checked verification/session helpers.
    ///
    /// Games that additionally implement [`OracleProjection`] should override
    /// this hook to dispatch to their oracle `world_view` + `world_view_invariant`
    /// (the engine cannot wire that bridge automatically without trait
    /// specialization). Games that do not expose an oracle projection leave the
    /// default and the session's checked-stepping path treats the predicate as
    /// trivially true.
    fn oracle_world_view_invariant(&self, _state: &Self::State) -> bool {
        true
    }

    /// Transition postcondition checked in instrumented stepping.
    fn transition_postcondition(
        &self,
        _pre: &Self::State,
        _actions: &Self::JointActionBuf,
        _post: &Self::State,
        _outcome: &KernelOutcome<Self::RewardBuf>,
    ) -> bool {
        true
    }
}

/// Kernel transition surface over params/state/actions and joint-action stepping.
pub trait GameKernel {
    /// Parameter bundle used to initialize/reset game state.
    type Params: Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Concrete game state.
    type State: Clone + Debug + Default + Eq + Hash + PartialEq;
    /// Atomic player action type.
    type Action: Clone + Copy + Debug + Default + Eq + Hash + PartialEq;
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

    /// Stable machine-readable game name.
    fn name(&self) -> &'static str;
    /// Total number of players in the game.
    fn player_count(&self) -> usize;
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
    /// Initialize deterministic state from a seed and default params.
    fn init(&self, seed: Seed) -> Self::State {
        let params = self.default_params();
        self.init_with_params(seed, &params)
    }
    /// Whether the state is terminal.
    fn is_terminal(&self, state: &Self::State) -> bool;
    /// Emit active players for the current tick.
    fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf);
    /// Emit legal actions for one player in the current state.
    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf);
    /// Apply one transition in-place.
    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut KernelOutcome<Self::RewardBuf>,
    );
}

/// Observation and compact-observation encoding surface.
pub trait ObservationModel: GameKernel {
    /// Canonical observation type shared across viewpoints.
    type Obs: Clone + Debug + Default + Eq + PartialEq;
    /// Buffer type for compact observation words.
    type WordBuf: Buffer<Item = u64> + Clone + Debug + Default + Eq + Hash + PartialEq;

    /// Build a player-scoped observation.
    fn observe_player(&self, state: &Self::State, player: PlayerId) -> Self::Obs;
    /// Build a spectator observation.
    fn observe_spectator(&self, state: &Self::State) -> Self::Obs;
    /// Encode a player observation into compact words.
    fn encode_player_observation(&self, _observation: &Self::Obs, out: &mut Self::WordBuf) {
        out.clear();
    }
    /// Encode a spectator observation into compact words.
    fn encode_spectator_observation(&self, _observation: &Self::Obs, out: &mut Self::WordBuf) {
        out.clear();
    }
    /// Convenience helper that observes a player and encodes the result.
    fn encode_player_view(&self, state: &Self::State, player: PlayerId, out: &mut Self::WordBuf) {
        let observation = self.observe_player(state, player);
        self.encode_player_observation(&observation, out);
    }
}

/// Additive oracle/debug projection surface.
pub trait OracleProjection: ObservationModel {
    /// Oracle/debug world-view type.
    type WorldView: Clone + Debug + Default + Eq + PartialEq;

    /// Build an oracle/debug world view.
    fn world_view(&self, state: &Self::State) -> Self::WorldView;

    /// Invariant for world/debug views.
    fn world_view_invariant(&self, _state: &Self::State, _world: &Self::WorldView) -> bool {
        true
    }
}

/// Compact codec surface over actions, observations, and rewards.
pub trait CompactCodec: ObservationModel {
    /// Compact codec descriptor for an explicit parameter bundle.
    ///
    /// Per specification §6.5 this is the primary method and defaults to
    /// the parameter-independent zero-valued spec described there.
    fn compact_spec_for(&self, _params: &Self::Params) -> CompactSpec {
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

    /// Compact codec descriptor for default parameters.
    ///
    /// Per specification §6.5 `compact_spec()` means
    /// `compact_spec_for(default_params())`.
    fn compact_spec(&self) -> CompactSpec {
        self.compact_spec_for(&self.default_params())
    }

    /// Encode an action into compact representation.
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

    /// Validate compact observation shape against an explicit parameter bundle.
    fn compact_invariant_for(&self, params: &Self::Params, words: &Self::WordBuf) -> bool {
        self.compact_spec_for(params)
            .validate_observation_words(words.as_slice())
            .is_ok()
    }

    /// Validate compact observation shape against default params.
    fn compact_invariant(&self, words: &Self::WordBuf) -> bool {
        let params = self.default_params();
        self.compact_invariant_for(&params, words)
    }
}

/// Runtime contract/invariant hook surface.
pub trait ContractSurface: ObservationModel {
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

    /// Oracle-world invariant hook used by checked verification/session helpers.
    ///
    /// Games that expose an oracle world view via [`OracleProjection`] should
    /// override this hook to assert their `world_view_invariant` against the
    /// currently projected world. Games without an oracle surface leave the
    /// default and the assertion becomes a no-op.
    fn oracle_world_view_invariant(&self, _state: &Self::State) -> bool {
        true
    }

    /// Transition postcondition checked in instrumented stepping.
    fn transition_postcondition(
        &self,
        _pre: &Self::State,
        _actions: &Self::JointActionBuf,
        _post: &Self::State,
        _outcome: &KernelOutcome<Self::RewardBuf>,
    ) -> bool {
        true
    }
}

/// Curated game surface used by session/environment/runtime helpers.
///
/// This umbrella intentionally excludes `OracleProjection`.
pub trait Game: GameKernel + ObservationModel + CompactCodec + ContractSurface {
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

impl<T> Game for T where T: GameKernel + ObservationModel + CompactCodec + ContractSurface {}

impl<T> GameKernel for T
where
    T: GameAuthoring,
{
    type Params = T::Params;
    type State = T::State;
    type Action = T::Action;
    type PlayerBuf = T::PlayerBuf;
    type ActionBuf = T::ActionBuf;
    type JointActionBuf = T::JointActionBuf;
    type RewardBuf = T::RewardBuf;

    fn name(&self) -> &'static str {
        <T as GameAuthoring>::name(self)
    }

    fn player_count(&self) -> usize {
        <T as GameAuthoring>::player_count(self)
    }

    fn default_params(&self) -> Self::Params {
        <T as GameAuthoring>::default_params(self)
    }

    fn params_invariant(&self, params: &Self::Params) -> bool {
        <T as GameAuthoring>::params_invariant(self, params)
    }

    fn init_with_params(&self, seed: Seed, params: &Self::Params) -> Self::State {
        <T as GameAuthoring>::init_with_params(self, seed, params)
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        <T as GameAuthoring>::is_terminal(self, state)
    }

    fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf) {
        <T as GameAuthoring>::players_to_act(self, state, out)
    }

    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf) {
        <T as GameAuthoring>::legal_actions(self, state, player, out)
    }

    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut KernelOutcome<Self::RewardBuf>,
    ) {
        <T as GameAuthoring>::step_in_place(self, state, joint_actions, rng, out)
    }
}

impl<T> ObservationModel for T
where
    T: GameAuthoring,
{
    type Obs = T::Obs;
    type WordBuf = T::WordBuf;

    fn observe_player(&self, state: &Self::State, player: PlayerId) -> Self::Obs {
        <T as GameAuthoring>::observe_player(self, state, player)
    }

    fn observe_spectator(&self, state: &Self::State) -> Self::Obs {
        <T as GameAuthoring>::observe_spectator(self, state)
    }

    fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        <T as GameAuthoring>::encode_player_observation(self, observation, out)
    }

    fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        <T as GameAuthoring>::encode_spectator_observation(self, observation, out)
    }

    fn encode_player_view(&self, state: &Self::State, player: PlayerId, out: &mut Self::WordBuf) {
        <T as GameAuthoring>::encode_player_view(self, state, player, out)
    }
}

impl<T> CompactCodec for T
where
    T: GameAuthoring,
{
    fn compact_spec(&self) -> CompactSpec {
        <T as GameAuthoring>::compact_spec(self)
    }

    fn compact_spec_for(&self, params: &Self::Params) -> CompactSpec {
        <T as GameAuthoring>::compact_spec_for(self, params)
    }

    fn encode_action(&self, action: &Self::Action) -> u64 {
        <T as GameAuthoring>::encode_action(self, action)
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        <T as GameAuthoring>::decode_action(self, encoded)
    }

    fn decode_action_checked(&self, encoded: u64) -> Result<Self::Action, CompactError> {
        <T as GameAuthoring>::decode_action_checked(self, encoded)
    }

    fn compact_invariant_for(&self, params: &Self::Params, words: &Self::WordBuf) -> bool {
        <T as GameAuthoring>::compact_invariant_for(self, params, words)
    }

    fn compact_invariant(&self, words: &Self::WordBuf) -> bool {
        <T as GameAuthoring>::compact_invariant(self, words)
    }
}

impl<T> ContractSurface for T
where
    T: GameAuthoring,
{
    fn state_invariant(&self, state: &Self::State) -> bool {
        <T as GameAuthoring>::state_invariant(self, state)
    }

    fn action_invariant(&self, action: &Self::Action) -> bool {
        <T as GameAuthoring>::action_invariant(self, action)
    }

    fn player_observation_invariant(
        &self,
        state: &Self::State,
        player: PlayerId,
        observation: &Self::Obs,
    ) -> bool {
        <T as GameAuthoring>::player_observation_invariant(self, state, player, observation)
    }

    fn spectator_observation_invariant(
        &self,
        state: &Self::State,
        observation: &Self::Obs,
    ) -> bool {
        <T as GameAuthoring>::spectator_observation_invariant(self, state, observation)
    }

    fn oracle_world_view_invariant(&self, state: &Self::State) -> bool {
        <T as GameAuthoring>::oracle_world_view_invariant(self, state)
    }

    fn transition_postcondition(
        &self,
        pre: &Self::State,
        actions: &Self::JointActionBuf,
        post: &Self::State,
        outcome: &KernelOutcome<Self::RewardBuf>,
    ) -> bool {
        <T as GameAuthoring>::transition_postcondition(self, pre, actions, post, outcome)
    }
}
