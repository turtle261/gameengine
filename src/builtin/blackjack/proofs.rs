use super::{Blackjack, BlackjackAction, BlackjackPhase, HandValue, MAX_HAND_CARDS};
use crate::buffer::FixedVec;
use crate::game::Game;
use crate::types::PlayerAction;

#[kani::proof]
#[kani::unwind(64)]
fn concrete_seed_shuffle_is_a_full_permutation() {
    let state = Blackjack.init(11);
    let mut counts = [0u8; 14];
    for card in state.deck {
        counts[card as usize] += 1;
    }
    let mut rank = 1usize;
    while rank <= 13 {
        assert_eq!(counts[rank], 4);
        rank += 1;
    }
}

#[kani::proof]
#[kani::unwind(64)]
fn player_observation_hides_opponent_hand_before_terminal() {
    let state = Blackjack.init(11);
    let observation = Blackjack.observe_player(&state, 0);
    if state.phase != BlackjackPhase::Terminal {
        assert_eq!(observation.opponent_visible_len, 0);
    }
}

#[kani::proof]
#[kani::unwind(64)]
fn initial_observation_contracts_hold_for_concrete_seed() {
    let game = Blackjack;
    let state = game.init(11);
    crate::verification::assert_observation_contracts(&game, &state);
}

#[kani::proof]
#[kani::unwind(64)]
fn stand_action_replays_deterministically_for_seed_17() {
    let state = Blackjack.init(17);
    let mut actions = FixedVec::<PlayerAction<BlackjackAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: BlackjackAction::Stand,
        })
        .unwrap();
    crate::verification::assert_transition_contracts(&Blackjack, &state, &actions, 17);
}

#[kani::proof]
#[kani::unwind(32)]
fn hand_evaluation_matches_busted_flag() {
    let len: u8 = kani::any();
    kani::assume(len <= MAX_HAND_CARDS as u8);
    let mut cards = [1u8; MAX_HAND_CARDS];
    for card in &mut cards {
        *card = kani::any();
        kani::assume((1..=13).contains(card));
    }
    let value = Blackjack::evaluate_hand(&cards, len);
    assert_eq!(
        value,
        HandValue {
            total: value.total,
            soft: value.soft,
            busted: value.total > 21,
        }
    );
}
