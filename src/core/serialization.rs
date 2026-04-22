//! Canonical external history serialization as defined by the specification.
//!
//! This module implements the \u{00a7}8.2 bitstream surface: the reversible encoding
//! of action tokens, percepts, and full initialized histories into a
//! least-significant-bit-first bitstring. Binary predictors such as FAC-CTW
//! consume these bits directly; the incidental \code{u64}-packet transport view
//! is a representational artifact and is not the normative history object.
//!
//! All width arithmetic follows `CompactSpec` derived quantities
//! (`action_bits`, `observation_word_bits`, `reward_word_bits`), with
//! width-zero fields contributing the empty bitstring.

use crate::compact::CompactSpec;
use crate::core::env::{ActionToken, Percept};

/// Growable bitstream that packs bits least-significant-bit first.
///
/// Bit index `i` of the stream lands in byte `i / 8`, at intra-byte bit
/// position `i % 8` (i.e. bit `1 << (i % 8)`). Empty streams carry a
/// zero-length byte buffer and zero bit count.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BitStream {
    bytes: Vec<u8>,
    bit_len: usize,
}

impl BitStream {
    /// Creates a fresh empty bitstream.
    #[inline]
    pub const fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_len: 0,
        }
    }

    /// Creates an empty bitstream with capacity for at least `bit_capacity` bits.
    pub fn with_bit_capacity(bit_capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(bit_capacity.div_ceil(8)),
            bit_len: 0,
        }
    }

    /// Number of bits written so far.
    #[inline]
    pub const fn bit_len(&self) -> usize {
        self.bit_len
    }

    /// Returns `true` when the stream is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.bit_len == 0
    }

    /// Returns the packed byte buffer. Trailing bits in the final byte beyond
    /// `bit_len() % 8` are guaranteed to be zero.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Clears the stream in place, preserving allocated byte capacity.
    pub fn clear(&mut self) {
        self.bytes.clear();
        self.bit_len = 0;
    }

    /// Returns the bit at position `index` (panics if out of range).
    pub fn get_bit(&self, index: usize) -> bool {
        assert!(
            index < self.bit_len,
            "bit index {index} out of range for stream of length {}",
            self.bit_len,
        );
        let byte: u8 = self.bytes[index / 8];
        (byte >> (index % 8)) & 1 == 1
    }

    /// Appends a single bit.
    pub fn push_bit(&mut self, bit: bool) {
        let bit_index: usize = self.bit_len;
        if bit_index.is_multiple_of(8) {
            self.bytes.push(0);
        }
        if bit {
            let slot: &mut u8 = &mut self.bytes[bit_index / 8];
            *slot |= 1u8 << (bit_index % 8);
        }
        self.bit_len = bit_index + 1;
    }

    /// Appends the `width`-bit LSB-first expansion of `value`.
    ///
    /// Realizes `bits_w(v) = b_0 b_1 .. b_{w-1}` with
    /// `v = sum_{i=0}^{w-1} b_i 2^i`. A `width` of 0 contributes no bits and
    /// `value` is only inspected within its low `width` bits; bits at positions
    /// `>= width` must be zero when `width < 64`.
    pub fn push_bits(&mut self, value: u64, width: u8) {
        debug_assert!(
            width >= 64 || value >> width == 0,
            "value {value} does not fit within {width} bits",
        );
        let width_usize: usize = width as usize;
        let mut i: usize = 0;
        while i < width_usize {
            self.push_bit((value >> i) & 1 == 1);
            i += 1;
        }
    }

    /// Returns an iterator over the stream's bits in order.
    pub fn iter_bits(&self) -> BitStreamBits<'_> {
        BitStreamBits {
            stream: self,
            next: 0,
        }
    }
}

/// Iterator over an existing [`BitStream`]'s bits.
#[derive(Clone, Debug)]
pub struct BitStreamBits<'a> {
    stream: &'a BitStream,
    next: usize,
}

impl<'a> Iterator for BitStreamBits<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        if self.next >= self.stream.bit_len {
            return None;
        }
        let bit: bool = self.stream.get_bit(self.next);
        self.next += 1;
        Some(bit)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining: usize = self.stream.bit_len - self.next;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for BitStreamBits<'a> {}

/// Appends `ser(a)` = `bits_{action_bits}(encoded)` to `out`.
///
/// The width is taken from `spec.action_bits()` and must satisfy
/// `token.encoded() < spec.action_count` (as enforced at construction time).
pub fn serialize_action(out: &mut BitStream, spec: &CompactSpec, token: ActionToken) {
    out.push_bits(token.encoded(), spec.action_bits());
}

