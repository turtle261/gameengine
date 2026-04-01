//! Parallel deterministic replay helpers.

use rayon::prelude::*;

use crate::game::Game;
use crate::session::InteractiveSession;
use crate::types::{DynamicReplayTrace, PlayerAction, Seed};

/// Sequence of staged joint actions used for one replay execution.
pub type JointActionTrace<A> = Vec<Vec<PlayerAction<A>>>;

/// Replays many deterministic traces in parallel and returns resulting replay traces.
pub fn replay_many<G>(
    game: &G,
    traces: &[(Seed, JointActionTrace<G::Action>)],
) -> Vec<DynamicReplayTrace<G::JointActionBuf, G::RewardBuf>>
where
    G: Game + Copy + Send + Sync,
    G::Action: Send + Sync,
    G::JointActionBuf: Send,
    G::RewardBuf: Send,
{
    traces
        .par_iter()
        .map(|(seed, steps)| {
            let mut session = InteractiveSession::new(*game, *seed);
            for step in steps {
                if session.is_terminal() {
                    break;
                }
                session.step(step);
            }
            session.into_trace()
        })
        .collect()
}
