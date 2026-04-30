//! AIXI-native compact environment front door and compatibility wrappers.

use core::fmt;

use crate::buffer::{Buffer, FixedVec};
use crate::compact::{CompactError, CompactSpec};
use crate::core::observe::Observer;
use crate::game::Game;
use crate::session::{HistoryStore, SessionKernel};
use crate::types::{PlayerAction, PlayerId, Reward, Seed};

/// Compact observation packet represented as fixed-capacity machine words.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct BitPacket<const MAX_WORDS: usize> {
    words: FixedVec<u64, MAX_WORDS>,
}

impl<const MAX_WORDS: usize> BitPacket<MAX_WORDS> {
    /// Returns the currently populated word slice.
    pub fn words(&self) -> &[u64] {
        self.words.as_slice()
    }

    /// Clears all packet words.
    pub fn clear(&mut self) {
        self.words.clear();
    }

    fn push_word(&mut self, word: u64) {
        self.words.push(word).expect("bit packet capacity exceeded");
    }
}

/// Reward emitted by the environment in raw and compact-encoded form.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CompactReward {
    /// Raw reward value from game semantics.
    pub raw: Reward,
    /// Compactly encoded reward value according to `CompactSpec`.
    pub encoded: u64,
}

/// One agent-facing percept token with compact observation and reward.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Percept<const MAX_WORDS: usize> {
    /// Encoded observation packet for this percept token.
    pub observation_bits: BitPacket<MAX_WORDS>,
    /// Raw and compact reward representation.
    pub reward: CompactReward,
    /// True if the percept denotes a terminal state.
    pub terminated: bool,
}

/// Backwards-compatible name for one emitted percept token.
pub type EnvStep<const MAX_WORDS: usize> = Percept<MAX_WORDS>;

/// Error returned when constructing a checked external action token.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ActionTokenError {
    /// Encoded symbol is outside the finite external action alphabet.
    OutOfAlphabet {
        /// Rejected compact action symbol.
        encoded: u64,
        /// Declared compact action alphabet size.
        action_count: u64,
    },
}

impl fmt::Display for ActionTokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfAlphabet {
                encoded,
                action_count,
            } => write!(
                f,
                "encoded action {encoded} is outside alphabet 0..{action_count}"
            ),
        }
    }
}

impl std::error::Error for ActionTokenError {}

/// Checked external action symbol consumed by the AIXI front door.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ActionToken {
    encoded: u64,
}

impl ActionToken {
    /// Creates a checked token for the given action alphabet size.
    pub fn try_new(encoded: u64, action_count: u64) -> Result<Self, ActionTokenError> {
        if encoded >= action_count {
            return Err(ActionTokenError::OutOfAlphabet {
                encoded,
                action_count,
            });
        }
        Ok(Self { encoded })
    }

    /// Creates a checked token using the current compact spec.
    pub fn from_spec(spec: &CompactSpec, encoded: u64) -> Result<Self, ActionTokenError> {
        Self::try_new(encoded, spec.action_count)
    }

    /// Returns the compact action symbol carried by this token.
    pub const fn encoded(self) -> u64 {
        self.encoded
    }
}

/// Errors produced by compact environment reset/step operations.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EnvError {
    /// Step was requested after the session already terminated.
    SessionTerminated,
    /// Observation encoding exceeded configured packet capacity.
    ObservationOverflow {
        /// Number of words requested by the game encoder.
        actual_words: usize,
        /// Maximum words accepted by this environment wrapper.
        max_words: usize,
    },
    /// Observation stream violated the compact schema constraints.
    InvalidObservationEncoding {
        /// Canonical compact constraint violation details.
        reason: CompactError,
    },
    /// Reward cannot be represented by the configured compact reward range.
    RewardOutOfRange {
        /// Raw out-of-range reward.
        reward: Reward,
        /// Minimum representable reward.
        min: Reward,
        /// Maximum representable reward.
        max: Reward,
    },
    /// Reward encoding violated compact schema constraints.
    InvalidRewardEncoding {
        /// Canonical compact constraint violation details.
        reason: CompactError,
    },
    /// Raw action bits are outside the compact action alphabet.
    InvalidActionToken {
        /// Checked token construction failure details.
        reason: ActionTokenError,
    },
    /// Parameter bundle was rejected by the game's parameter invariant.
    InvalidParameters {
        /// Stable machine-readable game name.
        game: &'static str,
    },
    /// Game compact surface violates the AIXI front-door action contract.
    InvalidAixiFrontDoor {
        /// Stable machine-readable game name.
        game: &'static str,
        /// Specific violated AIXI front-door obligation.
        reason: AixiFrontDoorError,
    },
}

/// Structured reasons why a game cannot soundly expose the AIXI front door.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AixiFrontDoorError {
    /// The compact action alphabet is empty.
    EmptyActionAlphabet,
    /// A declared in-alphabet action fails to decode to a semantic action.
    NonTotalActionDecode {
        /// Declared in-alphabet compact action symbol.
        encoded: u64,
        /// Declared compact action alphabet size.
        action_count: u64,
    },
}

impl fmt::Display for AixiFrontDoorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyActionAlphabet => f.write_str("compact action alphabet must be non-empty"),
            Self::NonTotalActionDecode {
                encoded,
                action_count,
            } => write!(
                f,
                "declared in-alphabet action {encoded} failed to decode under action_count={action_count}"
            ),
        }
    }
}

