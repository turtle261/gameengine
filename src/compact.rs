//! Compact encoding specifications and validation helpers.

use core::fmt;

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
    /// Encoded reward exceeded declared compact bit width.
    RewardEncodingExceedsBitWidth {
        /// Encoded compact reward value.
        encoded: u64,
        /// Declared compact reward bit width.
        reward_bits: u8,
    },
    /// Observation word stream length differs from declared schema.
    ObservationLengthMismatch {
        /// Actual number of observation words emitted.
        actual_len: usize,
        /// Declared number of observation words.
        expected_len: usize,
    },
    /// Observation word exceeded declared observation bit width.
    ObservationWordOutOfRange {
        /// Word index in observation stream.
        index: usize,
        /// Actual encoded word value.
        word: u64,
        /// Maximum representable word value for the schema.
        max_word: u64,
    },
    /// Encoded action had no valid decoding.
    InvalidActionEncoding {
        /// Encoded action value.
        encoded: u64,
    },
}

impl fmt::Display for CompactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RewardOutOfRange {
                reward,
                min_reward,
                max_reward,
            } => write!(
                f,
                "reward {reward} is outside compact range [{min_reward}, {max_reward}]"
            ),
            Self::EncodedRewardOutOfRange {
                encoded,
                min_reward,
                max_reward,
            } => write!(
                f,
                "encoded reward {encoded} decodes outside compact range [{min_reward}, {max_reward}]"
            ),
            Self::RewardEncodingExceedsBitWidth {
                encoded,
                reward_bits,
            } => write!(
                f,
                "encoded reward {encoded} exceeds declared reward bit width {reward_bits}"
            ),
            Self::ObservationLengthMismatch {
                actual_len,
                expected_len,
            } => write!(
                f,
                "observation stream length {actual_len} does not match declared length {expected_len}"
            ),
            Self::ObservationWordOutOfRange {
                index,
                word,
                max_word,
            } => write!(
                f,
                "observation word {index} has value {word}, exceeding schema maximum {max_word}"
            ),
            Self::InvalidActionEncoding { encoded } => {
                write!(f, "invalid action encoding {encoded}")
            }
        }
    }
}

impl std::error::Error for CompactError {}

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

    /// Maximum representable compact reward value from declared bit width.
    pub fn max_reward_value(&self) -> u64 {
        if self.reward_bits == 0 {
            0
        } else if self.reward_bits >= 64 {
            u64::MAX
        } else {
            (1u64 << self.reward_bits) - 1
        }
    }

    /// Validates one encoded reward against declared reward bit width.
    pub fn validate_encoded_reward_bits(&self, encoded: u64) -> Result<(), CompactError> {
        if encoded > self.max_reward_value() {
            return Err(CompactError::RewardEncodingExceedsBitWidth {
                encoded,
                reward_bits: self.reward_bits,
            });
        }
        Ok(())
    }

    /// Validates a full observation stream against declared shape and bit bounds.
    pub fn validate_observation_words(&self, words: &[u64]) -> Result<(), CompactError> {
        if words.len() != self.observation_stream_len {
            return Err(CompactError::ObservationLengthMismatch {
                actual_len: words.len(),
                expected_len: self.observation_stream_len,
            });
        }

        let max_word = self.max_observation_value();
        let mut index = 0usize;
        while index < words.len() {
            let word = words[index];
            if word > max_word {
                return Err(CompactError::ObservationWordOutOfRange {
                    index,
                    word,
                    max_word,
                });
            }
            index += 1;
        }
        Ok(())
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
        let encoded = encoded as u64;
        self.validate_encoded_reward_bits(encoded)?;
        Ok(encoded)
    }

    /// Checked reward decoder.
    pub fn try_decode_reward(&self, encoded: u64) -> Result<Reward, CompactError> {
        self.validate_encoded_reward_bits(encoded)?;
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
        if self.min_reward > self.max_reward {
            return false;
        }
        let Ok(min_encoded) = self.try_encode_reward(self.min_reward) else {
            return false;
        };
        let Ok(max_encoded) = self.try_encode_reward(self.max_reward) else {
            return false;
        };
        min_encoded <= max_encoded
            && self.try_decode_reward(min_encoded).ok() == Some(self.min_reward)
            && self.try_decode_reward(max_encoded).ok() == Some(self.max_reward)
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

    #[test]
    fn observation_stream_validation_catches_shape_errors() {
        let spec = CompactSpec {
            action_count: 2,
            observation_bits: 3,
            observation_stream_len: 2,
            reward_bits: 2,
            min_reward: 0,
            max_reward: 1,
            reward_offset: 0,
        };
        assert!(spec.validate_observation_words(&[1, 7]).is_ok());
        assert!(spec.validate_observation_words(&[1]).is_err());
        assert!(spec.validate_observation_words(&[1, 8]).is_err());
    }

    #[test]
    fn reward_bit_width_is_enforced() {
        let spec = CompactSpec {
            action_count: 2,
            observation_bits: 8,
            observation_stream_len: 1,
            reward_bits: 2,
            min_reward: 0,
            max_reward: 3,
            reward_offset: 0,
        };
        assert!(spec.try_decode_reward(4).is_err());
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

    #[kani::proof]
    fn compact_observation_words_match_schema() {
        let spec = CompactSpec {
            action_count: 2,
            observation_bits: 3,
            observation_stream_len: 1,
            reward_bits: 2,
            min_reward: 0,
            max_reward: 1,
            reward_offset: 0,
        };
        let word: u64 = kani::any();
        if word <= spec.max_observation_value() {
            assert!(spec.validate_observation_words(&[word]).is_ok());
        } else {
            assert!(spec.validate_observation_words(&[word]).is_err());
        }
    }

    #[kani::proof]
    fn compact_reward_bit_width_is_enforced() {
        let spec = CompactSpec {
            action_count: 2,
            observation_bits: 1,
            observation_stream_len: 1,
            reward_bits: 2,
            min_reward: 0,
            max_reward: 3,
            reward_offset: 0,
        };
        assert!(spec.try_decode_reward(4).is_err());
    }
}
