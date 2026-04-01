//! Shared card/deck helpers for card-based builtin environments.

/// A compact summary of blackjack hand value semantics.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BlackjackValue {
    /// Total score after soft-ace normalization.
    pub total: u8,
    /// Whether the hand still contains at least one soft ace.
    pub soft: bool,
    /// Whether the hand score exceeds 21.
    pub busted: bool,
}

/// Evaluate a blackjack hand from rank values in `[1, 13]`.
pub fn evaluate_blackjack_hand<const MAX_CARDS: usize>(
    cards: &[u8; MAX_CARDS],
    len: u8,
) -> BlackjackValue {
    let mut total = 0u8;
    let mut aces = 0u8;
    let limit = len as usize;
    let mut index = 0usize;
    while index < MAX_CARDS && index < limit {
        let card = cards[index];
        match card {
            1 => {
                total = total.saturating_add(11);
                aces += 1;
            }
            11..=13 => total = total.saturating_add(10),
            value => total = total.saturating_add(value),
        }
        index += 1;
    }
    for _ in 0..MAX_CARDS {
        if total <= 21 || aces == 0 {
            break;
        }
        total -= 10;
        aces -= 1;
    }
    BlackjackValue {
        total,
        soft: aces > 0,
        busted: total > 21,
    }
}

/// Fill a 52-card deck using ranks `[1, 13]` with four suits per rank.
pub fn fill_standard_deck_52(deck: &mut [u8; 52]) {
    let mut index = 0usize;
    for _ in 0..4 {
        for rank in 1..=13 {
            deck[index] = rank;
            index += 1;
        }
    }
}

/// Returns true when `deck` is a full 52-card rank multiset with four of each rank 1..=13.
pub fn is_standard_deck_52_permutation(deck: &[u8; 52]) -> bool {
    let mut counts = [0u8; 14];
    for card in deck {
        if !(1..=13).contains(card) {
            return false;
        }
        counts[*card as usize] += 1;
    }
    for count in counts.iter().skip(1) {
        if *count != 4 {
            return false;
        }
    }
    true
}

/// Pack cards as 4-bit nibbles into a single `u64`.
pub fn pack_cards_nibbles<const MAX_CARDS: usize>(cards: &[u8; MAX_CARDS], len: u8) -> u64 {
    let mut packed = 0u64;
    let limit = len as usize;
    let mut index = 0usize;
    while index < MAX_CARDS && index < limit {
        packed |= u64::from(cards[index]) << (index * 4);
        index += 1;
    }
    packed
}