impl fmt::Display for EnvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionTerminated => write!(f, "cannot step a terminal session"),
            Self::ObservationOverflow {
                actual_words,
                max_words,
            } => {
                write!(
                    f,
                    "observation packet requires {actual_words} words but maximum is {max_words}"
                )
            }
            Self::InvalidObservationEncoding { reason } => {
                write!(f, "observation does not satisfy compact schema: {reason}")
            }
            Self::RewardOutOfRange { reward, min, max } => {
                write!(
                    f,
                    "reward {reward} is outside compact spec range [{min}, {max}]"
                )
            }
            Self::InvalidRewardEncoding { reason } => {
                write!(f, "reward does not satisfy compact schema: {reason}")
            }
            Self::InvalidActionToken { reason } => {
                write!(f, "invalid action token: {reason}")
            }
            Self::InvalidParameters { game } => {
                write!(f, "invalid parameter bundle for game `{game}`")
            }
            Self::InvalidAixiFrontDoor { game, reason } => {
                write!(
                    f,
                    "game `{game}` violates AIXI front-door contract: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for EnvError {}

/// Primary AIXI-native environment interface.
pub trait AixiEnvironment<const MAX_WORDS: usize> {
    /// Parameter bundle used to initialize/reset environment state.
    type Params;

    /// Resets environment state and returns initial percept token.
    fn reset_seed(&mut self, seed: Seed) -> Result<Percept<MAX_WORDS>, EnvError>;

    /// Resets environment state from explicit params and returns initial percept.
    fn reset_seed_with_params(
        &mut self,
        seed: Seed,
        params: Self::Params,
    ) -> Result<Percept<MAX_WORDS>, EnvError>;

    /// Steps environment using a checked external action token.
    fn step(&mut self, action: ActionToken) -> Result<Percept<MAX_WORDS>, EnvError>;
}

/// Historical compatibility alias for the same AIXI-native contract.
pub trait InfotheoryEnvironment<const MAX_WORDS: usize>: AixiEnvironment<MAX_WORDS> {}

impl<T, const MAX_WORDS: usize> InfotheoryEnvironment<MAX_WORDS> for T where
    T: AixiEnvironment<MAX_WORDS>
{
}

/// Generic environment adapter over `SessionKernel` and compact codecs.
#[derive(Clone, Debug)]
pub struct Environment<G, H, const MAX_WORDS: usize>
where
    G: Game,
    H: HistoryStore<G>,
{
    session: SessionKernel<G, H>,
    agent_player: PlayerId,
}

/// Default environment alias with dynamic history and packet capacity.
pub type DefaultEnvironment<G, const MAX_WORDS: usize = 16> =
    Environment<G, crate::session::DynamicHistory<G, 512, 8>, MAX_WORDS>;

impl<G, H, const MAX_WORDS: usize> Environment<G, H, MAX_WORDS>
where
    G: Game,
    H: HistoryStore<G>,
{
    fn validate_params(game: &G, params: &G::Params) -> Result<(), EnvError> {
        if game.params_invariant(params) {
            Ok(())
        } else {
            Err(EnvError::InvalidParameters { game: game.name() })
        }
    }

    fn validate_player(game: &G, player: PlayerId) -> Result<(), EnvError> {
        let player_count = game.player_count();
        if player >= player_count {
            return Err(EnvError::InvalidParameters { game: game.name() });
        }
        Ok(())
    }

    fn validate_aixi_front_door_for(game: &G, params: &G::Params) -> Result<(), EnvError> {
        let spec = game.compact_spec_for(params);
        if spec.action_count == 0 {
            return Err(EnvError::InvalidAixiFrontDoor {
                game: game.name(),
                reason: AixiFrontDoorError::EmptyActionAlphabet,
            });
        }

        let mut encoded = 0u64;
        while encoded < spec.action_count {
            if game.decode_action(encoded).is_none() {
                return Err(EnvError::InvalidAixiFrontDoor {
                    game: game.name(),
                    reason: AixiFrontDoorError::NonTotalActionDecode {
                        encoded,
                        action_count: spec.action_count,
                    },
                });
            }
            encoded += 1;
        }

        Ok(())
    }

    /// Creates a new compact environment initialized with explicit params and agent id.
    pub fn try_new_with_agent_params(
        game: G,
        seed: Seed,
        agent_player: PlayerId,
        params: G::Params,
    ) -> Result<Self, EnvError> {
        Self::validate_params(&game, &params)?;
        Self::validate_player(&game, agent_player)?;
        Self::validate_aixi_front_door_for(&game, &params)?;
        Ok(Self {
            session: SessionKernel::new_with_params(game, seed, params),
            agent_player,
        })
    }

    /// Creates a new compact environment initialized with explicit params and agent id.
    pub fn new_with_agent_params(
        game: G,
        seed: Seed,
        agent_player: PlayerId,
        params: G::Params,
    ) -> Self {
        Self::try_new_with_agent_params(game, seed, agent_player, params)
            .expect("invalid parameter bundle for compact environment")
    }

    /// Creates a new compact environment from an observer selector.
    ///
    /// Compatibility wrapper: `Observer::Spectator` selects agent player `0`.
    pub fn try_new_with_params(
        game: G,
        seed: Seed,
        observer: Observer,
        params: G::Params,
    ) -> Result<Self, EnvError> {
        let agent_player = match observer {
            Observer::Player(player) => player,
            Observer::Spectator => 0,
        };
        Self::try_new_with_agent_params(game, seed, agent_player, params)
    }

    /// Creates a new compact environment from an observer selector.
    ///
    /// Compatibility wrapper: `Observer::Spectator` selects agent player `0`.
    pub fn new_with_params(game: G, seed: Seed, observer: Observer, params: G::Params) -> Self {
        Self::try_new_with_params(game, seed, observer, params)
            .expect("invalid parameter bundle for compact environment")
    }

    /// Creates a new compact environment for a designated agent player.
    pub fn new_for_agent(game: G, seed: Seed, agent_player: PlayerId) -> Self {
        let params = game.default_params();
        Self::new_with_agent_params(game, seed, agent_player, params)
    }

    /// Creates a new compact environment from an observer selector.
    ///
    /// Compatibility wrapper: `Observer::Spectator` selects agent player `0`.
    pub fn new(game: G, seed: Seed, observer: Observer) -> Self {
        let params = game.default_params();
        Self::new_with_params(game, seed, observer, params)
    }

    /// Test-only immutable access to the underlying session kernel.
    ///
    /// Per specification §8 the AIXI-native `Environment` is a black-box
    /// boundary: the caller must not be able to inspect hidden runtime
    /// state, RNG internals, or replay/history handles beyond the
    /// explicit setup inputs and the emitted percept history. This
    /// accessor is therefore gated behind `#[cfg(test)]` so it is only
    /// reachable by reference regression tests that exercise the
    /// non-public kernel projection; it is not part of any public or
    /// crate-internal production code path.
    #[cfg(test)]
    pub(crate) fn session(&self) -> &SessionKernel<G, H> {
        &self.session
    }

    /// Returns the designated acting player.
    ///
    /// Per specification §8 this is the sole additional public accessor
    /// on the concrete AIXI-native `Environment` beyond the
    /// reset/step methods declared by `AixiEnvironment`.
    pub fn agent_player(&self) -> PlayerId {
        self.agent_player
    }

    fn encoded_zero_reward(&self) -> Result<CompactReward, EnvError> {
        let spec = self.session.compact_spec();
        let reward = 0i64;
        let encoded = spec
            .try_encode_reward(reward)
            .map_err(|reason| match reason {
                CompactError::RewardOutOfRange { .. } => EnvError::RewardOutOfRange {
                    reward,
                    min: spec.min_reward,
                    max: spec.max_reward,
                },
                other => EnvError::InvalidRewardEncoding { reason: other },
            })?;
        Ok(CompactReward {
            raw: reward,
            encoded,
        })
    }

    /// Resets session state and returns the initial percept token.
    pub fn reset(&mut self, seed: Seed) -> Result<Percept<MAX_WORDS>, EnvError> {
        self.session.reset(seed);
        Ok(Percept {
            observation_bits: self.encode_current_observation()?,
            reward: self.encoded_zero_reward()?,
            terminated: self.session.is_terminal(),
        })
    }

    /// Resets state from explicit params and returns the initial percept token.
    pub fn reset_with_params(
        &mut self,
        seed: Seed,
        params: G::Params,
    ) -> Result<Percept<MAX_WORDS>, EnvError> {
        Self::validate_params(self.session.game(), &params)?;
        Self::validate_aixi_front_door_for(self.session.game(), &params)?;
        self.session.reset_with_params(seed, params);
        Ok(Percept {
            observation_bits: self.encode_current_observation()?,
            reward: self.encoded_zero_reward()?,
            terminated: self.session.is_terminal(),
        })
    }

    /// Steps the environment from a checked action token.
    pub fn step(&mut self, action: ActionToken) -> Result<Percept<MAX_WORDS>, EnvError> {
        if self.session.is_terminal() {
            return Err(EnvError::SessionTerminated);
        }

        let spec = self.session.compact_spec();
        let encoded = ActionToken::from_spec(&spec, action.encoded())
            .map_err(|reason| EnvError::InvalidActionToken { reason })?
            .encoded();
        let action =
            self.session
                .game()
                .decode_action(encoded)
                .ok_or(EnvError::InvalidAixiFrontDoor {
                    game: self.session.game().name(),
                    reason: AixiFrontDoorError::NonTotalActionDecode {
                        encoded,
                        action_count: spec.action_count,
                    },
                })?;

        let mut actions = G::JointActionBuf::default();
        actions
            .push(PlayerAction {
                player: self.agent_player,
                action,
            })
            .expect("joint action buffer capacity exceeded");

        let (reward, terminated) = {
            let outcome = self.session.step_with_joint_actions(&actions);
            (outcome.reward_for(self.agent_player), outcome.is_terminal())
        };

        let spec = self.session.compact_spec();
        let encoded_reward = spec
            .try_encode_reward(reward)
            .map_err(|reason| match reason {
                CompactError::RewardOutOfRange { .. } => EnvError::RewardOutOfRange {
                    reward,
                    min: spec.min_reward,
                    max: spec.max_reward,
                },
                other => EnvError::InvalidRewardEncoding { reason: other },
            })?;

        Ok(Percept {
            observation_bits: self.encode_current_observation()?,
            reward: CompactReward {
                raw: reward,
                encoded: encoded_reward,
            },
            terminated,
        })
    }

    /// Compatibility helper that accepts raw compact action words.
    pub fn step_bits(&mut self, action_bits: u64) -> Result<Percept<MAX_WORDS>, EnvError> {
        let token = ActionToken::from_spec(&self.session.compact_spec(), action_bits)
            .map_err(|reason| EnvError::InvalidActionToken { reason })?;
        self.step(token)
    }

    /// Encodes current observation into a bounded compact packet.
    ///
    /// This helper is internal to the reset/step percept-emission path
    /// and is not part of the black-box public `Environment` surface per
    /// specification §8. It is exposed to crate-internal regression
    /// tests that exercise the compact-spec contract.
    pub(crate) fn encode_current_observation(&self) -> Result<BitPacket<MAX_WORDS>, EnvError> {
        let mut encoded = G::WordBuf::default();
        self.session.game().encode_player_view(
            self.session.state(),
            self.agent_player,
            &mut encoded,
        );
        if encoded.len() > MAX_WORDS {
            return Err(EnvError::ObservationOverflow {
                actual_words: encoded.len(),
                max_words: MAX_WORDS,
            });
        }
        self.session
            .compact_spec()
            .validate_observation_words(encoded.as_slice())
            .map_err(|reason| EnvError::InvalidObservationEncoding { reason })?;

        let mut packet = BitPacket::default();
        for &word in encoded.as_slice() {
            packet.push_word(word);
        }
        Ok(packet)
    }
}

impl<G, H, const MAX_WORDS: usize> AixiEnvironment<MAX_WORDS> for Environment<G, H, MAX_WORDS>
where
    G: Game,
    H: HistoryStore<G>,
{
    type Params = G::Params;

    /// Resets environment and emits initial percept token.
    fn reset_seed(&mut self, seed: Seed) -> Result<Percept<MAX_WORDS>, EnvError> {
        self.reset(seed)
    }

    /// Resets environment from explicit params and emits initial percept token.
    fn reset_seed_with_params(
        &mut self,
        seed: Seed,
        params: Self::Params,
    ) -> Result<Percept<MAX_WORDS>, EnvError> {
        self.reset_with_params(seed, params)
    }

    /// Steps environment using a checked action token.
    fn step(&mut self, action: ActionToken) -> Result<Percept<MAX_WORDS>, EnvError> {
        Environment::step(self, action)
    }
}

#[cfg(test)]
mod regression_tests {
    use super::{
        ActionToken, ActionTokenError, AixiFrontDoorError, DefaultEnvironment, EnvError, Observer,
    };
    use crate::buffer::FixedVec;
    use crate::compact::CompactSpec;
    use crate::game::GameAuthoring;
    use crate::rng::DeterministicRng;
    use crate::types::{KernelOutcome, PlayerAction, PlayerId, PlayerReward, Seed, Termination};

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct DemoGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct DemoState {
        terminal: bool,
        marker: u8,
    }

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    enum DemoAction {
        #[default]
        Step,
    }

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct BadObservationGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct BadRewardGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct ParamRewardGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct RejectingParamsGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct ZeroActionGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct NonTotalDecodeGame;

    impl GameAuthoring for DemoGame {
        type Params = u8;
        type State = DemoState;
        type Action = DemoAction;
        type Obs = u8;
        type PlayerBuf = FixedVec<PlayerId, 2>;
        type ActionBuf = FixedVec<DemoAction, 1>;
        type JointActionBuf = FixedVec<PlayerAction<DemoAction>, 2>;
        type RewardBuf = FixedVec<PlayerReward, 2>;
        type WordBuf = FixedVec<u64, 1>;

        fn default_params(&self) -> Self::Params {
            0
        }

        fn name(&self) -> &'static str {
            "demo"
        }

        fn player_count(&self) -> usize {
            2
        }

        fn init_with_params(&self, _seed: Seed, params: &Self::Params) -> Self::State {
            DemoState {
                terminal: false,
                marker: *params,
            }
        }

        fn is_terminal(&self, state: &Self::State) -> bool {
            state.terminal
        }

        fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            if !state.terminal {
                out.push(0).unwrap();
                out.push(1).unwrap();
            }
        }

        fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf) {
            out.clear();
            if !state.terminal && player < 2 {
                out.push(DemoAction::Step).unwrap();
            }
        }

        fn observe_player(&self, _state: &Self::State, player: PlayerId) -> Self::Obs {
            player as u8
        }

        fn observe_spectator(&self, _state: &Self::State) -> Self::Obs {
            99
        }

        fn step_in_place(
            &self,
            state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut KernelOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 10,
                })
                .unwrap();
            out.rewards
                .push(PlayerReward {
                    player: 1,
                    reward: 20,
                })
                .unwrap();
            state.terminal = true;
            out.termination = Termination::Terminal { winner: Some(0) };
        }

        fn compact_spec_for(&self, _params: &Self::Params) -> CompactSpec {
            CompactSpec {
                action_count: 1,
                observation_bits: 64,
                observation_stream_len: 1,
                reward_bits: 6,
                min_reward: 0,
                max_reward: 63,
                reward_offset: 0,
            }
        }

        fn encode_action(&self, action: &Self::Action) -> u64 {
            match action {
                DemoAction::Step => 0,
            }
        }

        fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
            (encoded == 0).then_some(DemoAction::Step)
        }

        fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            out.clear();
            out.push(100 + u64::from(*observation)).unwrap();
        }

        fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            out.clear();
            out.push(200 + u64::from(*observation)).unwrap();
        }
    }

    impl GameAuthoring for BadObservationGame {
        type Params = ();
        type State = ();
        type Action = u8;
        type Obs = u8;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 1>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn name(&self) -> &'static str {
            "bad-observation"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init_with_params(&self, _seed: Seed, _params: &Self::Params) -> Self::State {}

        fn is_terminal(&self, _state: &Self::State) -> bool {
            false
        }

        fn players_to_act(&self, _state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            out.push(0).unwrap();
        }

        fn legal_actions(
            &self,
            _state: &Self::State,
            _player: PlayerId,
            out: &mut Self::ActionBuf,
        ) {
            out.clear();
            out.push(0).unwrap();
        }

        fn observe_player(&self, _state: &Self::State, _player: PlayerId) -> Self::Obs {
            8
        }

        fn observe_spectator(&self, _state: &Self::State) -> Self::Obs {
            8
        }

        fn step_in_place(
            &self,
            _state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut KernelOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 0,
                })
                .unwrap();
        }

        fn compact_spec_for(&self, _params: &Self::Params) -> CompactSpec {
            CompactSpec {
                action_count: 1,
                observation_bits: 3,
                observation_stream_len: 1,
                reward_bits: 1,
                min_reward: 0,
                max_reward: 0,
                reward_offset: 0,
            }
        }

        fn encode_action(&self, action: &Self::Action) -> u64 {
            u64::from(*action)
        }

        fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
            (encoded == 0).then_some(0)
        }

        fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            out.clear();
            out.push(u64::from(*observation)).unwrap();
        }

        fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            self.encode_player_observation(observation, out);
        }
    }

    impl GameAuthoring for BadRewardGame {
        type Params = ();
        type State = bool;
        type Action = u8;
        type Obs = u8;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 1>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn name(&self) -> &'static str {
            "bad-reward"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init_with_params(&self, _seed: Seed, _params: &Self::Params) -> Self::State {
            false
        }

        fn is_terminal(&self, state: &Self::State) -> bool {
            *state
        }

        fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            if !*state {
                out.push(0).unwrap();
            }
        }

        fn legal_actions(&self, state: &Self::State, _player: PlayerId, out: &mut Self::ActionBuf) {
            out.clear();
            if !*state {
                out.push(0).unwrap();
            }
        }

        fn observe_player(&self, _state: &Self::State, _player: PlayerId) -> Self::Obs {
            0
        }

        fn observe_spectator(&self, _state: &Self::State) -> Self::Obs {
            0
        }

        fn step_in_place(
            &self,
            state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut KernelOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 3,
                })
                .unwrap();
            *state = true;
            out.termination = Termination::Terminal { winner: Some(0) };
        }

        fn compact_spec_for(&self, _params: &Self::Params) -> CompactSpec {
            CompactSpec {
                action_count: 1,
                observation_bits: 1,
                observation_stream_len: 1,
                reward_bits: 1,
                min_reward: 0,
                max_reward: 3,
                reward_offset: 0,
            }
        }

        fn encode_action(&self, action: &Self::Action) -> u64 {
            u64::from(*action)
        }

        fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
            (encoded == 0).then_some(0)
        }

        fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            out.clear();
            out.push(u64::from(*observation)).unwrap();
        }

        fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            self.encode_player_observation(observation, out);
        }
    }

    impl GameAuthoring for ParamRewardGame {
        type Params = u8;
        type State = u8;
        type Action = u8;
        type Obs = u8;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 1>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn default_params(&self) -> Self::Params {
            0
        }

        fn name(&self) -> &'static str {
            "param-reward"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init_with_params(&self, _seed: Seed, params: &Self::Params) -> Self::State {
            *params
        }

        fn is_terminal(&self, _state: &Self::State) -> bool {
            false
        }

        fn players_to_act(&self, _state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            out.push(0).unwrap();
        }

        fn legal_actions(
            &self,
            _state: &Self::State,
            _player: PlayerId,
            out: &mut Self::ActionBuf,
        ) {
            out.clear();
            out.push(0).unwrap();
        }

        fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::Obs {
            *state
        }

        fn observe_spectator(&self, state: &Self::State) -> Self::Obs {
            *state
        }

        fn step_in_place(
            &self,
            state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut KernelOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: i64::from(*state),
                })
                .unwrap();
        }

        fn compact_spec_for(&self, params: &Self::Params) -> CompactSpec {
            let max_reward = i64::from(*params);
            let reward_bits = if max_reward == 0 {
                1
            } else {
                (u64::BITS - (max_reward as u64).leading_zeros()) as u8
            };
            CompactSpec {
                action_count: 1,
                observation_bits: 8,
                observation_stream_len: 1,
                reward_bits,
                min_reward: 0,
                max_reward,
                reward_offset: 0,
            }
        }

        fn encode_action(&self, action: &Self::Action) -> u64 {
            u64::from(*action)
        }

        fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
            (encoded == 0).then_some(0)
        }

        fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            out.clear();
            out.push(u64::from(*observation)).unwrap();
        }

        fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            self.encode_player_observation(observation, out);
        }
    }

    impl GameAuthoring for RejectingParamsGame {
        type Params = i32;
        type State = i32;
        type Action = u8;
        type Obs = i32;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 1>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn default_params(&self) -> Self::Params {
            0
        }

        fn name(&self) -> &'static str {
            "rejecting-params"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn params_invariant(&self, params: &Self::Params) -> bool {
            *params >= 0
        }

        fn init_with_params(&self, _seed: Seed, params: &Self::Params) -> Self::State {
            assert!(*params >= 0);
            *params
        }

        fn is_terminal(&self, _state: &Self::State) -> bool {
            false
        }

        fn players_to_act(&self, _state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            out.push(0).unwrap();
        }

        fn legal_actions(
            &self,
            _state: &Self::State,
            _player: PlayerId,
            out: &mut Self::ActionBuf,
        ) {
            out.clear();
            out.push(0).unwrap();
        }

        fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::Obs {
            *state
        }

        fn observe_spectator(&self, state: &Self::State) -> Self::Obs {
            *state
        }

        fn step_in_place(
            &self,
            _state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut KernelOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 0,
                })
                .unwrap();
        }

        fn compact_spec_for(&self, _params: &Self::Params) -> CompactSpec {
            CompactSpec {
                action_count: 1,
                observation_bits: 8,
                observation_stream_len: 1,
                reward_bits: 1,
                min_reward: 0,
                max_reward: 0,
                reward_offset: 0,
            }
        }

        fn encode_action(&self, action: &Self::Action) -> u64 {
            u64::from(*action)
        }

        fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
            (encoded == 0).then_some(0)
        }

        fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            out.clear();
            out.push(*observation as u64).unwrap();
        }

        fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            self.encode_player_observation(observation, out);
        }
    }

    impl GameAuthoring for ZeroActionGame {
        type Params = ();
        type State = ();
        type Action = u8;
        type Obs = u8;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 1>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn name(&self) -> &'static str {
            "zero-action"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init_with_params(&self, _seed: Seed, _params: &Self::Params) -> Self::State {}

        fn is_terminal(&self, _state: &Self::State) -> bool {
            false
        }

        fn players_to_act(&self, _state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            out.push(0).unwrap();
        }

        fn legal_actions(
            &self,
            _state: &Self::State,
            _player: PlayerId,
            out: &mut Self::ActionBuf,
        ) {
            out.clear();
        }

        fn observe_player(&self, _state: &Self::State, _player: PlayerId) -> Self::Obs {
            0
        }

        fn observe_spectator(&self, _state: &Self::State) -> Self::Obs {
            0
        }

        fn step_in_place(
            &self,
            _state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut KernelOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 0,
                })
                .unwrap();
        }

        fn compact_spec_for(&self, _params: &Self::Params) -> CompactSpec {
            CompactSpec {
                action_count: 0,
                observation_bits: 1,
                observation_stream_len: 1,
                reward_bits: 1,
                min_reward: 0,
                max_reward: 0,
                reward_offset: 0,
            }
        }
    }

    impl GameAuthoring for NonTotalDecodeGame {
        type Params = u8;
        type State = u8;
        type Action = u8;
        type Obs = u8;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 2>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn default_params(&self) -> Self::Params {
            1
        }

        fn name(&self) -> &'static str {
            "non-total-decode"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init_with_params(&self, _seed: Seed, params: &Self::Params) -> Self::State {
            *params
        }

        fn is_terminal(&self, _state: &Self::State) -> bool {
            false
        }

        fn players_to_act(&self, _state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            out.push(0).unwrap();
        }

        fn legal_actions(
            &self,
            _state: &Self::State,
            _player: PlayerId,
            out: &mut Self::ActionBuf,
        ) {
            out.clear();
            out.push(0).unwrap();
            out.push(1).unwrap();
        }

        fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::Obs {
            *state
        }

        fn observe_spectator(&self, state: &Self::State) -> Self::Obs {
            *state
        }

        fn step_in_place(
            &self,
            _state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut KernelOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 0,
                })
                .unwrap();
        }

        fn compact_spec_for(&self, params: &Self::Params) -> CompactSpec {
            CompactSpec {
                action_count: u64::from(*params),
                observation_bits: 1,
                observation_stream_len: 1,
                reward_bits: 1,
                min_reward: 0,
                max_reward: 0,
                reward_offset: 0,
            }
        }

        fn encode_action(&self, action: &Self::Action) -> u64 {
            u64::from(*action)
        }

        fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
            (encoded == 0).then_some(0)
        }

        fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            out.clear();
            out.push(u64::from(*observation)).unwrap();
        }

        fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            self.encode_player_observation(observation, out);
        }
    }

    #[test]
    fn step_uses_agent_player_reward() {
        let mut env = DefaultEnvironment::<DemoGame, 2>::new_for_agent(DemoGame, 3, 1);
        let step = env.step_bits(0).unwrap();
        assert_eq!(step.reward.raw, 20);
        assert_eq!(step.reward.encoded, 20);
    }

    #[test]
    fn reset_emits_initial_percept_with_zero_reward() {
        let mut env = DefaultEnvironment::<DemoGame, 2>::new(DemoGame, 3, Observer::Player(0));
        let initial = env.reset(3).unwrap();
        assert_eq!(initial.reward.raw, 0);
        assert_eq!(initial.reward.encoded, 0);
        assert!(!initial.terminated);
    }

    #[test]
    fn out_of_alphabet_action_bits_are_rejected_by_token_gate() {
        let mut env = DefaultEnvironment::<DemoGame, 2>::new(DemoGame, 3, Observer::Player(0));
        let spec = env.session().compact_spec();
        assert!(matches!(
            ActionToken::from_spec(&spec, 1),
            Err(ActionTokenError::OutOfAlphabet { .. })
        ));
        assert!(matches!(
            env.step_bits(1),
            Err(EnvError::InvalidActionToken {
                reason: ActionTokenError::OutOfAlphabet {
                    encoded: 1,
                    action_count: 1
                }
            })
        ));
    }

    #[test]
    fn constructors_reject_empty_action_alphabet() {
        assert!(matches!(
            DefaultEnvironment::<ZeroActionGame, 1>::try_new_with_agent_params(
                ZeroActionGame,
                1,
                0,
                (),
            ),
            Err(EnvError::InvalidAixiFrontDoor {
                game: "zero-action",
                reason: AixiFrontDoorError::EmptyActionAlphabet,
            })
        ));
    }

    #[test]
    fn constructors_reject_non_total_decoding_surface() {
        assert!(matches!(
            DefaultEnvironment::<NonTotalDecodeGame, 1>::try_new_with_agent_params(
                NonTotalDecodeGame,
                1,
                0,
                2,
            ),
            Err(EnvError::InvalidAixiFrontDoor {
                game: "non-total-decode",
                reason: AixiFrontDoorError::NonTotalActionDecode {
                    encoded: 1,
                    action_count: 2,
                },
            })
        ));
    }

    #[test]
    fn reset_with_params_rejects_invalid_aixi_surface() {
        let mut env = DefaultEnvironment::<NonTotalDecodeGame, 1>::new(
            NonTotalDecodeGame,
            1,
            Observer::Player(0),
        );
        assert_eq!(
            env.reset_with_params(7, 2),
            Err(EnvError::InvalidAixiFrontDoor {
                game: "non-total-decode",
                reason: AixiFrontDoorError::NonTotalActionDecode {
                    encoded: 1,
                    action_count: 2,
                },
            })
        );
    }

    #[test]
    fn step_rechecks_token_against_current_compact_spec() {
        let mut env = DefaultEnvironment::<NonTotalDecodeGame, 1>::new(
            NonTotalDecodeGame,
            1,
            Observer::Player(0),
        );
        env.reset_with_params(5, 1).unwrap();
        let foreign_token = ActionToken::try_new(1, 2).expect("foreign token");
        assert_eq!(
            env.step(foreign_token),
            Err(EnvError::InvalidActionToken {
                reason: ActionTokenError::OutOfAlphabet {
                    encoded: 1,
                    action_count: 1,
                },
            })
        );
    }

    #[test]
    fn stepping_terminal_session_returns_error() {
        let mut env = DefaultEnvironment::<DemoGame, 2>::new(DemoGame, 3, Observer::Player(0));
        env.step_bits(0).unwrap();
        assert_eq!(env.step_bits(0), Err(EnvError::SessionTerminated));
    }

    #[test]
    fn spectator_observations_use_spectator_encoder() {
        let env = DefaultEnvironment::<DemoGame, 2>::new(DemoGame, 3, Observer::Spectator);
        let packet = env.encode_current_observation().unwrap();
        assert_eq!(packet.words(), &[100]);
    }

    #[test]
    fn reset_with_params_updates_session_seed_params_state() {
        let mut env = DefaultEnvironment::<DemoGame, 2>::new(DemoGame, 3, Observer::Player(0));
        assert_eq!(env.session().state().marker, 0);
        env.reset_with_params(11, 42).unwrap();
        assert_eq!(env.session().current_tick(), 0);
        assert_eq!(env.session().state().marker, 42);
    }

    #[test]
    fn observation_schema_violations_are_rejected() {
        let env = DefaultEnvironment::<BadObservationGame, 1>::new(
            BadObservationGame,
            1,
            Observer::Player(0),
        );
        assert!(matches!(
            env.encode_current_observation(),
            Err(EnvError::InvalidObservationEncoding { .. })
        ));
    }

    #[test]
    fn reward_bit_width_violations_are_rejected() {
        let mut env =
            DefaultEnvironment::<BadRewardGame, 1>::new(BadRewardGame, 1, Observer::Player(0));
        assert!(matches!(
            env.step_bits(0),
            Err(EnvError::InvalidRewardEncoding { .. })
        ));
    }

    #[test]
    fn observation_rejects_out_of_range_player_observer() {
        assert!(matches!(
            DefaultEnvironment::<DemoGame, 2>::try_new_with_agent_params(DemoGame, 3, 7, 0),
            Err(EnvError::InvalidParameters { game: "demo" })
        ));
    }

    #[test]
    fn reward_encoding_uses_active_session_params() {
        let mut env =
            DefaultEnvironment::<ParamRewardGame, 1>::new(ParamRewardGame, 1, Observer::Player(0));
        env.reset_with_params(1, 5).unwrap();
        let step = env.step_bits(0).unwrap();
        assert_eq!(step.reward.raw, 5);
        assert_eq!(step.reward.encoded, 5);
    }

    #[test]
    fn reset_with_invalid_params_returns_error() {
        let mut env = DefaultEnvironment::<RejectingParamsGame, 1>::new(
            RejectingParamsGame,
            1,
            Observer::Player(0),
        );
        assert_eq!(
            env.reset_with_params(1, -1),
            Err(EnvError::InvalidParameters {
                game: "rejecting-params"
            })
        );
    }

    #[test]
    fn try_new_with_invalid_params_returns_error() {
        assert!(matches!(
            DefaultEnvironment::<RejectingParamsGame, 1>::try_new_with_params(
                RejectingParamsGame,
                1,
                Observer::Player(0),
                -1,
            ),
            Err(EnvError::InvalidParameters {
                game: "rejecting-params"
            })
        ));
    }
}

