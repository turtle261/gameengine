use super::*;
use crate::game::Game;
use crate::session::Session;
use crate::types::{PlayerAction, PlayerReward};
use crate::verification::{
    assert_compact_roundtrip, assert_observation_contracts, assert_transition_contracts,
};

#[test]
fn movement_clamps_at_walls() {
    let game = Platformer::default();
    let mut state = game.init(1);
    let mut rng = DeterministicRng::from_seed_and_stream(1, 1);
    let mut outcome = StepOutcome::<FixedVec<PlayerReward, 1>>::default();
    let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: PlatformerAction::Left,
        })
        .unwrap();
    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert_eq!(game.observe_spectator(&state).x, 0);

    state
        .world
        .set_body_position(PLAYER_BODY_ID, game.config.player_center(11, 0));
    outcome.clear();
    actions.clear();
    actions
        .push(PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        })
        .unwrap();
    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert_eq!(game.observe_spectator(&state).x, 11);
}

#[test]
fn berry_collection_is_idempotent() {
    let game = Platformer::default();
    let mut state = game.init(1);
    state
        .world
        .set_body_position(PLAYER_BODY_ID, game.config.player_center(1, 0));
    let mut rng = DeterministicRng::from_seed_and_stream(1, 1);
    let mut outcome = StepOutcome::<FixedVec<PlayerReward, 1>>::default();
    let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        })
        .unwrap();

    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    let remaining = state.remaining_berries;
    outcome.clear();
    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert_eq!(state.remaining_berries, remaining);
}

#[test]
fn final_berry_terminates_with_bonus() {
    let game = Platformer::default();
    let mut state = game.init(9);
    state.remaining_berries = 1u8 << 5;
    game.sync_berries(&mut state);
    state
        .world
        .set_body_position(PLAYER_BODY_ID, game.config.player_center(11, 0));
    let mut rng = DeterministicRng::from_seed_and_stream(9, 1);
    let mut outcome = StepOutcome::<FixedVec<PlayerReward, 1>>::default();
    let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        })
        .unwrap();
    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert!(game.is_terminal(&state));
    assert!(outcome.reward_for(0) >= 10);
}

#[test]
fn seeded_sessions_replay_exactly() {
    let mut left = Session::new(Platformer::default(), 3);
    let mut right = Session::new(Platformer::default(), 3);
    let actions = [
        PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        },
        PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        },
        PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        },
    ];
    for action in actions {
        left.step(std::slice::from_ref(&action));
        right.step(std::slice::from_ref(&action));
    }
    assert_eq!(left.trace(), right.trace());
    assert_eq!(left.state(), right.state());
}

#[test]
fn verification_helpers_hold_for_jump() {
    let game = Platformer::default();
    let state = game.init(3);
    let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        })
        .unwrap();
    assert_transition_contracts(&game, &state, &actions, 3);
    assert_observation_contracts(&game, &state);
    assert_compact_roundtrip(&game, &PlatformerAction::Jump);
}

#[test]
fn physics_world_tracks_actor_and_berries() {
    let state = Platformer::default().init(3);
    let world = Platformer::default().world_view(&state);
    assert_eq!(world.physics.bodies.len(), PLATFORMER_BODIES);
    assert!(world.physics.invariant());
}
