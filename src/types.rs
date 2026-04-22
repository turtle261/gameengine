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

/// Validation error for checked joint-action profiles.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JointActionProfileError {
    /// A listed player id was outside `0..player_count`.
    PlayerOutOfRange {
        /// Rejected player id.
        player: PlayerId,
        /// Maximum valid player count.
        player_count: usize,
    },
    /// A player id appeared more than once in the profile.
    DuplicatePlayer {
        /// Repeated player id.
        player: PlayerId,
    },
}

/// Checked wrapper around a well-formed joint-action buffer.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct JointActionProfile<JA> {
    actions: JA,
}

impl<JA> JointActionProfile<JA> {
    /// Returns the wrapped joint-action buffer by shared reference.
    pub fn as_inner(&self) -> &JA {
        &self.actions
    }

    /// Consumes this profile and returns the wrapped buffer.
    pub fn into_inner(self) -> JA {
        self.actions
    }
}

impl<JA, A> JointActionProfile<JA>
where
    JA: Buffer<Item = PlayerAction<A>>,
{
    /// Validates one joint-action buffer against a player range and uniqueness law.
    pub fn try_new(actions: JA, player_count: usize) -> Result<Self, JointActionProfileError> {
        let entries = actions.as_slice();
        let mut index = 0usize;
        while index < entries.len() {
            let player = entries[index].player;
            if player >= player_count {
                return Err(JointActionProfileError::PlayerOutOfRange {
                    player,
                    player_count,
                });
            }
            let mut prior = 0usize;
            while prior < index {
                if entries[prior].player == player {
                    return Err(JointActionProfileError::DuplicatePlayer { player });
                }
                prior += 1;
            }
            index += 1;
        }
        Ok(Self { actions })
    }

    /// Returns the validated profile as an action slice.
    pub fn as_slice(&self) -> &[PlayerAction<A>] {
        self.actions.as_slice()
    }
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

/// Kernel-layer outcome emitted by one transition: per-player rewards
/// together with termination state.
///
/// Per specification §2.3 the `KernelOutcome<R>` pair is exactly
/// `(rewards, termination)` with default `(R::default(), Ongoing)`. It
/// carries no tick; tick attribution lives alongside the cached outcome
/// inside `SessionStepRecord<JA,R>` and at the session level as
/// `session.current_tick()`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct KernelOutcome<R> {
    /// Per-player rewards produced by the transition.
    pub rewards: R,
    /// Termination state after the transition.
    pub termination: Termination,
}

impl<R> Default for KernelOutcome<R>
where
    R: Default,
{
    fn default() -> Self {
        Self {
            rewards: R::default(),
            termination: Termination::Ongoing,
        }
    }
}

impl<R> KernelOutcome<R>
where
    R: Buffer<Item = PlayerReward>,
{
    /// Resets the outcome to its ongoing default.
    ///
    /// Per specification §2.3 this invokes `clear()` on the reward buffer
    /// rather than reconstructing it, which preserves the underlying
    /// allocation for fixed-capacity buffer implementations.
    pub fn clear(&mut self) {
        self.rewards.clear();
        self.termination = Termination::Ongoing;
    }

    /// Returns reward for `player`, or `0` when no entry exists.
    ///
    /// Scans left-to-right and returns the first matching reward entry.
    /// Under the reward-buffer well-formedness contract (each player id
    /// appears at most once), this is the unique entry for that player;
    /// the left-to-right rule totalizes the function on malformed
    /// buffers and is not load-bearing semantic behavior.
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

/// One recorded session step: tick, joint action, and the cached
/// `KernelOutcome` emitted by that transition.
///
/// Per specification §2.4 this is the canonical per-step record stored
/// in `ReplayTrace` and `DynamicReplayTrace`.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct SessionStepRecord<JA, R> {
    /// Tick at which this step was recorded.
    pub tick: Tick,
    /// Joint action applied at `tick`.
    pub actions: JA,
    /// Kernel outcome emitted by the transition at `tick`.
    pub outcome: KernelOutcome<R>,
}

/// Fixed-capacity replay trace.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ReplayTrace<JA, R, const LOG: usize>
where
    SessionStepRecord<JA, R>: Default,
{
    /// Seed used to initialize the session.
    pub seed: Seed,
    /// Recorded transition log.
    pub steps: FixedVec<SessionStepRecord<JA, R>, LOG>,
}

impl<JA, R, const LOG: usize> ReplayTrace<JA, R, LOG>
where
    SessionStepRecord<JA, R>: Default,
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
    pub steps: Vec<SessionStepRecord<JA, R>>,
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
    /// Appends one session step cloned from the given references.
    ///
    /// Per specification §2.4 this records `(tick, actions.clone(),
    /// outcome.clone())` as one `SessionStepRecord<JA,R>`.
    pub fn record(&mut self, tick: Tick, actions: &JA, outcome: &KernelOutcome<R>) {
        self.steps.push(SessionStepRecord {
            tick,
            actions: actions.clone(),
            outcome: outcome.clone(),
        });
    }
}

impl<JA, R, const LOG: usize> ReplayTrace<JA, R, LOG>
where
    JA: Clone + Default,
    R: Clone + Default,
{
    /// Appends one session step to the fixed-capacity log.
    ///
    /// Panics iff the log's compile-time capacity is exceeded, per
    /// specification §2.4.
    pub fn record(&mut self, tick: Tick, actions: &JA, outcome: &KernelOutcome<R>) {
        self.steps
            .push(SessionStepRecord {
                tick,
                actions: actions.clone(),
                outcome: outcome.clone(),
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

/// Computes the legacy regression 64-bit hash using an internal FNV-1a variant.
pub fn stable_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = StableHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::{JointActionProfile, JointActionProfileError, PlayerAction};
    use crate::buffer::FixedVec;

    #[test]
    fn joint_action_profile_rejects_duplicate_players() {
        let mut actions = FixedVec::<PlayerAction<u8>, 2>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: 1,
            })
            .unwrap();
        actions
            .push(PlayerAction {
                player: 0,
                action: 2,
            })
            .unwrap();
        assert_eq!(
            JointActionProfile::try_new(actions, 2),
            Err(JointActionProfileError::DuplicatePlayer { player: 0 })
        );
    }

    #[test]
    fn joint_action_profile_accepts_well_formed_actions() {
        let mut actions = FixedVec::<PlayerAction<u8>, 2>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: 1,
            })
            .unwrap();
        actions
            .push(PlayerAction {
                player: 1,
                action: 2,
            })
            .unwrap();
        let profile = JointActionProfile::try_new(actions, 2).unwrap();
        assert_eq!(profile.as_slice().len(), 2);
    }
}

#[cfg(kani)]
mod proofs {
    use super::{KernelOutcome, PlayerReward, ReplayTrace, Termination};
    use crate::buffer::FixedVec;

    #[kani::proof]
    fn step_outcome_reward_lookup_defaults_to_zero() {
        let mut outcome = KernelOutcome::<FixedVec<PlayerReward, 2>>::default();
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
        let mut outcome = KernelOutcome::<FixedVec<PlayerReward, 2>>::default();
        outcome
            .rewards
            .push(PlayerReward {
                player: 0,
                reward: 1,
            })
            .unwrap();
        outcome.termination = Termination::Ongoing;
        trace.record(1, &actions, &outcome);
        assert_eq!(trace.len(), 1);
    }
}
