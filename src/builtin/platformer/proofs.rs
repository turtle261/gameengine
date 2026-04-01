use super::{ALL_BERRIES_MASK, PLAYER_BODY_ID, Platformer, PlatformerAction, PlatformerState};
use crate::buffer::FixedVec;
use crate::game::Game;
use crate::types::PlayerAction;

#[kani::proof]
#[kani::unwind(64)]
fn wall_clamps_hold_for_all_edge_positions() {
    let game = Platformer::default();
    let mut state = PlatformerState::default();
    let x: u8 = kani::any();
    kani::assume(x < game.config.width);
    state
        .world
        .set_body_position(PLAYER_BODY_ID, game.config.player_center(x, 0));
    let mut rng = crate::rng::DeterministicRng::from_seed(1);
    let mut outcome =
        crate::types::StepOutcome::<FixedVec<crate::types::PlayerReward, 1>>::default();
    let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: PlatformerAction::Left,
        })
        .unwrap();
    game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
    assert!(game.observe_spectator(&state).x < game.config.width);
}

#[kani::proof]
#[kani::unwind(64)]
fn jump_reward_is_bounded() {
    let state = Platformer::default().init(1);
    let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        })
        .unwrap();
    crate::verification::assert_transition_contracts(&Platformer::default(), &state, &actions, 1);
}

#[kani::proof]
#[kani::unwind(64)]
fn initial_observation_and_world_contracts_hold() {
    let game = Platformer::default();
    let state = game.init(1);
    crate::verification::assert_observation_contracts(&game, &state);
}

#[kani::proof]
#[kani::unwind(64)]
fn berry_mask_tracks_trigger_activation() {
    let mut state = PlatformerState::default();
    state.remaining_berries = ALL_BERRIES_MASK ^ 0b000001;
    Platformer::default().sync_berries(&mut state);
    assert!(!state.world.require_body(super::FIRST_BERRY_BODY_ID).active);
}
