use core::hash::{Hash, Hasher};

use crate::buffer::{Buffer, FixedVec};

pub type Reward = i64;
pub type Tick = u64;
pub type PlayerId = usize;
pub type Seed = u64;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct PlayerReward {
    pub player: PlayerId,
    pub reward: Reward,
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PlayerAction<A> {
    pub player: PlayerId,
    pub action: A,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Termination {
    #[default]
    Ongoing,
    Terminal {
        winner: Option<PlayerId>,
    },
}

impl Termination {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal { .. })
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StepOutcome<R> {
    pub tick: Tick,
    pub rewards: R,
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
    pub fn clear(&mut self) {
        self.tick = 0;
        self.rewards.clear();
        self.termination = Termination::Ongoing;
    }

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

    pub fn is_terminal(&self) -> bool {
        self.termination.is_terminal()
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ReplayStep<JA, R> {
    pub tick: Tick,
    pub actions: JA,
    pub rewards: R,
    pub termination: Termination,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ReplayTrace<JA, R, const LOG: usize>
where
    ReplayStep<JA, R>: Default,
{
    pub seed: Seed,
    pub steps: FixedVec<ReplayStep<JA, R>, LOG>,
}

impl<JA, R, const LOG: usize> ReplayTrace<JA, R, LOG>
where
    ReplayStep<JA, R>: Default,
{
    pub fn new(seed: Seed) -> Self {
        Self {
            seed,
            steps: FixedVec::default(),
        }
    }

    pub fn clear(&mut self, seed: Seed) {
        self.seed = seed;
        self.steps.clear();
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct DynamicReplayTrace<JA, R> {
    pub seed: Seed,
    pub steps: Vec<ReplayStep<JA, R>>,
}

impl<JA, R> DynamicReplayTrace<JA, R> {
    pub fn new(seed: Seed) -> Self {
        Self {
            seed,
            steps: Vec::new(),
        }
    }

    pub fn clear(&mut self, seed: Seed) {
        self.seed = seed;
        self.steps.clear();
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

impl<JA, R> DynamicReplayTrace<JA, R>
where
    JA: Clone,
    R: Clone,
{
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
