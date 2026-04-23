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
    let mut left = Session::new(BiasedRockPaperScissor, 19);
    let mut right = Session::new(BiasedRockPaperScissor, 19);
    let actions = [
        PlayerAction {
            player: 0,
            action: BiasedRockPaperScissorAction::Rock,
        },
        PlayerAction {
            player: 0,
            action: BiasedRockPaperScissorAction::Paper,
        },
        PlayerAction {
            player: 0,
            action: BiasedRockPaperScissorAction::Scissors,
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
fn rock_loss_bias_repeats_rock() {
    let game = BiasedRockPaperScissor;
    let mut state = BiasedRockPaperScissorState {
        opponent_action: BiasedRockPaperScissorAction::Rock,
        reward: -1,
    };
    let mut rng = DeterministicRng::from_seed_and_stream(1, 1);
    let mut outcome = KernelOutcome::<FixedVec<crate::types::PlayerReward, 1>>::default();

    let mut actions = FixedVec::<PlayerAction<BiasedRockPaperScissorAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: BiasedRockPaperScissorAction::Paper,
        })
        .unwrap();
    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);

    assert_eq!(state.opponent_action, BiasedRockPaperScissorAction::Rock);
    assert_eq!(outcome.reward_for(0), 1);
}

#[test]
fn verification_helpers_hold_for_rock_action() {
    let game = BiasedRockPaperScissor;
    let state = game.init(3);
    let mut actions = FixedVec::<PlayerAction<BiasedRockPaperScissorAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: BiasedRockPaperScissorAction::Rock,
        })
        .unwrap();
    assert_transition_contracts(&game, &state, &actions, 3);
    assert_observation_contracts(&game, &state);
    assert_compact_roundtrip(
        &game,
        &game.default_params(),
        &BiasedRockPaperScissorAction::Scissors,
    );
}
