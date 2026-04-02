//! Helpers that compare runtime game behavior against executable model semantics.

use crate::buffer::Buffer;
use crate::proof::model::RefinementWitness;
use crate::rng::DeterministicRng;
use crate::session::{FixedHistory, SessionKernel};
use crate::types::{ReplayStep, Seed, StepOutcome};

pub fn assert_model_init_refinement<G: RefinementWitness>(
    game: &G,
    seed: Seed,
    params: &G::Params,
) {
    let state = game.init_with_params(seed, params);
    let model = game.model_init_with_params(seed, params);
    assert!(game.safety_state_invariant(&state));
    assert!(game.state_refines_model(&state, &model));
    assert_eq!(game.is_terminal(&state), game.model_is_terminal(&model));
    assert!(game.compact_spec_refines_model(params));

    let mut runtime_players = G::PlayerBuf::default();
    let mut model_players = G::PlayerBuf::default();
    game.players_to_act(&state, &mut runtime_players);
    game.model_players_to_act(&model, &mut model_players);
    assert_eq!(runtime_players, model_players);

    for player in 0..game.player_count() {
        let mut runtime_actions = G::ActionBuf::default();
        let mut model_actions = G::ActionBuf::default();
        game.legal_actions(&state, player, &mut runtime_actions);
        game.model_legal_actions(&model, player, &mut model_actions);
        assert_eq!(runtime_actions, model_actions);
    }
}

pub fn assert_model_observation_refinement<G: RefinementWitness>(game: &G, state: &G::State) {
    let model = game.runtime_state_to_model(state);
    for player in 0..game.player_count() {
        let observation = game.observe_player(state, player);
        let model_observation = game.model_observe_player(&model, player);
        assert!(game.safety_player_observation_invariant(state, player, &observation));
        assert!(game.observation_refines_model(&observation, &model_observation));
    }

    let spectator = game.observe_spectator(state);
    let model_spectator = game.model_observe_spectator(&model);
    assert!(game.safety_spectator_observation_invariant(state, &spectator));
    assert!(game.observation_refines_model(&spectator, &model_spectator));

    let world = game.world_view(state);
    let model_world = game.model_world_view(&model);
    assert!(game.safety_world_view_invariant(state, &world));
    assert!(game.world_view_refines_model(&world, &model_world));
}

pub fn assert_model_step_refinement<G: RefinementWitness>(
    game: &G,
    pre: &G::State,
    actions: &G::JointActionBuf,
    seed: Seed,
) {
    assert!(game.safety_state_invariant(pre));
    for action in actions.as_slice() {
        assert!(game.safety_action_invariant(&action.action));
    }

    let mut runtime_state = pre.clone();
    let mut model_state = game.runtime_state_to_model(pre);
    let mut runtime_rng = DeterministicRng::from_seed_and_stream(seed, 99);
    let mut model_rng = runtime_rng;
    let mut runtime_outcome = StepOutcome::<G::RewardBuf>::default();
    let mut model_outcome = StepOutcome::<G::RewardBuf>::default();

    game.step_in_place(
        &mut runtime_state,
        actions,
        &mut runtime_rng,
        &mut runtime_outcome,
    );
    game.model_step_in_place(
        &mut model_state,
        actions,
        &mut model_rng,
        &mut model_outcome,
    );

    assert_eq!(runtime_rng, model_rng);
    assert_eq!(runtime_outcome, model_outcome);
    assert!(game.safety_state_invariant(&runtime_state));
    assert!(game.state_refines_model(&runtime_state, &model_state));
    assert_model_observation_refinement(game, &runtime_state);
    assert!(game.safety_transition_postcondition(pre, actions, &runtime_state, &runtime_outcome,));
}

pub fn assert_model_replay_refinement<G>(
    game: G,
    seed: Seed,
    params: G::Params,
    trace: &[G::JointActionBuf],
) where
    G: RefinementWitness + Clone,
    ReplayStep<G::JointActionBuf, G::RewardBuf>: Default,
{
    type ProofHistory<T> = FixedHistory<T, 8, 4, 1>;

    let mut session =
        SessionKernel::<G, ProofHistory<G>>::new_with_params(game.clone(), seed, params.clone());
    let mut model_state = game.model_init_with_params(seed, &params);
    let mut model_rng = DeterministicRng::from_seed_and_stream(seed, 1);

    for actions in trace {
        if session.is_terminal() {
            break;
        }
        let outcome = session.step_with_joint_actions(actions).clone();
        let mut model_outcome = StepOutcome::<G::RewardBuf>::default();
        game.model_step_in_place(
            &mut model_state,
            actions,
            &mut model_rng,
            &mut model_outcome,
        );
        model_outcome.tick = session.current_tick();
        assert_eq!(outcome, model_outcome);
        assert_eq!(session.rng(), model_rng);
        assert!(game.state_refines_model(session.state(), &model_state));
        assert_model_observation_refinement(&game, session.state());

        let recorded = &session.trace().steps[(session.current_tick() - 1) as usize];
        assert_eq!(recorded.tick, outcome.tick);
        assert_eq!(&recorded.actions, actions);
        assert_eq!(&recorded.rewards, &outcome.rewards);
        assert_eq!(recorded.termination, outcome.termination);
    }

    let executed_ticks = session.trace().len() as u64;
    let mut target_tick = 0u64;
    while target_tick <= executed_ticks {
        let restored_state = session
            .state_at(target_tick)
            .expect("recorded tick must be restorable");
        let fork = session
            .fork_at(target_tick)
            .expect("recorded tick must produce a rewound fork");
        let mut replay_state = game.model_init_with_params(seed, &params);
        let mut replay_rng = DeterministicRng::from_seed_and_stream(seed, 1);
        let mut replay_tick = 0usize;
        while replay_tick < target_tick as usize {
            let mut replay_outcome = StepOutcome::<G::RewardBuf>::default();
            game.model_step_in_place(
                &mut replay_state,
                &trace[replay_tick],
                &mut replay_rng,
                &mut replay_outcome,
            );
            replay_tick += 1;
        }

        assert!(game.safety_state_invariant(&restored_state));
        assert!(game.state_refines_model(&restored_state, &replay_state));
        assert_model_observation_refinement(&game, &restored_state);
        assert_eq!(fork.current_tick(), target_tick);
        assert_eq!(*fork.state(), restored_state);
        assert_eq!(fork.rng(), replay_rng);
        assert_model_observation_refinement(&game, fork.state());

        target_tick += 1;
    }
}
