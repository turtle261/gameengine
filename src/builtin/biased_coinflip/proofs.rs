use super::{BiasedCoinFlip, BiasedCoinFlipAction};
use crate::buffer::FixedVec;
use crate::game::GameAuthoring;
use crate::types::PlayerAction;

#[kani::proof]
#[kani::unwind(16)]
fn compact_observation_stays_one_bit() {
    let game = BiasedCoinFlip::default();
    let state = game.init(7);
    let observation = game.observe_player(&state, 0);
    let mut encoded = FixedVec::<u64, 1>::default();
    game.encode_player_observation(&observation, &mut encoded);
    assert_eq!(encoded.len(), 1);
    assert!(encoded.as_slice()[0] <= 1);
}

#[kani::proof]
#[kani::unwind(16)]
fn transition_contracts_hold_for_head_guess() {
    let game = BiasedCoinFlip::default();
    let state = game.init(7);
    let mut actions = FixedVec::<PlayerAction<BiasedCoinFlipAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: BiasedCoinFlipAction::GuessHeads,
        })
        .unwrap();
    crate::verification::assert_transition_contracts(&game, &state, &actions, 7);
}
