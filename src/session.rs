use core::fmt::Debug;

use crate::buffer::{Buffer, default_array};
use crate::game::Game;
use crate::policy::Policy;
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, ReplayTrace, Seed, StepOutcome, Tick};

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct HistorySnapshot<S> {
    pub tick: Tick,
    pub state: S,
    pub rng: DeterministicRng,
}

pub trait HistoryStore<G: Game>: Clone {
    type Trace: Clone + Debug + Eq + PartialEq;

    fn from_seed(seed: Seed, initial_state: &G::State, initial_rng: DeterministicRng) -> Self;
    fn reset(&mut self, seed: Seed, initial_state: &G::State, initial_rng: DeterministicRng);
    fn record(
        &mut self,
        tick: Tick,
        state: &G::State,
        rng: DeterministicRng,
        actions: &G::JointActionBuf,
        outcome: &StepOutcome<G::RewardBuf>,
    );
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn trace(&self) -> &Self::Trace;
    fn into_trace(self) -> Self::Trace;
    fn restore(&self, game: &G, target_tick: Tick) -> Option<(G::State, DeterministicRng)>;
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct FixedHistory<G: Game, const LOG: usize, const SNAPSHOTS: usize, const SNAP_EVERY: usize>
where
    crate::types::ReplayStep<G::JointActionBuf, G::RewardBuf>: Default,
{
    seed: Seed,
    initial_state: G::State,
    initial_rng: DeterministicRng,
    trace: ReplayTrace<G::JointActionBuf, G::RewardBuf, LOG>,
    snapshots: [HistorySnapshot<G::State>; SNAPSHOTS],
    snapshot_count: usize,
}

impl<G: Game, const LOG: usize, const SNAPSHOTS: usize, const SNAP_EVERY: usize> Clone
    for FixedHistory<G, LOG, SNAPSHOTS, SNAP_EVERY>
where
    crate::types::ReplayStep<G::JointActionBuf, G::RewardBuf>: Default,
{
    fn clone(&self) -> Self {
        Self {
            seed: self.seed,
            initial_state: self.initial_state.clone(),
            initial_rng: self.initial_rng,
            trace: self.trace.clone(),
            snapshots: self.snapshots.clone(),
            snapshot_count: self.snapshot_count,
        }
    }
}

impl<G: Game, const LOG: usize, const SNAPSHOTS: usize, const SNAP_EVERY: usize>
    FixedHistory<G, LOG, SNAPSHOTS, SNAP_EVERY>
where
    crate::types::ReplayStep<G::JointActionBuf, G::RewardBuf>: Default,
{
    fn store_snapshot(&mut self, tick: Tick, state: &G::State, rng: DeterministicRng) {
        if SNAPSHOTS == 0 || SNAP_EVERY == 0 {
            return;
        }
        if !tick.is_multiple_of(SNAP_EVERY as u64) {
            return;
        }
        let slot = ((tick / SNAP_EVERY as u64) as usize - 1) % SNAPSHOTS;
        self.snapshots[slot] = HistorySnapshot {
            tick,
            state: state.clone(),
            rng,
        };
        if self.snapshot_count < SNAPSHOTS {
            self.snapshot_count += 1;
        }
    }

    fn best_snapshot(&self, target_tick: Tick) -> Option<&HistorySnapshot<G::State>> {
        let mut best_index = None;
        let mut best_tick = 0;
        let mut index = 0usize;
        while index < self.snapshot_count {
            let snapshot = &self.snapshots[index];
            if snapshot.tick <= target_tick && (best_index.is_none() || snapshot.tick > best_tick) {
                best_index = Some(index);
                best_tick = snapshot.tick;
            }
            index += 1;
        }
        best_index.map(|index| &self.snapshots[index])
    }
}

impl<G: Game, const LOG: usize, const SNAPSHOTS: usize, const SNAP_EVERY: usize> HistoryStore<G>
    for FixedHistory<G, LOG, SNAPSHOTS, SNAP_EVERY>
where
    crate::types::ReplayStep<G::JointActionBuf, G::RewardBuf>: Default,
{
    type Trace = ReplayTrace<G::JointActionBuf, G::RewardBuf, LOG>;

    fn from_seed(seed: Seed, initial_state: &G::State, initial_rng: DeterministicRng) -> Self {
        Self {
            seed,
            initial_state: initial_state.clone(),
            initial_rng,
            trace: ReplayTrace::new(seed),
            snapshots: default_array(),
            snapshot_count: 0,
        }
    }

    fn reset(&mut self, seed: Seed, initial_state: &G::State, initial_rng: DeterministicRng) {
        self.seed = seed;
        self.initial_state = initial_state.clone();
        self.initial_rng = initial_rng;
        self.trace.clear(seed);
        self.snapshots = default_array();
        self.snapshot_count = 0;
    }

    fn record(
        &mut self,
        tick: Tick,
        state: &G::State,
        rng: DeterministicRng,
        actions: &G::JointActionBuf,
        outcome: &StepOutcome<G::RewardBuf>,
    ) {
        self.trace
            .record(tick, actions, &outcome.rewards, outcome.termination);
        self.store_snapshot(tick, state, rng);
    }

    fn len(&self) -> usize {
        self.trace.len()
    }

    fn is_empty(&self) -> bool {
        self.trace.is_empty()
    }

    fn trace(&self) -> &Self::Trace {
        &self.trace
    }

    fn into_trace(self) -> Self::Trace {
        self.trace
    }

    fn restore(&self, game: &G, target_tick: Tick) -> Option<(G::State, DeterministicRng)> {
        if target_tick > self.trace.len() as u64 {
            return None;
        }

        let (mut state, mut rng, start_tick) =
            if let Some(snapshot) = self.best_snapshot(target_tick) {
                (snapshot.state.clone(), snapshot.rng, snapshot.tick)
            } else {
                (self.initial_state.clone(), self.initial_rng, 0)
            };

        let mut outcome = StepOutcome::<G::RewardBuf>::default();
        let steps = self.trace.steps.as_slice();
        let mut index = start_tick as usize;
        while index < steps.len() {
            let step = &steps[index];
            if step.tick > target_tick {
                break;
            }
            outcome.clear();
            game.step_in_place(&mut state, &step.actions, &mut rng, &mut outcome);
            index += 1;
        }

        Some((state, rng))
    }
}

#[derive(Clone, Debug)]
pub struct SessionKernel<G: Game, H: HistoryStore<G>> {
    game: G,
    state: G::State,
    rng: DeterministicRng,
    tick: Tick,
    history: H,
    players_to_act: G::PlayerBuf,
    legal_actions: G::ActionBuf,
    joint_actions: G::JointActionBuf,
    outcome: StepOutcome<G::RewardBuf>,
}

pub type Session<G> = SessionKernel<G, FixedHistory<G, 256, 32, 8>>;

impl<G: Game, H: HistoryStore<G>> SessionKernel<G, H> {
    pub fn new(game: G, seed: Seed) -> Self {
        let state = game.init(seed);
        let rng = DeterministicRng::from_seed_and_stream(seed, 1);
        let history = H::from_seed(seed, &state, rng);
        Self {
            game,
            state,
            rng,
            tick: 0,
            history,
            players_to_act: G::PlayerBuf::default(),
            legal_actions: G::ActionBuf::default(),
            joint_actions: G::JointActionBuf::default(),
            outcome: StepOutcome::default(),
        }
    }

