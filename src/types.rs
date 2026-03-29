use std::hash::{Hash, Hasher};

pub type Reward = i64;
pub type Tick = u64;
pub type PlayerId = usize;
pub type Seed = u64;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct PlayerReward {
    pub player: PlayerId,
    pub reward: Reward,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StepOutcome {
    pub tick: Tick,
    pub rewards: Vec<PlayerReward>,
    pub termination: Termination,
}

impl StepOutcome {
    pub fn with_player_capacity(players: usize) -> Self {
        Self {
            tick: 0,
            rewards: Vec::with_capacity(players.max(1)),
            termination: Termination::Ongoing,
        }
    }

    pub fn clear(&mut self) {
        self.tick = 0;
        self.rewards.clear();
        self.termination = Termination::Ongoing;
    }

    pub fn reward_for(&self, player: PlayerId) -> Reward {
        self.rewards
            .iter()
            .find(|reward| reward.player == player)
            .map(|reward| reward.reward)
            .unwrap_or(0)
    }

    pub fn is_terminal(&self) -> bool {
        self.termination.is_terminal()
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ReplayStep<A> {
    pub tick: Tick,
    pub actions: Vec<PlayerAction<A>>,
    pub rewards: Vec<PlayerReward>,
    pub termination: Termination,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ReplayTrace<A> {
    pub seed: Seed,
    pub steps: Vec<ReplayStep<A>>,
}

impl<A> ReplayTrace<A> {
    pub fn new(seed: Seed) -> Self {
        Self {
            seed,
            steps: Vec::new(),
        }
    }

    pub fn with_capacity(seed: Seed, capacity: usize) -> Self {
        Self {
            seed,
            steps: Vec::with_capacity(capacity),
        }
    }
}

impl<A: Clone> ReplayTrace<A> {
    pub fn record(&mut self, tick: Tick, actions: &[PlayerAction<A>], outcome: &StepOutcome) {
        self.steps.push(ReplayStep {
            tick,
            actions: actions.to_vec(),
            rewards: outcome.rewards.clone(),
            termination: outcome.termination,
        });
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
