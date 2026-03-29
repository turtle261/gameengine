use crate::game::Game;
use crate::policy::Policy;
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, ReplayTrace, Seed, StepOutcome};

pub struct Session<G: Game> {
    game: G,
    state: G::State,
    rng: DeterministicRng,
    tick: u64,
    trace: ReplayTrace<G::Action>,
    players_to_act: Vec<PlayerId>,
    legal_actions: Vec<G::Action>,
    joint_actions: Vec<PlayerAction<G::Action>>,
    outcome: StepOutcome,
}

impl<G: Game> Session<G> {
    pub fn new(game: G, seed: Seed) -> Self {
        let players = game.player_count();
        let state = game.init(seed);
        Self {
            game,
            state,
            rng: DeterministicRng::from_seed_and_stream(seed, 1),
            tick: 0,
            trace: ReplayTrace::new(seed),
            players_to_act: Vec::with_capacity(players),
            legal_actions: Vec::new(),
            joint_actions: Vec::with_capacity(players.max(1)),
            outcome: StepOutcome::with_player_capacity(players),
        }
    }

    pub fn game(&self) -> &G {
        &self.game
    }

    pub fn state(&self) -> &G::State {
        &self.state
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    pub fn rng(&self) -> DeterministicRng {
        self.rng
    }

    pub fn is_terminal(&self) -> bool {
        self.game.is_terminal(&self.state)
    }

    pub fn player_observation(&self, player: PlayerId) -> G::PlayerObservation {
        self.game.observe_player(&self.state, player)
    }

    pub fn spectator_observation(&self) -> G::SpectatorObservation {
        self.game.observe_spectator(&self.state)
    }

    pub fn legal_actions_for(&mut self, player: PlayerId) -> &[G::Action] {
        self.game
            .legal_actions(&self.state, player, &mut self.legal_actions);
        &self.legal_actions
    }

    pub fn step(&mut self, actions: &[PlayerAction<G::Action>]) -> &StepOutcome {
        assert!(
            !self.game.is_terminal(&self.state),
            "cannot step a terminal session",
        );
        self.outcome.clear();
        self.game
            .step_in_place(&mut self.state, actions, &mut self.rng, &mut self.outcome);
        self.tick += 1;
        self.outcome.tick = self.tick;
        self.trace.record(self.tick, actions, &self.outcome);
        &self.outcome
    }

    pub fn step_with_policies(&mut self, policies: &mut [&mut dyn Policy<G>]) -> &StepOutcome {
        self.players_to_act.clear();
        self.game
            .players_to_act(&self.state, &mut self.players_to_act);
        self.joint_actions.clear();

        for &player in &self.players_to_act {
            self.game
                .legal_actions(&self.state, player, &mut self.legal_actions);
            let observation = self.game.observe_player(&self.state, player);
            let policy = policies
                .get_mut(player)
                .expect("missing policy for active player");
            let action = policy.choose_action(
                &self.game,
                &self.state,
                player,
                &observation,
                &self.legal_actions,
                &mut self.rng,
            );
            self.joint_actions.push(PlayerAction { player, action });
        }

        let actions = self.joint_actions.clone();
        self.step(&actions)
    }

    pub fn run_until_terminal(
        &mut self,
        policies: &mut [&mut dyn Policy<G>],
        max_ticks: usize,
    ) -> &ReplayTrace<G::Action> {
        while !self.is_terminal() && (self.tick as usize) < max_ticks {
            self.step_with_policies(policies);
        }
        &self.trace
    }

    pub fn trace(&self) -> &ReplayTrace<G::Action> {
        &self.trace
    }

    pub fn into_trace(self) -> ReplayTrace<G::Action> {
        self.trace
    }
}
