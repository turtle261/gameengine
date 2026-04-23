use super::{KuhnPoker, KuhnPokerAction};
use crate::buffer::FixedVec;
use crate::game::GameAuthoring;
use crate::types::PlayerAction;

#[kani::proof]
#[kani::unwind(32)]
fn initial_observation_is_three_bit_code() {
    let game = KuhnPoker;
    let state = game.init(17);
    assert!(state.observation <= 6);
    assert!(matches!(state.observation, 0..=2 | 4..=6));
}

#[kani::proof]
#[kani::unwind(32)]
fn transition_contracts_hold_for_pass_action() {
    let game = KuhnPoker;
    let state = game.init(17);
    let mut actions = FixedVec::<PlayerAction<KuhnPokerAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: KuhnPokerAction::Pass,
        })
        .unwrap();
    crate::verification::assert_transition_contracts(&game, &state, &actions, 17);
}
