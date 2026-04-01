//! Session kernel, history stores, and replay/rewind utilities.

use core::fmt::Debug;
use std::collections::VecDeque;

use crate::buffer::{Buffer, default_array};
use crate::game::Game;
use crate::policy::Policy;
use crate::rng::DeterministicRng;
use crate::types::{DynamicReplayTrace, PlayerAction, ReplayTrace, Seed, StepOutcome, Tick};

/// Saved checkpoint used by history implementations for rewind.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct HistorySnapshot<S> {
    /// Tick represented by this snapshot.
    pub tick: Tick,
    /// Cloned game state.
    pub state: S,
    /// RNG state associated with `state`.
    pub rng: DeterministicRng,
}

/// Storage backend for session traces and rewind snapshots.
pub trait HistoryStore<G: Game>: Clone {
    /// Trace representation emitted by this history backend.
    type Trace: Clone + Debug + Eq + PartialEq;

    /// Creates a history store from initial session state.
    fn from_seed(seed: Seed, initial_state: &G::State, initial_rng: DeterministicRng) -> Self;
    /// Resets history to initial session state.
    fn reset(&mut self, seed: Seed, initial_state: &G::State, initial_rng: DeterministicRng);
    /// Records one transition and optional snapshot.
    fn record(
        &mut self,
        tick: Tick,
        state: &G::State,
        rng: DeterministicRng,
        actions: &G::JointActionBuf,
        outcome: &StepOutcome<G::RewardBuf>,
    );
    /// Returns recorded transition count.
    fn len(&self) -> usize;
    /// Returns whether no transitions are recorded.
    fn is_empty(&self) -> bool;
    /// Returns immutable trace view.
    fn trace(&self) -> &Self::Trace;
    /// Consumes history and returns owned trace.
    fn into_trace(self) -> Self::Trace;
    /// Restores state/RNG at `target_tick` when available.
    fn restore(&self, game: &G, target_tick: Tick) -> Option<(G::State, DeterministicRng)>;
}

/// Dynamically-sized history with bounded checkpoint deque.
#[derive(Debug, Eq, PartialEq)]
pub struct DynamicHistory<G: Game, const SNAPSHOTS: usize, const SNAP_EVERY: usize> {
    seed: Seed,
    initial_state: G::State,
    initial_rng: DeterministicRng,
    trace: DynamicReplayTrace<G::JointActionBuf, G::RewardBuf>,
    snapshots: VecDeque<HistorySnapshot<G::State>>,
}

impl<G: Game, const SNAPSHOTS: usize, const SNAP_EVERY: usize> Clone
    for DynamicHistory<G, SNAPSHOTS, SNAP_EVERY>
{
    fn clone(&self) -> Self {
        Self {
            seed: self.seed,
            initial_state: self.initial_state.clone(),
            initial_rng: self.initial_rng,
            trace: self.trace.clone(),
            snapshots: self.snapshots.clone(),
        }
    }
}

impl<G: Game, const SNAPSHOTS: usize, const SNAP_EVERY: usize>
    DynamicHistory<G, SNAPSHOTS, SNAP_EVERY>
{
    fn store_snapshot(&mut self, tick: Tick, state: &G::State, rng: DeterministicRng) {
        if SNAPSHOTS == 0 || SNAP_EVERY == 0 {
            return;
        }
        if !tick.is_multiple_of(SNAP_EVERY as u64) {
            return;
        }
        if self.snapshots.len() == SNAPSHOTS {
            let _ = self.snapshots.pop_front();
        }
        self.snapshots.push_back(HistorySnapshot {
            tick,
            state: state.clone(),
            rng,
        });
    }

    fn best_snapshot(&self, target_tick: Tick) -> Option<&HistorySnapshot<G::State>> {
        self.snapshots
            .iter()
            .filter(|snapshot| snapshot.tick <= target_tick)
            .max_by_key(|snapshot| snapshot.tick)
    }
}

