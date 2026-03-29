use rayon::prelude::*;

use crate::game::Game;
use crate::session::Session;
use crate::types::{PlayerAction, ReplayTrace, Seed};

pub fn replay_many<G>(
    game: &G,
    traces: &[(Seed, Vec<Vec<PlayerAction<G::Action>>>)],
) -> Vec<ReplayTrace<G::Action>>
where
    G: Game + Clone + Sync,
    G::Action: Send + Sync,
    G::State: Send,
{
    traces
        .par_iter()
        .map(|(seed, actions_per_tick)| {
            let mut session = Session::new(game.clone(), *seed);
            for actions in actions_per_tick {
                if session.is_terminal() {
                    break;
                }
                session.step(actions);
            }
            session.into_trace()
        })
        .collect()
}