#[cfg(kani)]
mod proofs {
    use super::{DefaultEnvironment, EnvError, Observer};
    use crate::buffer::FixedVec;
    use crate::compact::CompactSpec;
    use crate::game::GameAuthoring;
    use crate::rng::DeterministicRng;
    use crate::types::{KernelOutcome, PlayerAction, PlayerId, PlayerReward, Seed, Termination};

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct ObservationViolationGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct RewardBitsViolationGame;

    impl GameAuthoring for ObservationViolationGame {
        type Params = ();
        type State = ();
        type Action = u8;
        type Obs = u8;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 1>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn name(&self) -> &'static str {
            "observation-violation"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init_with_params(&self, _seed: Seed, _params: &Self::Params) -> Self::State {}

        fn is_terminal(&self, _state: &Self::State) -> bool {
            false
        }

        fn players_to_act(&self, _state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            out.push(0).unwrap();
        }

        fn legal_actions(
            &self,
            _state: &Self::State,
            _player: PlayerId,
            out: &mut Self::ActionBuf,
        ) {
            out.clear();
            out.push(0).unwrap();
        }

        fn observe_player(&self, _state: &Self::State, _player: PlayerId) -> Self::Obs {
            8
        }

        fn observe_spectator(&self, _state: &Self::State) -> Self::Obs {
            8
        }

