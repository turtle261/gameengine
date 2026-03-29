use std::marker::PhantomData;

use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::PlayerId;

pub trait Policy<G: Game> {
    fn choose_action(
        &mut self,
        game: &G,
        state: &G::State,
        player: PlayerId,
        observation: &G::PlayerObservation,
        legal_actions: &[G::Action],
        rng: &mut DeterministicRng,
    ) -> G::Action;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FirstLegalPolicy;

impl<G: Game> Policy<G> for FirstLegalPolicy {
    fn choose_action(
        &mut self,
        _game: &G,
        _state: &G::State,
        _player: PlayerId,
        _observation: &G::PlayerObservation,
        legal_actions: &[G::Action],
        _rng: &mut DeterministicRng,
    ) -> G::Action {
        legal_actions
            .first()
            .copied()
            .expect("policy requires at least one legal action")
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RandomPolicy;

impl<G: Game> Policy<G> for RandomPolicy {
    fn choose_action(
        &mut self,
        _game: &G,
        _state: &G::State,
        _player: PlayerId,
        _observation: &G::PlayerObservation,
        legal_actions: &[G::Action],
        rng: &mut DeterministicRng,
    ) -> G::Action {
        let index = rng.gen_range(legal_actions.len());
        legal_actions[index]
    }
}

#[derive(Clone, Debug)]
pub struct ScriptedPolicy<A> {
    script: Vec<A>,
    position: usize,
}

impl<A> ScriptedPolicy<A> {
    pub fn new(script: Vec<A>) -> Self {
        Self {
            script,
            position: 0,
        }
    }
}

impl<G> Policy<G> for ScriptedPolicy<G::Action>
where
    G: Game,
{
    fn choose_action(
        &mut self,
        _game: &G,
        _state: &G::State,
        _player: PlayerId,
        _observation: &G::PlayerObservation,
        legal_actions: &[G::Action],
        _rng: &mut DeterministicRng,
    ) -> G::Action {
        if let Some(action) = self.script.get(self.position) {
            self.position += 1;
            if legal_actions.contains(action) {
                return *action;
            }
        }
        legal_actions
            .first()
            .copied()
            .expect("policy requires at least one legal action")
    }
}

pub struct FnPolicy<G, F> {
    f: F,
    _marker: PhantomData<G>,
}

impl<G, F> FnPolicy<G, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _marker: PhantomData,
        }
    }
}

impl<G, F> Policy<G> for FnPolicy<G, F>
where
    G: Game,
    F: FnMut(
        &G,
        &G::State,
        PlayerId,
        &G::PlayerObservation,
        &[G::Action],
        &mut DeterministicRng,
    ) -> G::Action,
{
    fn choose_action(
        &mut self,
        game: &G,
        state: &G::State,
        player: PlayerId,
        observation: &G::PlayerObservation,
        legal_actions: &[G::Action],
        rng: &mut DeterministicRng,
    ) -> G::Action {
        (self.f)(game, state, player, observation, legal_actions, rng)
    }
}
