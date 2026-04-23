use super::*;
use crate::buffer::FixedVec;
use crate::game::GameAuthoring;
use crate::session::Session;
use crate::types::{PlayerAction, PlayerReward};
use crate::verification::{
    assert_compact_roundtrip, assert_observation_contracts, assert_transition_contracts,
};

#[test]
fn standing_offsets_observation_band() {
    let game = ExtendedTiger;
    let mut state = game.init(1);
    state.observation = 2;

    let mut rng = DeterministicRng::from_seed_and_stream(1, 1);
    let mut outcome = KernelOutcome::<FixedVec<PlayerReward, 1>>::default();
    let mut actions = FixedVec::<PlayerAction<ExtendedTigerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: ExtendedTigerAction::Stand,
        })
        .unwrap();

    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert_eq!(state.posture, TigerPosture::Standing);
    assert_eq!(state.observation, 6);
    assert_eq!(outcome.reward_for(0), -1);
}

#[test]
fn opening_while_sitting_penalizes() {
    let game = ExtendedTiger;
    let mut state = game.init(3);
    state.posture = TigerPosture::Sitting;
    let before = state;

    let mut rng = DeterministicRng::from_seed_and_stream(3, 1);
    let mut outcome = KernelOutcome::<FixedVec<PlayerReward, 1>>::default();
    let mut actions = FixedVec::<PlayerAction<ExtendedTigerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: ExtendedTigerAction::OpenDoorOne,
        })
        .unwrap();

    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert_eq!(state.posture, before.posture);
    assert_eq!(state.gold_door, before.gold_door);
    assert_eq!(state.tiger_door, before.tiger_door);
    assert_eq!(outcome.reward_for(0), -100);
}

#[test]
fn door_assignment_is_two_door_tiger_problem() {
    let game = ExtendedTiger;
    for seed in 0..128 {
        let state = game.init(seed);
        assert!((1..=2).contains(&state.gold_door));
        assert!((1..=2).contains(&state.tiger_door));
        assert_ne!(state.gold_door, state.tiger_door);
        assert!(game.state_invariant(&state));
    }
}

#[test]
fn standing_open_gold_rewards_thirty_and_resets_to_sitting() {
    let game = ExtendedTiger;
    let mut state = ExtendedTigerState {
        posture: TigerPosture::Standing,
        gold_door: 1,
        tiger_door: 2,
        observation: 5,
        reward: 0,
    };
    let mut rng = DeterministicRng::from_seed_and_stream(11, 1);
    let mut outcome = KernelOutcome::<FixedVec<PlayerReward, 1>>::default();
    let mut actions = FixedVec::<PlayerAction<ExtendedTigerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: ExtendedTigerAction::OpenDoorOne,
        })
        .unwrap();

    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);

    assert_eq!(outcome.reward_for(0), 30);
    assert_eq!(state.posture, TigerPosture::Sitting);
    assert_eq!(state.observation, 0);
    assert!(game.state_invariant(&state));
}

#[test]
fn listen_only_informs_while_sitting() {
    let game = ExtendedTiger;
    let mut state = ExtendedTigerState {
        posture: TigerPosture::Sitting,
        gold_door: 1,
        tiger_door: 2,
        observation: 0,
        reward: 0,
    };
    let mut rng = DeterministicRng::from_seed_and_stream(1, 1);
    let mut outcome = KernelOutcome::<FixedVec<PlayerReward, 1>>::default();
    let mut actions = FixedVec::<PlayerAction<ExtendedTigerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: ExtendedTigerAction::Listen,
        })
        .unwrap();

    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert_eq!(outcome.reward_for(0), -1);
    assert!([state.gold_door, state.tiger_door].contains(&state.observation));

    state.posture = TigerPosture::Standing;
    state.observation = 6;
    let mut outcome = KernelOutcome::<FixedVec<PlayerReward, 1>>::default();
    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert_eq!(outcome.reward_for(0), -1);
    assert_eq!(state.observation, 0);
}

#[test]
fn seeded_sessions_replay_exactly() {
    let mut left = Session::new(ExtendedTiger, 29);
    let mut right = Session::new(ExtendedTiger, 29);
    let actions = [
        PlayerAction {
            player: 0,
            action: ExtendedTigerAction::Listen,
        },
        PlayerAction {
            player: 0,
            action: ExtendedTigerAction::Stand,
        },
        PlayerAction {
            player: 0,
            action: ExtendedTigerAction::OpenDoorOne,
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
fn verification_helpers_hold_for_listen_action() {
    let game = ExtendedTiger;
    let state = game.init(5);
    let mut actions = FixedVec::<PlayerAction<ExtendedTigerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: ExtendedTigerAction::Listen,
        })
        .unwrap();
    assert_transition_contracts(&game, &state, &actions, 5);
    assert_observation_contracts(&game, &state);
    assert_compact_roundtrip(&game, &game.default_params(), &ExtendedTigerAction::Stand);
}
