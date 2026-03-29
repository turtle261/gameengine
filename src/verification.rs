use crate::buffer::Buffer;
use crate::compact::CompactGame;
use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::{Seed, StepOutcome};

pub fn assert_transition_contracts<G: Game>(
    game: &G,
    pre: &G::State,
    actions: &G::JointActionBuf,
    seed: Seed,
) {
    assert!(game.state_invariant(pre));
    for action in actions.as_slice() {
        assert!(game.action_invariant(&action.action));
    }

    let mut left_state = pre.clone();
    let mut right_state = pre.clone();
    let mut left_rng = DeterministicRng::from_seed_and_stream(seed, 99);
    let mut right_rng = DeterministicRng::from_seed_and_stream(seed, 99);
    let mut left_outcome = StepOutcome::<G::RewardBuf>::default();
    let mut right_outcome = StepOutcome::<G::RewardBuf>::default();

    game.step_in_place(&mut left_state, actions, &mut left_rng, &mut left_outcome);
    game.step_in_place(
        &mut right_state,
        actions,
        &mut right_rng,
        &mut right_outcome,
    );

    assert_eq!(left_state, right_state);
    assert_eq!(left_outcome, right_outcome);
    assert_eq!(left_rng, right_rng);
    assert!(game.state_invariant(&left_state));
    assert!(game.transition_postcondition(pre, actions, &left_state, &left_outcome,));
}

pub fn assert_observation_contracts<G: Game>(game: &G, state: &G::State) {
    assert!(game.state_invariant(state));
    for player in 0..game.player_count() {
        let observation = game.observe_player(state, player);
        assert!(game.player_observation_invariant(state, player, &observation,));
    }
    let spectator = game.observe_spectator(state);
    assert!(game.spectator_observation_invariant(state, &spectator));
    let world = game.world_view(state);
    assert!(game.world_view_invariant(state, &world));
}

pub fn assert_compact_roundtrip<G: CompactGame>(game: &G, action: &G::Action) {
    let encoded = game.encode_action(action);
    assert_eq!(game.decode_action(encoded), Some(*action));
}