    pub fn reset(&mut self, seed: Seed) {
        self.state = self.game.init(seed);
        self.rng = DeterministicRng::from_seed_and_stream(seed, 1);
        self.tick = 0;
        self.history.reset(seed, &self.state, self.rng);
        self.players_to_act.clear();
        self.legal_actions.clear();
        self.joint_actions.clear();
        self.outcome.clear();
    }

    pub fn game(&self) -> &G {
        &self.game
    }

    pub fn state(&self) -> &G::State {
        &self.state
    }

    pub fn current_tick(&self) -> Tick {
        self.tick
    }

    pub fn rng(&self) -> DeterministicRng {
        self.rng
    }

    pub fn trace(&self) -> &H::Trace {
        self.history.trace()
    }

    pub fn into_trace(self) -> H::Trace {
        self.history.into_trace()
    }

    pub fn is_terminal(&self) -> bool {
        self.game.is_terminal(&self.state)
    }

    pub fn player_observation(&self, player: usize) -> G::PlayerObservation {
        self.game.observe_player(&self.state, player)
    }

    pub fn spectator_observation(&self) -> G::SpectatorObservation {
        self.game.observe_spectator(&self.state)
    }

    pub fn world_view(&self) -> G::WorldView {
        self.game.world_view(&self.state)
    }

    pub fn legal_actions_for(&mut self, player: usize) -> &[G::Action] {
        self.game
            .legal_actions(&self.state, player, &mut self.legal_actions);
        self.legal_actions.as_slice()
    }

    pub fn step(&mut self, actions: &[PlayerAction<G::Action>]) -> &StepOutcome<G::RewardBuf> {
        self.joint_actions.clear();
        self.joint_actions
            .extend_from_slice(actions)
            .expect("joint action buffer capacity exceeded");
        let joint_actions = self.joint_actions.clone();
        self.step_with_joint_actions(&joint_actions)
    }

    pub fn step_with_joint_actions(
        &mut self,
        actions: &G::JointActionBuf,
    ) -> &StepOutcome<G::RewardBuf> {
        assert!(
            !self.game.is_terminal(&self.state),
            "cannot step a terminal session",
        );
        assert!(self.game.state_invariant(&self.state));
        for action in actions.as_slice() {
            assert!(self.game.action_invariant(&action.action));
        }

        let pre_state = self.state.clone();
        self.outcome.clear();
        self.game
            .step_in_place(&mut self.state, actions, &mut self.rng, &mut self.outcome);
        self.tick += 1;
        self.outcome.tick = self.tick;

        assert!(self.game.state_invariant(&self.state));
        let spectator = self.game.observe_spectator(&self.state);
        assert!(
            self.game
                .spectator_observation_invariant(&self.state, &spectator)
        );
        let world = self.game.world_view(&self.state);
        assert!(self.game.world_view_invariant(&self.state, &world));
        for player in 0..self.game.player_count() {
            let observation = self.game.observe_player(&self.state, player);
            assert!(
                self.game
                    .player_observation_invariant(&self.state, player, &observation)
            );
        }
        assert!(self.game.transition_postcondition(
            &pre_state,
            actions,
            &self.state,
            &self.outcome
        ));

        self.history
            .record(self.tick, &self.state, self.rng, actions, &self.outcome);
        &self.outcome
    }

