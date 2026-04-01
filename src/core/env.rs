//! Compact environment wrapper for infotheory-compatible stepping.

use core::fmt;

use crate::buffer::{Buffer, FixedVec};
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
        self.words
            .push(word)
            .expect("bit packet capacity exceeded");
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
    /// Reward cannot be represented by the configured compact reward range.
    RewardOutOfRange {
        /// Raw out-of-range reward.
        reward: Reward,
        /// Minimum representable reward.
        min: Reward,
        /// Maximum representable reward.
        max: Reward,
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
            Self::RewardOutOfRange { reward, min, max } => {
                write!(
                    f,
                    "reward {reward} is outside compact spec range [{min}, {max}]"
                )
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
    /// Resets environment state and returns initial compact observation.
    fn reset_seed(&mut self, seed: Seed) -> Result<BitPacket<MAX_WORDS>, EnvError>;

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

/// Default environment alias with fixed history and packet capacity.
pub type DefaultEnvironment<G, const MAX_WORDS: usize = 16> =
    Environment<G, crate::session::FixedHistory<G, 256, 32, 8>, MAX_WORDS>;

impl<G, H, const MAX_WORDS: usize> Environment<G, H, MAX_WORDS>
where
    G: Observe,
    H: HistoryStore<G>,
{
    /// Creates a new compact environment.
    pub fn new(game: G, seed: Seed, observer: Observer) -> Self {
        let agent_player = match observer {
            Observer::Player(player) => player,
            Observer::Spectator => 0,
        };
        Self {
            session: SessionKernel::new(game, seed),
            observer,
            agent_player,
        }
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

    /// Steps the environment from an encoded action value.
    pub fn step(&mut self, action_bits: u64) -> Result<EnvStep<MAX_WORDS>, EnvError> {
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
            (outcome.reward_for(0), outcome.is_terminal())
        };

        let spec = self.session.game().compact_spec();
        let encoded_reward = spec.try_encode_reward(reward).map_err(|_| {
            EnvError::RewardOutOfRange {
                reward,
                min: spec.min_reward,
                max: spec.max_reward,
            }
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
        self.session.game().observe_and_encode(
            self.session.state(),
            self.observer,
            &mut encoded,
        );
        if encoded.len() > MAX_WORDS {
            return Err(EnvError::ObservationOverflow {
                actual_words: encoded.len(),
                max_words: MAX_WORDS,
            });
        }

        let mut packet = BitPacket::default();
        for &word in encoded.as_slice() {
            packet.push_word(word);
        }
        Ok(packet)
    }
}

impl<G, H, const MAX_WORDS: usize> InfotheoryEnvironment<MAX_WORDS>
    for Environment<G, H, MAX_WORDS>
where
    G: Observe,
    H: HistoryStore<G>,
{
    /// Resets environment and emits initial packet.
    fn reset_seed(&mut self, seed: Seed) -> Result<BitPacket<MAX_WORDS>, EnvError> {
        self.reset(seed)
    }

    /// Steps environment with compact action bits.
    fn step_bits(&mut self, action_bits: u64) -> Result<EnvStep<MAX_WORDS>, EnvError> {
        self.step(action_bits)
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
