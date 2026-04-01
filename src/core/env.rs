//! Compact environment wrapper for infotheory-compatible stepping.

use core::fmt;

use crate::buffer::{Buffer, FixedVec};
use crate::compact::CompactError;
use crate::core::observe::{Observe, Observer};
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

/// One environment step result with compact observation and reward.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EnvStep<const MAX_WORDS: usize> {
    /// Encoded observation packet after the step.
    pub observation_bits: BitPacket<MAX_WORDS>,
    /// Raw and compact reward representation.
    pub reward: CompactReward,
    /// True if the episode has reached terminal state.
    pub terminated: bool,
    /// True if the episode was truncated externally.
    pub truncated: bool,
}

/// Errors produced by compact environment reset/step operations.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EnvError {
    /// Step was requested after the session already terminated.
    SessionTerminated,
    /// Action bit pattern does not decode into a legal action value.
    InvalidActionEncoding {
        /// Raw encoded action word.
        encoded: u64,
    },
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
    /// Selected agent player id is outside game player range.
    InvalidAgentPlayer {
        /// Requested player id.
        player: PlayerId,
        /// Number of players exposed by the game.
        player_count: usize,
    },
}

impl fmt::Display for EnvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionTerminated => write!(f, "cannot step a terminal session"),
            Self::InvalidActionEncoding { encoded } => {
                write!(f, "invalid compact action encoding: {encoded}")
            }
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
            Self::InvalidAgentPlayer {
                player,
                player_count,
            } => write!(
                f,
                "agent player {player} is outside player range 0..{player_count}"
            ),
        }
    }
}

impl std::error::Error for EnvError {}

/// Minimal infotheory-compatible compact environment interface.
pub trait InfotheoryEnvironment<const MAX_WORDS: usize> {
    /// Parameter bundle used to initialize/reset environment state.
    type Params;

    /// Resets environment state and returns initial compact observation.
    fn reset_seed(&mut self, seed: Seed) -> Result<BitPacket<MAX_WORDS>, EnvError>;

    /// Resets environment state from explicit params and returns compact observation.
    fn reset_seed_with_params(
        &mut self,
        seed: Seed,
        params: Self::Params,
    ) -> Result<BitPacket<MAX_WORDS>, EnvError>;

    /// Steps environment using compact action bits.
    fn step_bits(&mut self, action_bits: u64) -> Result<EnvStep<MAX_WORDS>, EnvError>;
}

/// Generic environment adapter over `SessionKernel` and compact codecs.
#[derive(Clone, Debug)]
pub struct Environment<G, H, const MAX_WORDS: usize>
where
    G: Observe,
    H: HistoryStore<G>,
{
    session: SessionKernel<G, H>,
    observer: Observer,
    agent_player: PlayerId,
}

/// Default environment alias with dynamic history and packet capacity.
pub type DefaultEnvironment<G, const MAX_WORDS: usize = 16> =
    Environment<G, crate::session::DynamicHistory<G, 512, 8>, MAX_WORDS>;