    pub fn step_with_policies(
        &mut self,
        policies: &mut [&mut dyn Policy<G>],
    ) -> &StepOutcome<G::RewardBuf> {
        self.players_to_act.clear();
        self.game
            .players_to_act(&self.state, &mut self.players_to_act);
        self.joint_actions.clear();

        for &player in self.players_to_act.as_slice() {
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
                self.legal_actions.as_slice(),
                &mut self.rng,
            );
            self.joint_actions
                .push(PlayerAction { player, action })
                .expect("joint action buffer capacity exceeded");
        }

        let actions = self.joint_actions.clone();
        self.step_with_joint_actions(&actions)
    }

    pub fn run_until_terminal(
        &mut self,
        policies: &mut [&mut dyn Policy<G>],
        max_ticks: usize,
    ) -> &H::Trace {
        while !self.is_terminal() && (self.tick as usize) < max_ticks {
            self.step_with_policies(policies);
        }
        self.trace()
    }

    pub fn rewind_to(&mut self, target_tick: Tick) -> bool {
        let Some((state, rng)) = self.history.restore(&self.game, target_tick) else {
            return false;
        };
        self.state = state;
        self.rng = rng;
        self.tick = target_tick;
        self.outcome.clear();
        true
    }

    pub fn replay_to(&mut self, target_tick: Tick) -> bool {
        self.rewind_to(target_tick)
    }

    pub fn state_at(&self, target_tick: Tick) -> Option<G::State> {
        self.history
            .restore(&self.game, target_tick)
            .map(|(state, _)| state)
    }

    pub fn fork_at(&self, target_tick: Tick) -> Option<Self>
    where
        G: Clone,
    {
        let mut fork = self.clone();
        fork.rewind_to(target_tick).then_some(fork)
    }
}

#[cfg(kani)]
mod proofs {
    use crate::buffer::FixedVec;
    use crate::game::Game;
    use crate::rng::DeterministicRng;
    use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};

    use super::SessionKernel;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct CounterGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct CounterState {
        value: u8,
        terminal: bool,
    }

    impl Game for CounterGame {
        type State = CounterState;
        type Action = u8;
        type PlayerObservation = CounterState;
        type SpectatorObservation = CounterState;
        type WorldView = CounterState;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 2>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn name(&self) -> &'static str {
            "counter"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init(&self, _seed: Seed) -> Self::State {
            CounterState {
                value: 0,
                terminal: false,
            }
        }

        fn is_terminal(&self, state: &Self::State) -> bool {
            state.terminal
        }

        fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            if !state.terminal {
                out.push(0).unwrap();
            }
        }

        fn legal_actions(
            &self,
            _state: &Self::State,
            _player: PlayerId,
            out: &mut Self::ActionBuf,
        ) {
            out.clear();
            out.push(0).unwrap();
            out.push(1).unwrap();
        }

        fn observe_player(
            &self,
            state: &Self::State,
            _player: PlayerId,
        ) -> Self::PlayerObservation {
            *state
        }

        fn observe_spectator(&self, state: &Self::State) -> Self::SpectatorObservation {
            *state
        }

        fn world_view(&self, state: &Self::State) -> Self::WorldView {
            *state
        }

        fn step_in_place(
            &self,
            state: &mut Self::State,
            joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut StepOutcome<Self::RewardBuf>,
        ) {
            let delta = if joint_actions.is_empty() {
                0
            } else {
                joint_actions.as_slice()[0].action
            };
            state.value = state.value.saturating_add(delta);
            state.terminal = state.value >= 2;
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: i64::from(delta),
                })
                .unwrap();
            out.termination = if state.terminal {
                Termination::Terminal { winner: Some(0) }
            } else {
                Termination::Ongoing
            };
        }

        fn state_invariant(&self, state: &Self::State) -> bool {
            !state.terminal || state.value >= 2
        }
    }

    #[kani::proof]
    #[kani::unwind(16)]
    fn rewind_restores_prior_state() {
        type Session = SessionKernel<CounterGame, super::FixedHistory<CounterGame, 8, 2, 1>>;

        let mut session = Session::new(CounterGame, 1);
        let mut action = FixedVec::<PlayerAction<u8>, 1>::default();
        action
            .push(PlayerAction {
                player: 0,
                action: 1,
            })
            .unwrap();
        session.step_with_joint_actions(&action);
        session.step_with_joint_actions(&action);
        assert!(session.rewind_to(1));
        assert_eq!(session.state().value, 1);
    }
}
