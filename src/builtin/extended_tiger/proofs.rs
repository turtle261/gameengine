use super::{ExtendedTiger, ExtendedTigerAction};
use crate::buffer::FixedVec;
use crate::game::GameAuthoring;
use crate::types::PlayerAction;

#[kani::proof]
#[kani::unwind(32)]
fn initial_observation_fits_three_bits() {
    let game = ExtendedTiger;
    let state = game.init(11);
    let observation = game.observe_player(&state, 0);
    assert!(observation.value <= 7);
}

#[kani::proof]
#[kani::unwind(32)]
fn transition_contracts_hold_for_stand_action() {
    let game = ExtendedTiger;
    let state = game.init(11);
    let mut actions = FixedVec::<PlayerAction<ExtendedTigerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: ExtendedTigerAction::Stand,
        })
        .unwrap();
    crate::verification::assert_transition_contracts(&game, &state, &actions, 11);
}