impl<G, H, const MAX_WORDS: usize> Environment<G, H, MAX_WORDS>
where
    G: Observe,
    H: HistoryStore<G>,
{
    /// Creates a new compact environment initialized with explicit params.
    pub fn new_with_params(game: G, seed: Seed, observer: Observer, params: G::Params) -> Self {
        let agent_player = match observer {
            Observer::Player(player) => player,
            Observer::Spectator => 0,
        };
        Self {
            session: SessionKernel::new_with_params(game, seed, params),
            observer,
            agent_player,
        }
    }

    /// Creates a new compact environment.
    pub fn new(game: G, seed: Seed, observer: Observer) -> Self {
        let params = game.default_params();
        Self::new_with_params(game, seed, observer, params)
    }

    /// Returns immutable access to the underlying session kernel.
    pub fn session(&self) -> &SessionKernel<G, H> {
        &self.session
    }

    /// Returns mutable access to the underlying session kernel.
    pub fn session_mut(&mut self) -> &mut SessionKernel<G, H> {
        &mut self.session
    }

    /// Returns current observer viewpoint.
    pub fn observer(&self) -> Observer {
        self.observer
    }

    /// Sets observer viewpoint used for future observation encodes.
    pub fn set_observer(&mut self, observer: Observer) {
        self.observer = observer;
        if let Observer::Player(player) = observer {
            self.agent_player = player;
        }
    }

    /// Returns the player id controlled by compact `step()` actions.
    pub fn agent_player(&self) -> PlayerId {
        self.agent_player
    }

    /// Sets the player id controlled by compact `step()` actions.
    pub fn set_agent_player(&mut self, player: PlayerId) {
        self.agent_player = player;
    }

    /// Resets session state and returns initial compact observation.
    pub fn reset(&mut self, seed: Seed) -> Result<BitPacket<MAX_WORDS>, EnvError> {
        self.session.reset(seed);
        self.encode_current_observation()
    }

    /// Resets state from explicit params and returns initial compact observation.
    pub fn reset_with_params(
        &mut self,
        seed: Seed,
        params: G::Params,
    ) -> Result<BitPacket<MAX_WORDS>, EnvError> {
        self.session.reset_with_params(seed, params);
        self.encode_current_observation()
    }

    /// Steps the environment from an encoded action value.
    pub fn step(&mut self, action_bits: u64) -> Result<EnvStep<MAX_WORDS>, EnvError> {
        if self.session.is_terminal() {
            return Err(EnvError::SessionTerminated);
        }

        let Some(action) = self.session.game().decode_action(action_bits) else {
            return Err(EnvError::InvalidActionEncoding {
                encoded: action_bits,
            });
        };

        let player_count = self.session.game().player_count();
        if self.agent_player >= player_count {
            return Err(EnvError::InvalidAgentPlayer {
                player: self.agent_player,
                player_count,
            });
        }

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

        let spec = self.session.game().compact_spec();
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

        Ok(EnvStep {
            observation_bits: self.encode_current_observation()?,
            reward: CompactReward {
                raw: reward,
                encoded: encoded_reward,
            },
            terminated,
            truncated: false,
        })
    }

    /// Encodes current observation into a bounded compact packet.
    pub fn encode_current_observation(&self) -> Result<BitPacket<MAX_WORDS>, EnvError> {
        let mut encoded = G::WordBuf::default();
        self.session
            .game()
            .observe_and_encode(self.session.state(), self.observer, &mut encoded);
        if encoded.len() > MAX_WORDS {
            return Err(EnvError::ObservationOverflow {
                actual_words: encoded.len(),
                max_words: MAX_WORDS,
            });
        }
        self.session
            .game()
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

impl<G, H, const MAX_WORDS: usize> InfotheoryEnvironment<MAX_WORDS> for Environment<G, H, MAX_WORDS>
where
    G: Observe,
    H: HistoryStore<G>,
{
    type Params = G::Params;

    /// Resets environment and emits initial packet.
    fn reset_seed(&mut self, seed: Seed) -> Result<BitPacket<MAX_WORDS>, EnvError> {
        self.reset(seed)
    }

    /// Resets environment from explicit params and emits initial packet.
    fn reset_seed_with_params(
        &mut self,
        seed: Seed,
        params: Self::Params,
    ) -> Result<BitPacket<MAX_WORDS>, EnvError> {
        self.reset_with_params(seed, params)
    }

    /// Steps environment with compact action bits.
    fn step_bits(&mut self, action_bits: u64) -> Result<EnvStep<MAX_WORDS>, EnvError> {
        self.step(action_bits)
    }
}

#[cfg(test)]
mod regression_tests {
    use super::{DefaultEnvironment, EnvError, Observer};
    use crate::buffer::FixedVec;
    use crate::compact::CompactSpec;
    use crate::game::Game;
    use crate::rng::DeterministicRng;
    use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};

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

    impl Game for DemoGame {
        type Params = u8;
        type State = DemoState;
        type Action = DemoAction;
        type Obs = u8;
        type WorldView = u8;
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

        fn world_view(&self, _state: &Self::State) -> Self::WorldView {
            0
        }

        fn step_in_place(
            &self,
            state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut StepOutcome<Self::RewardBuf>,
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

        fn compact_spec(&self) -> CompactSpec {
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

    impl Game for BadObservationGame {
        type Params = ();
        type State = ();
        type Action = u8;
        type Obs = u8;
        type WorldView = ();
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

        fn world_view(&self, _state: &Self::State) -> Self::WorldView {}

        fn step_in_place(
            &self,
            _state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut StepOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 0,
                })
                .unwrap();
        }

        fn compact_spec(&self) -> CompactSpec {
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

    impl Game for BadRewardGame {
        type Params = ();
        type State = bool;
        type Action = u8;
        type Obs = u8;
        type WorldView = ();
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

        fn world_view(&self, _state: &Self::State) -> Self::WorldView {}

        fn step_in_place(
            &self,
            state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut StepOutcome<Self::RewardBuf>,
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

        fn compact_spec(&self) -> CompactSpec {
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

    #[test]
    fn step_uses_agent_player_reward() {
        let mut env = DefaultEnvironment::<DemoGame, 2>::new(DemoGame, 3, Observer::Player(0));
        env.set_agent_player(1);
        let step = env.step(0).unwrap();
        assert_eq!(step.reward.raw, 20);
        assert_eq!(step.reward.encoded, 20);
    }

    #[test]
    fn stepping_terminal_session_returns_error() {
        let mut env = DefaultEnvironment::<DemoGame, 2>::new(DemoGame, 3, Observer::Player(0));
        env.step(0).unwrap();
        assert_eq!(env.step(0), Err(EnvError::SessionTerminated));
    }

    #[test]
    fn spectator_observations_use_spectator_encoder() {
        let env = DefaultEnvironment::<DemoGame, 2>::new(DemoGame, 3, Observer::Spectator);
        let packet = env.encode_current_observation().unwrap();
        assert_eq!(packet.words(), &[299]);
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
            env.step(0),
            Err(EnvError::InvalidRewardEncoding { .. })
        ));
    }
}

#[cfg(kani)]
mod proofs {
    use super::{DefaultEnvironment, EnvError, Observer};
    use crate::buffer::FixedVec;
    use crate::compact::CompactSpec;
    use crate::game::Game;
    use crate::rng::DeterministicRng;
    use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct ObservationViolationGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct RewardBitsViolationGame;

    impl Game for ObservationViolationGame {
        type Params = ();
        type State = ();
        type Action = u8;
        type Obs = u8;
        type WorldView = ();
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

        fn world_view(&self, _state: &Self::State) -> Self::WorldView {}

        fn step_in_place(
            &self,
            _state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut StepOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 0,
                })
                .unwrap();
        }

        fn compact_spec(&self) -> CompactSpec {
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

    impl Game for RewardBitsViolationGame {
        type Params = ();
        type State = bool;
        type Action = u8;
        type Obs = u8;
        type WorldView = ();
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

        fn world_view(&self, _state: &Self::State) -> Self::WorldView {}

        fn step_in_place(
            &self,
            state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut StepOutcome<Self::RewardBuf>,
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

        fn compact_spec(&self) -> CompactSpec {
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
            env.step(0),
            Err(EnvError::InvalidRewardEncoding { .. })
        ));
    }
}

#[cfg(all(test, feature = "builtin"))]
mod tests {
    use super::{DefaultEnvironment, Observer};
    use crate::builtin::{TicTacToe, TicTacToeAction};
    use crate::game::Game;

    #[test]
    fn env_wrapper_emits_compact_observations() {
        let mut env = DefaultEnvironment::<TicTacToe, 4>::new(TicTacToe, 7, Observer::Player(0));
        let initial = env.encode_current_observation().unwrap();
        assert_eq!(initial.words(), &[0]);

        let action = TicTacToe.encode_action(&TicTacToeAction(0));
        let step = env.step(action).unwrap();
        assert_eq!(step.observation_bits.words().len(), 1);
    }
}
