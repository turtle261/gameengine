/// Declares the standard Kani refinement harness triplet for a verified game.
#[macro_export]
macro_rules! declare_refinement_harnesses {
    (
        game = $game:expr,
        params = $params:expr,
        seed = $seed:expr,
        actions = $actions:expr,
        trace = $trace:expr,
        init = $init_name:ident,
        step = $step_name:ident,
        replay = $replay_name:ident $(,)?
    ) => {
        #[kani::proof]
        fn $init_name() {
            let game = $game;
            let params = $params;
            $crate::proof::assert_model_init_refinement(&game, $seed, &params);
            let state = game.init_with_params($seed, &params);
            $crate::proof::assert_model_observation_refinement(&game, &state);
        }

        #[kani::proof]
        fn $step_name() {
            let game = $game;
            let params = $params;
            let state = game.init_with_params($seed, &params);
            let actions = $actions;
            $crate::proof::assert_model_step_refinement(&game, &state, &actions, $seed);
        }

        #[kani::proof]
        fn $replay_name() {
            let game = $game;
            let params = $params;
            let trace = $trace;
            $crate::proof::assert_model_replay_refinement(game, $seed, params, &trace);
        }
    };
    (
        game = $game:expr,
        params = $params:expr,
        seed = $seed:expr,
        actions = $actions:expr,
        init = $init_name:ident,
        step = $step_name:ident,
        replay = $replay_name:ident $(,)?
    ) => {
        $crate::declare_refinement_harnesses!(
            game = $game,
            params = $params,
            seed = $seed,
            actions = $actions,
            trace = {
                let actions = $actions;
                [actions]
            },
            init = $init_name,
            step = $step_name,
            replay = $replay_name,
        );
    };
}
