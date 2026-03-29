use rayon::prelude::*;

use crate::game::Game;
use crate::session::Session;
use crate::types::{PlayerAction, ReplayTrace, Seed};

pub type JointActionTrace<A> = Vec<Vec<PlayerAction<A>>>;

pub fn replay_many<G>(
    game: &G,
    traces: &[(Seed, JointActionTrace<G::Action>)],
) -> Vec<ReplayTrace<G::JointActionBuf, G::RewardBuf, 256>>
where
    G: Game + Copy + Send + Sync,
    G::Action: Send + Sync,
    G::JointActionBuf: Send,
    G::RewardBuf: Send,
{
    traces
        .par_iter()
        .map(|(seed, steps)| {
            let mut session = Session::new(*game, *seed);
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
