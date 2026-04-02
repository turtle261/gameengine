use super::*;
use crate::core::env::DefaultEnvironment;
use crate::core::observe::Observer;
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

#[test]
fn parameterized_rewards_update_transition_and_compact_contracts() {
    let mut config = PlatformerConfig {
        sprain_numerator: 0,
        berry_reward: 4,
        finish_bonus: 30,
        ..PlatformerConfig::default()
    };
    config.berry_y = config.jump_delta;
    let game = Platformer::default();
    let spec = game.compact_spec_for_params(&config);

    let mut state = game.init_with_params(1, &config);
    state.remaining_berries = 1;
    game.sync_berries(&mut state);
    state
        .world
        .set_body_position(PLAYER_BODY_ID, config.player_center(config.berry_xs[0], 0));

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

    assert_eq!(outcome.reward_for(0), 34);
    assert!(spec.max_reward >= 34);
    assert!(spec.try_encode_reward(34).is_ok());
}

#[test]
fn parameterized_environment_uses_wide_observation_schema() {
    let config = PlatformerConfig {
        width: 40,
        height: 10,
        jump_delta: 7,
        berry_y: 7,
        berry_xs: [1, 6, 11, 16, 21, 26],
        ..PlatformerConfig::default()
    };
    let mut env = DefaultEnvironment::<Platformer, 1>::new(
        Platformer::default(),
        3,
        Observer::Player(0),
    );
    let packet = env.reset_with_params(3, config).unwrap();
    assert_eq!(packet.words().len(), 1);
    assert!(packet.words()[0] > 4095);
}
