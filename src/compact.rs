//! Compact encoding specifications and validation helpers.

use crate::types::Reward;

/// Structured compact codec errors.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompactError {
    /// Reward was outside declared compact range.
    RewardOutOfRange {
        /// Input reward value.
        reward: Reward,
        /// Minimum allowed reward.
        min_reward: Reward,
        /// Maximum allowed reward.
        max_reward: Reward,
    },
    /// Encoded reward decoded outside declared compact range.
    EncodedRewardOutOfRange {
        /// Encoded compact reward value.
        encoded: u64,
        /// Minimum allowed reward.
        min_reward: Reward,
        /// Maximum allowed reward.
        max_reward: Reward,
    },
    /// Encoded action had no valid decoding.
    InvalidActionEncoding {
        /// Encoded action value.
        encoded: u64,
    },
}

/// Compact schema descriptor for action/observation/reward encoding.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CompactSpec {
    /// Number of legal compact action values.
    pub action_count: u64,
    /// Bit width of one observation word.
    pub observation_bits: u8,
    /// Number of observation words emitted per observation.
    pub observation_stream_len: usize,
    /// Bit width of encoded reward.
    pub reward_bits: u8,
    /// Minimum reward value.
    pub min_reward: Reward,
    /// Maximum reward value.
    pub max_reward: Reward,
    /// Signed offset used for reward encoding.
    pub reward_offset: Reward,
}

impl CompactSpec {
    /// Maximum representable value for one observation word.
    pub fn max_observation_value(&self) -> u64 {
        if self.observation_bits == 0 {
            0
        } else if self.observation_bits >= 64 {
            u64::MAX
        } else {
            (1u64 << self.observation_bits) - 1
        }
    }

    /// Encode reward and panic on out-of-range input.
    pub fn encode_reward(&self, reward: Reward) -> u64 {
        self.try_encode_reward(reward)
            .expect("reward out of compact range")
    }

    /// Decode reward and panic on out-of-range encoded input.
    pub fn decode_reward(&self, encoded: u64) -> Reward {
        self.try_decode_reward(encoded)
            .expect("encoded reward out of compact range")
    }

    /// Checked reward encoder.
    pub fn try_encode_reward(&self, reward: Reward) -> Result<u64, CompactError> {
        if reward < self.min_reward || reward > self.max_reward {
            return Err(CompactError::RewardOutOfRange {
                reward,
                min_reward: self.min_reward,
                max_reward: self.max_reward,
            });
        }
        let encoded = i128::from(reward) + i128::from(self.reward_offset);
        if !(0..=i128::from(u64::MAX)).contains(&encoded) {
            return Err(CompactError::RewardOutOfRange {
                reward,
                min_reward: self.min_reward,
                max_reward: self.max_reward,
            });
        }
        Ok(encoded as u64)
    }

    /// Checked reward decoder.
    pub fn try_decode_reward(&self, encoded: u64) -> Result<Reward, CompactError> {
        let decoded = i128::from(encoded) - i128::from(self.reward_offset);
        if decoded < i128::from(self.min_reward) || decoded > i128::from(self.max_reward) {
            return Err(CompactError::EncodedRewardOutOfRange {
                encoded,
                min_reward: self.min_reward,
                max_reward: self.max_reward,
            });
        }
        if decoded < i128::from(Reward::MIN) || decoded > i128::from(Reward::MAX) {
            return Err(CompactError::EncodedRewardOutOfRange {
                encoded,
                min_reward: self.min_reward,
                max_reward: self.max_reward,
            });
        }
        Ok(decoded as Reward)
    }

    /// Validate internal reward-range consistency.
    pub fn reward_range_is_sound(&self) -> bool {
        self.min_reward <= self.max_reward
            && self.try_encode_reward(self.min_reward).is_ok()
            && self.try_encode_reward(self.max_reward).is_ok()
            && self.encode_reward(self.min_reward) <= self.encode_reward(self.max_reward)
    }
}

/// Encode one finite enum action using an explicit canonical action table.
pub fn encode_enum_action<T>(action: T, action_table: &[T]) -> u64
where
    T: Copy + Eq,
{
    let mut index = 0usize;
    while index < action_table.len() {
        if action_table[index] == action {
            return index as u64;
        }
        index += 1;
    }
    panic!("action missing from compact action table");
}

/// Decode one finite enum action using an explicit canonical action table.
pub fn decode_enum_action<T>(encoded: u64, action_table: &[T]) -> Option<T>
where
    T: Copy,
{
    action_table.get(encoded as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::CompactSpec;

    #[test]
    fn try_decode_reward_rejects_large_values_without_overflow() {
        let spec = CompactSpec {
            action_count: 2,
            observation_bits: 8,
            observation_stream_len: 1,
            reward_bits: 2,
            min_reward: -1,
            max_reward: 1,
            reward_offset: 1,
        };
        assert!(spec.try_decode_reward(u64::MAX).is_err());
    }

    #[test]
    fn try_encode_reward_handles_negative_ranges() {
        let spec = CompactSpec {
            action_count: 2,
            observation_bits: 8,
            observation_stream_len: 1,
            reward_bits: 3,
            min_reward: -3,
            max_reward: 2,
            reward_offset: 3,
        };
        assert_eq!(spec.try_encode_reward(-3).unwrap(), 0);
        assert_eq!(spec.try_encode_reward(2).unwrap(), 5);
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
