use super::*;
use crate::buffer::FixedVec;
use crate::game::GameAuthoring;
use crate::session::Session;
use crate::types::PlayerAction;
use crate::verification::{
    assert_compact_roundtrip, assert_observation_contracts, assert_transition_contracts,
};

#[test]
fn seeded_sessions_replay_exactly() {
    let mut left = Session::new(KuhnPoker, 23);
    let mut right = Session::new(KuhnPoker, 23);
    let actions = [
        PlayerAction {
            player: 0,
            action: KuhnPokerAction::Bet,
        },
        PlayerAction {
            player: 0,
            action: KuhnPokerAction::Pass,
        },
        PlayerAction {
            player: 0,
            action: KuhnPokerAction::Bet,
        },
    ];

    for action in actions {
        let left_outcome = left.step(std::slice::from_ref(&action)).clone();
        let right_outcome = right.step(std::slice::from_ref(&action)).clone();
        assert_eq!(left.state(), right.state());
        assert_eq!(left_outcome, right_outcome);
    }
}

#[test]
fn observation_encoding_matches_documented_shape() {
    let game = KuhnPoker;
    for seed in 1..=128 {
        let state = game.init(seed);
        assert!(state.observation <= 6);
        assert!(matches!(state.observation, 0..=2 | 4..=6));
        assert_observation_contracts(&game, &state);
    }
}

#[test]
fn rewards_stay_in_contract_bounds() {
    let mut session = Session::new(KuhnPoker, 7);
    for _ in 0..128 {
        let reward = session
            .step(&[PlayerAction {
                player: 0,
                action: KuhnPokerAction::Bet,
            }])
            .reward_for(0);
        assert!((-2..=2).contains(&reward));
    }
}

#[test]
fn reset_round_deals_distinct_three_card_deck_cards() {
    let game = KuhnPoker;
    for seed in 0..256 {
        let state = game.init(seed);
        assert_ne!(state.agent_card, state.opponent_card);
        assert!(game.state_invariant(&state));
    }
}

#[test]
fn passing_to_opening_bet_loses_only_ante() {
    let game = KuhnPoker;
    let mut state = KuhnPokerState {
        agent_card: KuhnPokerCard::King,
        opponent_card: KuhnPokerCard::Jack,
        opponent_action: KuhnPokerAction::Bet,
        observation: 2,
        reward: 0,
    };
    let mut rng = DeterministicRng::from_seed_and_stream(7, 1);
    let mut outcome = KernelOutcome::<FixedVec<crate::types::PlayerReward, 1>>::default();
    let mut actions = FixedVec::<PlayerAction<KuhnPokerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: KuhnPokerAction::Pass,
        })
        .unwrap();

    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert_eq!(outcome.reward_for(0), -1);
}

#[test]
fn betting_after_opponent_pass_can_win_uncontested() {
    let game = KuhnPoker;
    let mut state = KuhnPokerState {
        agent_card: KuhnPokerCard::Queen,
        opponent_card: KuhnPokerCard::Jack,
        opponent_action: KuhnPokerAction::Pass,
        observation: 5,
        reward: 0,
    };
    let mut rng = DeterministicRng::from_seed_and_stream(7, 1);
    let mut outcome = KernelOutcome::<FixedVec<crate::types::PlayerReward, 1>>::default();
    let mut actions = FixedVec::<PlayerAction<KuhnPokerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: KuhnPokerAction::Bet,
        })
        .unwrap();

    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert_eq!(outcome.reward_for(0), 1);
}

#[test]
fn verification_helpers_hold_for_opening_bet() {
    let game = KuhnPoker;
    let state = game.init(13);
    let mut actions = FixedVec::<PlayerAction<KuhnPokerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: KuhnPokerAction::Bet,
        })
        .unwrap();
    assert_transition_contracts(&game, &state, &actions, 13);
    assert_compact_roundtrip(&game, &game.default_params(), &KuhnPokerAction::Pass);
}
