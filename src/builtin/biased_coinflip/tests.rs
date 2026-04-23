use super::*;
use crate::game::GameAuthoring;
use crate::session::Session;
use crate::types::PlayerAction;
use crate::verification::{
    assert_compact_roundtrip, assert_observation_contracts, assert_transition_contracts,
};

#[test]
fn seeded_sessions_replay_exactly() {
    let game = BiasedCoinFlip::default();
    let mut left = Session::new(game, 11);
    let mut right = Session::new(game, 11);
    let actions = [
        PlayerAction {
            player: 0,
            action: BiasedCoinFlipAction::GuessHeads,
        },
        PlayerAction {
            player: 0,
            action: BiasedCoinFlipAction::GuessTails,
        },
        PlayerAction {
            player: 0,
            action: BiasedCoinFlipAction::GuessHeads,
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
fn deterministic_bias_rewards_matching_guess() {
    let config = BiasedCoinFlipConfig {
        head_numerator: 1,
        head_denominator: 1,
    };
    let game = BiasedCoinFlip::new(config);
    let mut session = Session::new(game, 1);

    let reward = session
        .step(&[PlayerAction {
            player: 0,
            action: BiasedCoinFlipAction::GuessHeads,
        }])
        .reward_for(0);
    assert_eq!(reward, 1);
}

#[test]
fn zero_bias_always_samples_tails() {
    let game = BiasedCoinFlip::new(BiasedCoinFlipConfig {
        head_numerator: 0,
        head_denominator: 1,
    });
    let mut session = Session::new(game, 9);

    for _ in 0..16 {
        let reward = session
            .step(&[PlayerAction {
                player: 0,
                action: BiasedCoinFlipAction::GuessTails,
            }])
            .reward_for(0);
        assert_eq!(session.state().coin_face, 0);
        assert_eq!(reward, 1);
    }
}

#[test]
fn invalid_bias_is_rejected_by_invariant() {
    assert!(
        !BiasedCoinFlipConfig {
            head_numerator: 2,
            head_denominator: 1
        }
        .invariant()
    );
    assert!(
        !BiasedCoinFlipConfig {
            head_numerator: 0,
            head_denominator: 0
        }
        .invariant()
    );
}

#[test]
fn verification_helpers_hold_for_binary_guess() {
    let game = BiasedCoinFlip::default();
    let state = game.init(9);
    let mut actions = FixedVec::<PlayerAction<BiasedCoinFlipAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: BiasedCoinFlipAction::GuessHeads,
        })
        .unwrap();

    assert_transition_contracts(&game, &state, &actions, 9);
    assert_observation_contracts(&game, &state);
    assert_compact_roundtrip(
        &game,
        &game.default_params(),
        &BiasedCoinFlipAction::GuessHeads,
    );
}
