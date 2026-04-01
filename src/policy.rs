//! Policy interfaces and builtin policy strategies.

use std::marker::PhantomData;

use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::PlayerId;

/// Policy interface for selecting actions for active players.
pub trait Policy<G: Game> {
    /// Chooses one legal action for `player`.
    fn choose_action(
        &mut self,
        game: &G,
        state: &G::State,
        player: PlayerId,
        observation: &G::Obs,
        legal_actions: &[G::Action],
        rng: &mut DeterministicRng,
    ) -> G::Action;
}

/// Deterministic policy that always selects the first legal action.
#[derive(Clone, Copy, Debug, Default)]
pub struct FirstLegalPolicy;

impl<G: Game> Policy<G> for FirstLegalPolicy {
    fn choose_action(
        &mut self,
        _game: &G,
        _state: &G::State,
        _player: PlayerId,
        _observation: &G::Obs,
        legal_actions: &[G::Action],
        _rng: &mut DeterministicRng,
    ) -> G::Action {
        legal_actions
            .first()
            .copied()
            .expect("policy requires at least one legal action")
    }
}

/// Uniform-random policy over legal actions.
#[derive(Clone, Copy, Debug, Default)]
pub struct RandomPolicy;

impl<G: Game> Policy<G> for RandomPolicy {
    fn choose_action(
        &mut self,
        _game: &G,
        _state: &G::State,
        _player: PlayerId,
        _observation: &G::Obs,
        legal_actions: &[G::Action],
        rng: &mut DeterministicRng,
    ) -> G::Action {
        let index = rng.gen_range(legal_actions.len());
        legal_actions[index]
    }
}

/// Deterministic scripted policy with fallback to first legal action.
#[derive(Clone, Debug)]
pub struct ScriptedPolicy<A> {
    script: Vec<A>,
    position: usize,
    strict: bool,
}

impl<A> ScriptedPolicy<A> {
    /// Creates a scripted policy from a full action script.
    pub fn new(script: Vec<A>) -> Self {
        Self {
            script,
            position: 0,
            strict: false,
        }
    }

    /// Creates a strict scripted policy that fails fast on illegal or exhausted scripts.
    pub fn new_strict(script: Vec<A>) -> Self {
        Self {
            script,
            position: 0,
            strict: true,
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
        _observation: &G::Obs,
        legal_actions: &[G::Action],
        _rng: &mut DeterministicRng,
    ) -> G::Action {
        if let Some(action) = self.script.get(self.position) {
            self.position += 1;
            if legal_actions.contains(action) {
                return *action;
            }
            if self.strict {
                panic!(
                    "strict scripted policy action at index {} is illegal for current state",
                    self.position - 1
                );
            }
        } else if self.strict {
            panic!(
                "strict scripted policy exhausted at index {}",
                self.position
            );
        }
        legal_actions
            .first()
            .copied()
            .expect("policy requires at least one legal action")
    }
}

/// Policy adapter built from a closure.
pub struct FnPolicy<G, F> {
    f: F,
    _marker: PhantomData<G>,
}

impl<G, F> FnPolicy<G, F> {
    /// Creates a closure-backed policy.
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
    F: FnMut(&G, &G::State, PlayerId, &G::Obs, &[G::Action], &mut DeterministicRng) -> G::Action,
{
    fn choose_action(
        &mut self,
        game: &G,
        state: &G::State,
        player: PlayerId,
        observation: &G::Obs,
        legal_actions: &[G::Action],
        rng: &mut DeterministicRng,
    ) -> G::Action {
        (self.f)(game, state, player, observation, legal_actions, rng)
    }
}
