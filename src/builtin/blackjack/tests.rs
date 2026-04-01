use super::*;
use crate::game::Game;
use crate::policy::{FirstLegalPolicy, RandomPolicy};
use crate::session::Session;
use crate::types::PlayerAction;
use crate::verification::{
    assert_compact_roundtrip, assert_observation_contracts, assert_transition_contracts,
};

fn state_from_hands(player: &[u8], opponent: &[u8]) -> BlackjackState {
    let mut state = BlackjackState {
        deck: [0; DECK_SIZE],
        next_card: 0,
        player_cards: [0; MAX_HAND_CARDS],
        player_len: 0,
        opponent_cards: [0; MAX_HAND_CARDS],
        opponent_len: 0,
        phase: BlackjackPhase::PlayerTurn,
        winner: None,
    };
    Blackjack::fill_deck(&mut state.deck);
    for &card in player {
        Blackjack::push_player_card(&mut state, card);
    }
    for &card in opponent {
        Blackjack::push_opponent_card(&mut state, card);
    }
    state
}

#[test]
fn hand_value_handles_soft_aces() {
    assert_eq!(
        Blackjack::evaluate_hand(&[1, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 2),
        HandValue {
            total: 21,
            soft: true,
            busted: false,
        }
    );
    assert_eq!(
        Blackjack::evaluate_hand(&[1, 1, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0], 3),
        HandValue {
            total: 21,
            soft: true,
            busted: false,
        }
    );
    assert_eq!(
        Blackjack::evaluate_hand(&[1, 1, 10, 10, 0, 0, 0, 0, 0, 0, 0, 0], 4),
        HandValue {
            total: 22,
            soft: false,
            busted: true,
        }
    );
}

#[test]
fn shuffled_deck_is_a_full_permutation() {
    let state = Blackjack.init(11);
    let mut counts = [0u8; 14];
    for card in state.deck {
        counts[card as usize] += 1;
    }
    let mut rank = 1usize;
    while rank <= 13 {
        assert_eq!(counts[rank], 4, "rank {rank} should appear four times");
        rank += 1;
    }
    assert_observation_contracts(&Blackjack, &state);
}

#[test]
fn showdown_matrix_is_correct() {
    let mut player_win = state_from_hands(&[10, 10], &[9, 9]);
    assert_eq!(Blackjack::resolve_terminal(&mut player_win), 1);
    assert_eq!(player_win.winner, Some(0));

    let mut opponent_win = state_from_hands(&[10, 8], &[10, 9]);
    assert_eq!(Blackjack::resolve_terminal(&mut opponent_win), -1);
    assert_eq!(opponent_win.winner, Some(1));

    let mut push = state_from_hands(&[10, 7], &[9, 8]);
    assert_eq!(Blackjack::resolve_terminal(&mut push), 0);
    assert_eq!(push.winner, None);
}

#[test]
fn seeded_round_trip_is_reproducible() {
    let mut left = Session::new(Blackjack, 11);
    let mut right = Session::new(Blackjack, 11);
    let action = [PlayerAction {
        player: 0,
        action: BlackjackAction::Hit,
    }];
    let left_outcome = left.step(&action).clone();
    let right_outcome = right.step(&action).clone();
    assert_eq!(left.state(), right.state());
    assert_eq!(left_outcome, right_outcome);
}

#[test]
fn verification_helpers_hold_for_player_hit() {
    let game = Blackjack;
    let state = game.init(11);
    let mut actions = FixedVec::<PlayerAction<BlackjackAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: BlackjackAction::Hit,
        })
        .unwrap();
    assert_transition_contracts(&game, &state, &actions, 11);
    assert_compact_roundtrip(&game, &BlackjackAction::Hit);
}

#[test]
fn seeded_sessions_preserve_invariants_across_policies() {
    for seed in 1..=256 {
        let mut first = FirstLegalPolicy;
        let mut random = RandomPolicy;

        let mut first_session = Session::new(Blackjack, seed);
        assert!(Blackjack.state_invariant(first_session.state()));
        let mut first_policies: [&mut dyn crate::policy::Policy<Blackjack>; 1] = [&mut first];
        while !first_session.is_terminal() && first_session.current_tick() < 16 {
            first_session.step_with_policies(&mut first_policies);
        }
        assert!(Blackjack.state_invariant(first_session.state()));

        let mut random_session = Session::new(Blackjack, seed);
        assert!(Blackjack.state_invariant(random_session.state()));
        let mut random_policies: [&mut dyn crate::policy::Policy<Blackjack>; 1] = [&mut random];
        while !random_session.is_terminal() && random_session.current_tick() < 16 {
            random_session.step_with_policies(&mut random_policies);
        }
        assert!(Blackjack.state_invariant(random_session.state()));
    }
}
