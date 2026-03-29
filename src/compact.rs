use crate::game::Game;
use crate::types::{PlayerId, Reward};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CompactSpec {
    pub action_count: u64,
    pub observation_bits: u8,
    pub observation_stream_len: usize,
    pub reward_bits: u8,
    pub min_reward: Reward,
    pub max_reward: Reward,
    pub reward_offset: Reward,
}

impl CompactSpec {
    pub fn max_observation_value(&self) -> u64 {
        if self.observation_bits == 0 {
            0
        } else if self.observation_bits >= 64 {
            u64::MAX
        } else {
            (1u64 << self.observation_bits) - 1
        }
    }

    pub fn encode_reward(&self, reward: Reward) -> u64 {
        debug_assert!(reward >= self.min_reward);
        debug_assert!(reward <= self.max_reward);
        (reward + self.reward_offset) as u64
    }

    pub fn decode_reward(&self, encoded: u64) -> Reward {
        (encoded as Reward) - self.reward_offset
    }
}

pub trait CompactGame: Game {
    fn compact_spec(&self) -> CompactSpec;
    fn encode_action(&self, action: &Self::Action) -> u64;
    fn decode_action(&self, encoded: u64) -> Option<Self::Action>;
    fn encode_player_observation(&self, observation: &Self::PlayerObservation, out: &mut Vec<u64>);

    fn encode_spectator_observation(
        &self,
        observation: &Self::SpectatorObservation,
        out: &mut Vec<u64>,
    ) {
        let _ = observation;
        out.clear();
    }

    fn encode_player_view(&self, state: &Self::State, player: PlayerId, out: &mut Vec<u64>) {
        let observation = self.observe_player(state, player);
        self.encode_player_observation(&observation, out);
    }
}
