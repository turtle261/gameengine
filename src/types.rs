//! Core scalar types and replay data structures used across the engine.

use core::hash::{Hash, Hasher};

use crate::buffer::{Buffer, FixedVec};

/// Scalar reward type used by games.
pub type Reward = i64;
/// Monotonic simulation tick counter.
pub type Tick = u64;
/// Stable player identifier within one game.
pub type PlayerId = usize;
/// Deterministic seed type.
pub type Seed = u64;

/// Reward assigned to one player for a single transition.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct PlayerReward {
    /// Recipient player id.
    pub player: PlayerId,
    /// Reward value for that player.
    pub reward: Reward,
}

/// Action submitted by a specific player.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PlayerAction<A> {
    /// Acting player id.
    pub player: PlayerId,
    /// Concrete chosen action.
    pub action: A,
}

/// Episode termination state after a step.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Termination {
    /// Episode continues.
    #[default]
    Ongoing,
    /// Episode reached a terminal state.
    Terminal {
        /// Winner id for terminal outcomes, when applicable.
        winner: Option<PlayerId>,
    },
}

impl Termination {
    /// Returns `true` when the outcome is terminal.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal { .. })
    }
}

/// Output bundle from one transition.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StepOutcome<R> {
    /// Tick at which this outcome was produced.
    pub tick: Tick,
    /// Per-player rewards.
    pub rewards: R,
    /// Termination state.
    pub termination: Termination,
}

impl<R> Default for StepOutcome<R>
where
    R: Default,
{
    fn default() -> Self {
        Self {
            tick: 0,
            rewards: R::default(),
            termination: Termination::Ongoing,
        }
    }
}

impl<R> StepOutcome<R>
where
    R: Buffer<Item = PlayerReward>,
{
    /// Resets outcome to default ongoing state.
    pub fn clear(&mut self) {
        self.tick = 0;
        self.rewards.clear();
        self.termination = Termination::Ongoing;
    }

    /// Returns reward for `player`, or `0` when no entry exists.
    pub fn reward_for(&self, player: PlayerId) -> Reward {
        let rewards = self.rewards.as_slice();
        let mut index = 0usize;
        while index < rewards.len() {
            let reward = rewards[index];
            if reward.player == player {
                return reward.reward;
            }
            index += 1;
        }
        0
    }

    /// Returns whether this outcome is terminal.
    pub fn is_terminal(&self) -> bool {
        self.termination.is_terminal()
    }
}

/// One recorded replay step.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ReplayStep<JA, R> {
    /// Tick at which step was recorded.
    pub tick: Tick,
    /// Joint action applied at `tick`.
    pub actions: JA,
    /// Reward bundle emitted by the transition.
    pub rewards: R,
    /// Termination state after the transition.
    pub termination: Termination,
}

/// Fixed-capacity replay trace.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ReplayTrace<JA, R, const LOG: usize>
where
    ReplayStep<JA, R>: Default,
{
    /// Seed used to initialize the session.
    pub seed: Seed,
    /// Recorded transition log.
    pub steps: FixedVec<ReplayStep<JA, R>, LOG>,
}

impl<JA, R, const LOG: usize> ReplayTrace<JA, R, LOG>
where
    ReplayStep<JA, R>: Default,
{
    /// Creates an empty trace initialized with `seed`.
    pub fn new(seed: Seed) -> Self {
        Self {
            seed,
            steps: FixedVec::default(),
        }
    }

    /// Clears the trace and updates seed metadata.
    pub fn clear(&mut self, seed: Seed) {
        self.seed = seed;
        self.steps.clear();
    }

    /// Returns number of recorded steps.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Returns whether no steps are recorded.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// Dynamically-sized replay trace.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct DynamicReplayTrace<JA, R> {
    /// Seed used to initialize the session.
    pub seed: Seed,
    /// Recorded transition log.
    pub steps: Vec<ReplayStep<JA, R>>,
}

impl<JA, R> DynamicReplayTrace<JA, R> {
    /// Creates an empty dynamic trace.
    pub fn new(seed: Seed) -> Self {
        Self {
            seed,
            steps: Vec::new(),
        }
    }

    /// Clears the trace and updates seed metadata.
    pub fn clear(&mut self, seed: Seed) {
        self.seed = seed;
        self.steps.clear();
    }

    /// Returns number of recorded steps.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Returns whether no steps are recorded.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

impl<JA, R> DynamicReplayTrace<JA, R>
where
    JA: Clone,
    R: Clone,
{
    /// Appends one replay step cloned from the given references.
    pub fn record(&mut self, tick: Tick, actions: &JA, rewards: &R, termination: Termination) {
        self.steps.push(ReplayStep {
            tick,
            actions: actions.clone(),
            rewards: rewards.clone(),
            termination,
        });
    }
}

impl<JA, R, const LOG: usize> ReplayTrace<JA, R, LOG>
where
    JA: Clone + Default,
    R: Clone + Default,
{
    /// Appends one replay step to the fixed-capacity log.
    pub fn record(&mut self, tick: Tick, actions: &JA, rewards: &R, termination: Termination) {
        self.steps
            .push(ReplayStep {
                tick,
                actions: actions.clone(),
                rewards: rewards.clone(),
                termination,
            })
            .expect("replay trace capacity exceeded");
    }
}

#[derive(Default)]
struct StableHasher {
    state: u64,
}

impl StableHasher {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self {
            state: Self::OFFSET,
        }
    }
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(Self::PRIME);
        }
    }
}

/// Computes a stable 64-bit hash using an internal FNV-1a variant.
pub fn stable_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = StableHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(kani)]
mod proofs {
    use super::{PlayerReward, ReplayTrace, StepOutcome, Termination};
    use crate::buffer::FixedVec;

    #[kani::proof]
    fn step_outcome_reward_lookup_defaults_to_zero() {
        let mut outcome = StepOutcome::<FixedVec<PlayerReward, 2>>::default();
        assert_eq!(outcome.reward_for(0), 0);
        outcome
            .rewards
            .push(PlayerReward {
                player: 1,
                reward: 7,
            })
            .unwrap();
        assert_eq!(outcome.reward_for(0), 0);
        assert_eq!(outcome.reward_for(1), 7);
    }

    #[kani::proof]
    fn replay_trace_records_steps() {
        let mut trace = ReplayTrace::<FixedVec<u8, 2>, FixedVec<PlayerReward, 2>, 2>::new(3);
        let mut actions = FixedVec::default();
        actions.push(9).unwrap();
        let mut rewards = FixedVec::default();
        rewards
            .push(PlayerReward {
                player: 0,
                reward: 1,
            })
            .unwrap();
        trace.record(1, &actions, &rewards, Termination::Ongoing);
        assert_eq!(trace.len(), 1);
    }
}
