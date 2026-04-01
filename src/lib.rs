//! Deterministic game engine core with compact codecs, verification hooks, and render adapters.

pub mod core;
#[cfg(feature = "builtin")]
pub mod registry;
#[cfg(feature = "proof")]
pub mod proof;

pub mod buffer;
pub mod compact;
pub mod game;
#[cfg(feature = "builtin")]
pub mod builtin;
pub mod math;
#[cfg(feature = "parallel")]
pub mod parallel;
#[cfg(feature = "physics")]
pub mod physics;
pub mod policy;
#[cfg(feature = "render")]
pub mod render;
pub mod rng;
pub mod session;
pub mod types;
pub mod verification;
#[cfg(feature = "cli")]
pub mod cli;

pub use buffer::{BitWords, Buffer, CapacityError, FixedVec};
pub use compact::CompactSpec;
pub use game::Game;
pub use policy::{FirstLegalPolicy, FnPolicy, Policy, RandomPolicy, ScriptedPolicy};
pub use rng::{DeterministicRng, SplitMix64};
pub use session::{
    DynamicHistory, FixedHistory, HistorySnapshot, HistoryStore, InteractiveSession, Session,
    SessionKernel,
};
pub use types::{
    DynamicReplayTrace, PlayerAction, PlayerId, PlayerReward, ReplayStep, ReplayTrace, Reward,
    Seed, StepOutcome, Termination, Tick, stable_hash,
};
