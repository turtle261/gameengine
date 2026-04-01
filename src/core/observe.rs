//! Observation adapter trait and viewpoint selection types.

use core::fmt::Debug;

use crate::game::Game;
use crate::types::PlayerId;

/// Viewpoint used when requesting an observation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Observer {
    /// Player-local, potentially partial-information observation.
    Player(PlayerId),
    /// Full spectator observation.
    Spectator,
}

/// Adapter trait for producing and encoding generic observations.
pub trait Observe: Game {
    /// Observation type emitted by this adapter.
    type Obs: Clone + Debug + Default + Eq + PartialEq;

    /// Builds an observation for the selected viewpoint.
    fn observe(&self, state: &Self::State, who: Observer) -> Self::Obs;

    /// Encodes an observation into the compact word stream.
    fn encode_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf);

    /// Encodes an observation with explicit viewpoint context.
    fn encode_observation_for(
        &self,
        who: Observer,
        observation: &Self::Obs,
        out: &mut Self::WordBuf,
    ) {
        let _ = who;
        self.encode_observation(observation, out);
    }

    /// Convenience helper to observe and encode in one call.
    fn observe_and_encode(&self, state: &Self::State, who: Observer, out: &mut Self::WordBuf) {
        let observation = self.observe(state, who);
        self.encode_observation_for(who, &observation, out);
    }
}

impl<G> Observe for G
where
    G: Game<SpectatorObservation = <G as Game>::PlayerObservation>,
{
    type Obs = G::PlayerObservation;

    fn observe(&self, state: &Self::State, who: Observer) -> Self::Obs {
        match who {
            Observer::Player(player) => self.observe_player(state, player),
            Observer::Spectator => self.observe_spectator(state),
        }
    }

    fn encode_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        self.encode_player_observation(observation, out);
    }

    fn encode_observation_for(
        &self,
        who: Observer,
        observation: &Self::Obs,
        out: &mut Self::WordBuf,
    ) {
        match who {
            Observer::Player(_) => self.encode_player_observation(observation, out),
            Observer::Spectator => self.encode_spectator_observation(observation, out),
        }
    }
}