/// Appends `ser(x)` = `ser_O ser_R bit(t)` to `out`.
///
/// Observation words are written in increasing packet index order at width
/// `spec.observation_word_bits()`. The compact reward encoding is written at
/// width `spec.reward_word_bits()`. A single terminal bit is appended last.
///
/// The percept must have been produced by or be consistent with `spec`; in
/// particular `percept.observation_bits.words().len()` should equal
/// `spec.observation_stream_len`. The function is defensive: if fewer words
/// are present, missing words serialize as zero.
pub fn serialize_percept<const MAX_WORDS: usize>(
    out: &mut BitStream,
    spec: &CompactSpec,
    percept: &Percept<MAX_WORDS>,
) {
    let word_width: u8 = spec.observation_word_bits();
    let reward_width: u8 = spec.reward_word_bits();
    let stream_len: usize = spec.observation_stream_len;
    let words: &[u64] = percept.observation_bits.words();
    let mut index: usize = 0;
    while index < stream_len {
        let value: u64 = if index < words.len() { words[index] } else { 0 };
        out.push_bits(value, word_width);
        index += 1;
    }
    out.push_bits(percept.reward.encoded, reward_width);
    out.push_bit(percept.terminated);
}

/// Serializes a full history `(x_0, a_1, x_1, a_2, x_2, ..., a_n, x_n)`.
///
/// When `pending_action` is `Some(a)`, the history is pending initialized
/// through that trailing action token: the concatenation is truncated after
/// `ser(a_n)` as per the specification. Otherwise, the history is complete
/// initialized and ends in the last percept's terminal bit.
pub fn serialize_history<'p, const MAX_WORDS: usize, I>(
    out: &mut BitStream,
    spec: &CompactSpec,
    reset_percept: &Percept<MAX_WORDS>,
    action_percept_pairs: I,
    pending_action: Option<ActionToken>,
) where
    I: IntoIterator<Item = (ActionToken, &'p Percept<MAX_WORDS>)>,
{
    serialize_percept(out, spec, reset_percept);
    for (action, percept) in action_percept_pairs {
        serialize_action(out, spec, action);
        serialize_percept(out, spec, percept);
    }
    if let Some(action) = pending_action {
        serialize_action(out, spec, action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits_of(stream: &BitStream) -> Vec<bool> {
        stream.iter_bits().collect()
    }

    #[test]
    fn bits_w_zero_width_contributes_nothing() {
        let mut stream: BitStream = BitStream::new();
        stream.push_bits(0, 0);
        assert_eq!(stream.bit_len(), 0);
        assert!(stream.is_empty());
    }

    #[test]
    #[should_panic]
    fn bits_w_rejects_value_exceeding_width() {
        let mut stream: BitStream = BitStream::new();
        stream.push_bits(u64::MAX, 0);
    }

    #[test]
    fn bits_w_is_lsb_first() {
        let mut stream: BitStream = BitStream::new();
        // 0b0000_1011 with width 5 -> b_0 b_1 b_2 b_3 b_4 = 1 1 0 1 0
        stream.push_bits(0b0000_1011, 5);
        assert_eq!(bits_of(&stream), vec![true, true, false, true, false]);
    }

    #[test]
    fn bits_w_packs_into_bytes_lsb_first() {
        let mut stream: BitStream = BitStream::new();
        stream.push_bits(0x12, 8); // 0001_0010 -> LSB first: 0 1 0 0 1 0 0 0
        stream.push_bits(0x34, 8); // 0011_0100 -> LSB first: 0 0 1 0 1 1 0 0
        assert_eq!(stream.bit_len(), 16);
        assert_eq!(stream.as_bytes(), &[0x12, 0x34]);
    }

    #[test]
    fn bits_w_zero_pads_partial_final_byte() {
        let mut stream: BitStream = BitStream::new();
        stream.push_bits(0b101, 3);
        stream.push_bit(true);
        assert_eq!(stream.bit_len(), 4);
        assert_eq!(stream.as_bytes(), &[0b0000_1101]);
    }

    #[test]
    fn serialize_action_uses_action_bits_width() {
        // action_count=9 -> action_bits = ceil(log2(9)) = 4
        let spec: CompactSpec = CompactSpec {
            action_count: 9,
            observation_bits: 0,
            observation_stream_len: 0,
            reward_bits: 1,
            min_reward: 0,
            max_reward: 0,
            reward_offset: 0,
        };
        assert_eq!(spec.action_bits(), 4);
        let token: ActionToken = ActionToken::try_new(5, 9).unwrap();
        let mut stream: BitStream = BitStream::new();
        serialize_action(&mut stream, &spec, token);
        // 5 = 0b0101 -> LSB-first: 1 0 1 0
        assert_eq!(bits_of(&stream), vec![true, false, true, false]);
    }

    #[test]
    fn serialize_action_width_zero_is_empty_for_singleton_alphabet() {
        let spec: CompactSpec = CompactSpec {
            action_count: 1,
            observation_bits: 0,
            observation_stream_len: 0,
            reward_bits: 1,
            min_reward: 0,
            max_reward: 0,
            reward_offset: 0,
        };
        assert_eq!(spec.action_bits(), 0);
        let token: ActionToken = ActionToken::try_new(0, 1).unwrap();
        let mut stream: BitStream = BitStream::new();
        serialize_action(&mut stream, &spec, token);
        assert!(stream.is_empty());
    }
}
