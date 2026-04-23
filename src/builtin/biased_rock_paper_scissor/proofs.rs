use super::{BiasedRockPaperScissor, BiasedRockPaperScissorAction};
use crate::buffer::FixedVec;
use crate::game::GameAuthoring;
use crate::types::PlayerAction;

#[kani::proof]
#[kani::unwind(24)]
fn observation_encoding_stays_within_two_bits() {
    let game = BiasedRockPaperScissor;
    let state = game.init(5);
    let observation = game.observe_player(&state, 0);
    let mut words = FixedVec::<u64, 1>::default();
    game.encode_player_observation(&observation, &mut words);
    assert_eq!(words.len(), 1);
    assert!(words.as_slice()[0] <= 2);
}

#[kani::proof]
#[kani::unwind(24)]
fn transition_contracts_hold_for_paper_action() {
    let game = BiasedRockPaperScissor;
    let state = game.init(5);
    let mut actions = FixedVec::<PlayerAction<BiasedRockPaperScissorAction>, 1>::default();
    actions
        .push(PlayerAction {
            player: 0,
            action: BiasedRockPaperScissorAction::Paper,
        })
        .unwrap();
    crate::verification::assert_transition_contracts(&game, &state, &actions, 5);
}