        fn step_in_place(
            &self,
            _state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut KernelOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 0,
                })
                .unwrap();
        }

        fn compact_spec_for(&self, _params: &Self::Params) -> CompactSpec {
            CompactSpec {
                action_count: 1,
                observation_bits: 3,
                observation_stream_len: 1,
                reward_bits: 1,
                min_reward: 0,
                max_reward: 0,
                reward_offset: 0,
            }
        }

        fn encode_action(&self, action: &Self::Action) -> u64 {
            u64::from(*action)
        }

        fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
            (encoded == 0).then_some(0)
        }

        fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            out.clear();
            out.push(u64::from(*observation)).unwrap();
        }

        fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            self.encode_player_observation(observation, out);
        }
    }

    impl GameAuthoring for RewardBitsViolationGame {
        type Params = ();
        type State = bool;
        type Action = u8;
        type Obs = u8;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 1>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn name(&self) -> &'static str {
            "reward-violation"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init_with_params(&self, _seed: Seed, _params: &Self::Params) -> Self::State {
            false
        }

        fn is_terminal(&self, state: &Self::State) -> bool {
            *state
        }

        fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            if !*state {
                out.push(0).unwrap();
            }
        }

        fn legal_actions(&self, state: &Self::State, _player: PlayerId, out: &mut Self::ActionBuf) {
            out.clear();
            if !*state {
                out.push(0).unwrap();
            }
        }

        fn observe_player(&self, _state: &Self::State, _player: PlayerId) -> Self::Obs {
            0
        }

        fn observe_spectator(&self, _state: &Self::State) -> Self::Obs {
            0
        }

        fn step_in_place(
            &self,
            state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut KernelOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 3,
                })
                .unwrap();
            *state = true;
            out.termination = Termination::Terminal { winner: Some(0) };
        }

        fn compact_spec_for(&self, _params: &Self::Params) -> CompactSpec {
            CompactSpec {
                action_count: 1,
                observation_bits: 1,
                observation_stream_len: 1,
                reward_bits: 1,
                min_reward: 0,
                max_reward: 3,
                reward_offset: 0,
            }
        }

        fn encode_action(&self, action: &Self::Action) -> u64 {
            u64::from(*action)
        }

        fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
            (encoded == 0).then_some(0)
        }

        fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            out.clear();
            out.push(u64::from(*observation)).unwrap();
        }

        fn encode_spectator_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
            self.encode_player_observation(observation, out);
        }
    }

    #[kani::proof]
    fn env_rejects_invalid_observation_words() {
        let env = DefaultEnvironment::<ObservationViolationGame, 1>::new(
            ObservationViolationGame,
            1,
            Observer::Player(0),
        );
        assert!(matches!(
            env.encode_current_observation(),
            Err(EnvError::InvalidObservationEncoding { .. })
        ));
    }

    #[kani::proof]
    fn env_rejects_reward_encoding_that_exceeds_bit_width() {
        let mut env = DefaultEnvironment::<RewardBitsViolationGame, 1>::new(
            RewardBitsViolationGame,
            1,
            Observer::Player(0),
        );
        assert!(matches!(
            env.step_bits(0),
            Err(EnvError::InvalidRewardEncoding { .. })
        ));
    }
}

#[cfg(all(test, feature = "builtin"))]
mod tests {
    use super::{DefaultEnvironment, Observer};
    use crate::builtin::{TicTacToe, TicTacToeAction};
    use crate::game::GameAuthoring;

    #[test]
    fn env_wrapper_emits_compact_observations() {
        let mut env = DefaultEnvironment::<TicTacToe, 4>::new(TicTacToe, 7, Observer::Player(0));
        let initial = env.encode_current_observation().unwrap();
        assert_eq!(initial.words(), &[0]);

        let action = TicTacToe.encode_action(&TicTacToeAction(0));
        let step = env.step_bits(action).unwrap();
        assert_eq!(step.observation_bits.words().len(), 1);
    }
}
