use crate::buffer::Buffer;
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

    pub fn reward_range_is_sound(&self) -> bool {
        self.min_reward <= self.max_reward
            && self.encode_reward(self.min_reward) <= self.encode_reward(self.max_reward)
    }
}

pub trait CompactGame: Game {
    fn compact_spec(&self) -> CompactSpec;
    fn encode_action(&self, action: &Self::Action) -> u64;
    fn decode_action(&self, encoded: u64) -> Option<Self::Action>;
    fn encode_player_observation(
        &self,
        observation: &Self::PlayerObservation,
        out: &mut Self::WordBuf,
    );

    fn encode_spectator_observation(
        &self,
        observation: &Self::SpectatorObservation,
        out: &mut Self::WordBuf,
    ) {
        let _ = observation;
        out.clear();
    }

    fn encode_player_view(&self, state: &Self::State, player: PlayerId, out: &mut Self::WordBuf) {
        let observation = self.observe_player(state, player);
        self.encode_player_observation(&observation, out);
    }

    fn compact_invariant(&self, words: &Self::WordBuf) -> bool {
        let spec = self.compact_spec();
        if words.len() != spec.observation_stream_len {
            return false;
        }
        let max_value = spec.max_observation_value();
        let slice = words.as_slice();
        let mut index = 0usize;
        while index < slice.len() {
            if slice[index] > max_value {
                return false;
            }
            index += 1;
        }
        true
    }
}

#[cfg(kani)]
mod proofs {
    use super::CompactSpec;

    #[kani::proof]
    fn compact_reward_round_trip() {
        let spec = CompactSpec {
            action_count: 4,
            observation_bits: 6,
            observation_stream_len: 1,
            reward_bits: 5,
            min_reward: -3,
            max_reward: 11,
            reward_offset: 3,
        };
        let reward: i64 = kani::any();
        kani::assume(reward >= spec.min_reward && reward <= spec.max_reward);
        let encoded = spec.encode_reward(reward);
        assert_eq!(spec.decode_reward(encoded), reward);
        assert!(spec.reward_range_is_sound());
    }
}