impl<G: Game, const SNAPSHOTS: usize, const SNAP_EVERY: usize> HistoryStore<G>
    for DynamicHistory<G, SNAPSHOTS, SNAP_EVERY>
{
    type Trace = DynamicReplayTrace<G::JointActionBuf, G::RewardBuf>;

    fn from_seed(seed: Seed, initial_state: &G::State, initial_rng: DeterministicRng) -> Self {
        let mut snapshots = VecDeque::with_capacity(SNAPSHOTS);
        if SNAPSHOTS > 0 {
            snapshots.push_back(HistorySnapshot {
                tick: 0,
                state: initial_state.clone(),
                rng: initial_rng,
            });
        }
        Self {
            seed,
            initial_state: initial_state.clone(),
            initial_rng,
            trace: DynamicReplayTrace::new(seed),
            snapshots,
        }
    }

    fn reset(&mut self, seed: Seed, initial_state: &G::State, initial_rng: DeterministicRng) {
        self.seed = seed;
        self.initial_state = initial_state.clone();
        self.initial_rng = initial_rng;
        self.trace.clear(seed);
        self.snapshots.clear();
        if SNAPSHOTS > 0 {
            self.snapshots.push_back(HistorySnapshot {
                tick: 0,
                state: initial_state.clone(),
                rng: initial_rng,
            });
        }
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
        let mut index = start_tick as usize;
        while index < self.trace.steps.len() {
            let step = &self.trace.steps[index];
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

/// Fixed-capacity history with ring-buffer checkpoints.
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

/// Deterministic session kernel for stepping, tracing, and rewinding games.
#[derive(Clone, Debug)]
pub struct SessionKernel<G: Game, H: HistoryStore<G>> {
    game: G,
    params: G::Params,
    state: G::State,
    rng: DeterministicRng,
    tick: Tick,
    history: H,
    players_to_act: G::PlayerBuf,
    legal_actions: G::ActionBuf,
    joint_actions: G::JointActionBuf,
    outcome: StepOutcome<G::RewardBuf>,
}

/// Default dynamic-history session alias.
pub type Session<G> = SessionKernel<G, DynamicHistory<G, 512, 8>>;
/// Interactive dynamic-history session alias.
pub type InteractiveSession<G> = SessionKernel<G, DynamicHistory<G, 128, 8>>;

impl<G: Game, H: HistoryStore<G>> SessionKernel<G, H> {
    /// Creates a new session initialized from `seed`.
    pub fn new(game: G, seed: Seed) -> Self {
        let params = game.default_params();
        Self::new_with_params(game, seed, params)
    }

    /// Creates a new session initialized from `seed` and explicit params.
    pub fn new_with_params(game: G, seed: Seed, params: G::Params) -> Self {
        let state = game.init_with_params(seed, &params);
        assert!(game.state_invariant(&state));
        let rng = DeterministicRng::from_seed_and_stream(seed, 1);
        let history = H::from_seed(seed, &state, rng);
        Self {
            game,
            params,
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

    /// Resets session state and history to `seed`.
    pub fn reset(&mut self, seed: Seed) {
        let params = self.params.clone();
        self.reset_with_params(seed, params);
    }

    /// Resets session state/history to `seed` and updates active params.
    pub fn reset_with_params(&mut self, seed: Seed, params: G::Params) {
        self.params = params;
        self.state = self.game.init_with_params(seed, &self.params);
        self.rng = DeterministicRng::from_seed_and_stream(seed, 1);
        self.tick = 0;
        self.history.reset(seed, &self.state, self.rng);
        self.players_to_act.clear();
        self.legal_actions.clear();
        self.joint_actions.clear();
        self.outcome.clear();
    }

    /// Returns the game instance.
    pub fn game(&self) -> &G {
        &self.game
    }

    /// Returns active parameter bundle used by resets and initial state creation.
    pub fn params(&self) -> &G::Params {
        &self.params
    }

    /// Returns current game state.
    pub fn state(&self) -> &G::State {
        &self.state
    }

    /// Returns current tick.
    pub fn current_tick(&self) -> Tick {
        self.tick
    }

    /// Returns current RNG snapshot.
    pub fn rng(&self) -> DeterministicRng {
        self.rng
    }

    /// Returns immutable trace view.
    pub fn trace(&self) -> &H::Trace {
        self.history.trace()
    }

    /// Consumes session and returns owned trace.
    pub fn into_trace(self) -> H::Trace {
        self.history.into_trace()
    }

    /// Returns whether current state is terminal.
    pub fn is_terminal(&self) -> bool {
        self.game.is_terminal(&self.state)
    }

    /// Returns player-local observation.
    pub fn player_observation(&self, player: usize) -> G::Obs {
        self.game.observe_player(&self.state, player)
    }

    /// Returns spectator observation.
    pub fn spectator_observation(&self) -> G::Obs {
        self.game.observe_spectator(&self.state)
    }

    /// Returns world/debug view.
    pub fn world_view(&self) -> G::WorldView {
        self.game.world_view(&self.state)
    }

    /// Returns legal actions for `player` in current state.
    pub fn legal_actions_for(&mut self, player: usize) -> &[G::Action] {
        self.game
            .legal_actions(&self.state, player, &mut self.legal_actions);
        self.legal_actions.as_slice()
    }

    #[inline(always)]
    fn step_core(&mut self, actions: &G::JointActionBuf) {
        assert!(
            !self.game.is_terminal(&self.state),
            "cannot step a terminal session",
        );
        self.outcome.clear();
        self.game
            .step_in_place(&mut self.state, actions, &mut self.rng, &mut self.outcome);
        self.tick += 1;
        self.outcome.tick = self.tick;
    }

    #[inline(always)]
    fn record_step(&mut self, actions: &G::JointActionBuf) {
        self.history
            .record(self.tick, &self.state, self.rng, actions, &self.outcome);
    }

    fn collect_policy_actions(&mut self, policies: &mut [&mut dyn Policy<G>]) {
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
    }

    #[inline(always)]
    fn step_staged_joint_actions(&mut self) -> &StepOutcome<G::RewardBuf> {
        assert!(
            !self.game.is_terminal(&self.state),
            "cannot step a terminal session",
        );
        self.outcome.clear();
        self.game.step_in_place(
            &mut self.state,
            &self.joint_actions,
            &mut self.rng,
            &mut self.outcome,
        );
        self.tick += 1;
        self.outcome.tick = self.tick;
        self.history.record(
            self.tick,
            &self.state,
            self.rng,
            &self.joint_actions,
            &self.outcome,
        );
        &self.outcome
    }

    #[inline(always)]
    fn step_staged_joint_actions_checked(&mut self) -> &StepOutcome<G::RewardBuf> {
        assert!(
            !self.game.is_terminal(&self.state),
            "cannot step a terminal session",
        );
        assert!(self.game.state_invariant(&self.state));
        for action in self.joint_actions.as_slice() {
            assert!(self.game.action_invariant(&action.action));
        }

        let pre_state = self.state.clone();
        self.outcome.clear();
        self.game.step_in_place(
            &mut self.state,
            &self.joint_actions,
            &mut self.rng,
            &mut self.outcome,
        );
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
            &self.joint_actions,
            &self.state,
            &self.outcome
        ));

        self.history.record(
            self.tick,
            &self.state,
            self.rng,
            &self.joint_actions,
            &self.outcome,
        );
        &self.outcome
    }

    /// Steps using externally supplied action slice.
    pub fn step(&mut self, actions: &[PlayerAction<G::Action>]) -> &StepOutcome<G::RewardBuf> {
        self.joint_actions.clear();
        self.joint_actions
            .extend_from_slice(actions)
            .expect("joint action buffer capacity exceeded");
        self.step_staged_joint_actions()
    }

    /// Steps using externally supplied action slice with contract checks.
    pub fn step_checked(
        &mut self,
        actions: &[PlayerAction<G::Action>],
    ) -> &StepOutcome<G::RewardBuf> {
        self.joint_actions.clear();
        self.joint_actions
            .extend_from_slice(actions)
            .expect("joint action buffer capacity exceeded");
        self.step_staged_joint_actions_checked()
    }

    /// Steps with prebuilt joint-action buffer.
    #[inline(always)]
    pub fn step_with_joint_actions(
        &mut self,
        actions: &G::JointActionBuf,
    ) -> &StepOutcome<G::RewardBuf> {
        self.step_core(actions);
        self.record_step(actions);
        &self.outcome
    }

    /// Steps with contract checks enabled.
    pub fn step_with_joint_actions_checked(
        &mut self,
        actions: &G::JointActionBuf,
    ) -> &StepOutcome<G::RewardBuf> {
        assert!(self.game.state_invariant(&self.state));
        for action in actions.as_slice() {
            assert!(self.game.action_invariant(&action.action));
        }

        let pre_state = self.state.clone();
        self.step_core(actions);

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

        self.record_step(actions);
        &self.outcome
    }

    /// Collects actions from policies and steps once.
    pub fn step_with_policies(
        &mut self,
        policies: &mut [&mut dyn Policy<G>],
    ) -> &StepOutcome<G::RewardBuf> {
        self.collect_policy_actions(policies);
        self.step_staged_joint_actions()
    }

    /// Collects actions from policies and steps once with checks.
    pub fn step_with_policies_checked(
        &mut self,
        policies: &mut [&mut dyn Policy<G>],
    ) -> &StepOutcome<G::RewardBuf> {
        self.collect_policy_actions(policies);
        self.step_staged_joint_actions_checked()
    }

    /// Runs until terminal state or `max_ticks` is reached.
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

    /// Runs checked stepping until terminal state or `max_ticks`.
    pub fn run_until_terminal_checked(
        &mut self,
        policies: &mut [&mut dyn Policy<G>],
        max_ticks: usize,
    ) -> &H::Trace {
        while !self.is_terminal() && (self.tick as usize) < max_ticks {
            self.step_with_policies_checked(policies);
        }
        self.trace()
    }

    /// Rewinds session state to `target_tick` when restorable.
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

    /// Alias of `rewind_to` for replay-oriented call sites.
    pub fn replay_to(&mut self, target_tick: Tick) -> bool {
        self.rewind_to(target_tick)
    }

    /// Returns reconstructed state at `target_tick`.
    pub fn state_at(&self, target_tick: Tick) -> Option<G::State> {
        self.history
            .restore(&self.game, target_tick)
            .map(|(state, _)| state)
    }

    /// Returns a cloned session fork rewound to `target_tick`.
    pub fn fork_at(&self, target_tick: Tick) -> Option<Self>
    where
        G: Clone,
    {
        let mut fork = self.clone();
        fork.rewind_to(target_tick).then_some(fork)
    }
}

#[cfg(test)]
mod tests {
    use crate::buffer::FixedVec;
    use crate::game::Game;
    use crate::rng::DeterministicRng;
    use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};

    use super::{DynamicHistory, SessionKernel};

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct SpinnerGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct SpinnerState {
        tick: u16,
    }

    impl Game for SpinnerGame {
        type Params = ();
        type State = SpinnerState;
        type Action = u8;
        type Obs = SpinnerState;
        type WorldView = SpinnerState;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 1>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn name(&self) -> &'static str {
            "spinner"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init_with_params(&self, _seed: Seed, _params: &Self::Params) -> Self::State {
            SpinnerState { tick: 0 }
        }

        fn is_terminal(&self, _state: &Self::State) -> bool {
            false
        }

        fn players_to_act(&self, _state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            out.push(0).unwrap();
        }

        fn legal_actions(
            &self,
            _state: &Self::State,
            _player: PlayerId,
            out: &mut Self::ActionBuf,
        ) {
            out.clear();
            out.push(0).unwrap();
        }

        fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::Obs {
            *state
        }

        fn observe_spectator(&self, state: &Self::State) -> Self::Obs {
            *state
        }

        fn world_view(&self, state: &Self::State) -> Self::WorldView {
            *state
        }

        fn step_in_place(
            &self,
            state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut StepOutcome<Self::RewardBuf>,
        ) {
            state.tick += 1;
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 1,
                })
                .unwrap();
            out.termination = Termination::Ongoing;
        }
    }

    #[test]
    fn dynamic_history_records_long_sessions_without_overflow() {
        type Session = SessionKernel<SpinnerGame, DynamicHistory<SpinnerGame, 16, 4>>;

        let mut session = Session::new(SpinnerGame, 7);
        let action = [PlayerAction {
            player: 0,
            action: 0,
        }];
        for _ in 0..600 {
            session.step(&action);
        }

        assert_eq!(session.current_tick(), 600);
        assert_eq!(session.trace().len(), 600);
        assert_eq!(session.state_at(512), Some(SpinnerState { tick: 512 }));
        assert!(session.rewind_to(384));
        assert_eq!(session.state(), &SpinnerState { tick: 384 });
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
        type Params = ();
        type State = CounterState;
        type Action = u8;
        type Obs = CounterState;
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

        fn init_with_params(&self, _seed: Seed, _params: &Self::Params) -> Self::State {
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

        fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::Obs {
            *state
        }

        fn observe_spectator(&self, state: &Self::State) -> Self::Obs {
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
