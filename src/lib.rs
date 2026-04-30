//! Deterministic game engine core with compact codecs, verification hooks, and render adapters.

pub mod core;
pub mod proof;
#[cfg(feature = "builtin")]
pub mod registry;

pub mod buffer;
#[cfg(feature = "builtin")]
pub mod builtin;
#[cfg(feature = "cli")]
pub mod cli;
pub mod compact;
pub mod game;
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

pub use buffer::{BitWords, Buffer, CapacityError, FixedVec};
pub use compact::CompactSpec;
pub use core::env::{
    ActionToken, ActionTokenError, AixiEnvironment, AixiFrontDoorError, DefaultEnvironment,
    EnvError, Environment, Percept,
};
pub use core::serialization::{
    BitStream, BitStreamBits, serialize_action, serialize_history, serialize_percept,
};
pub use core::single_player::SinglePlayerGame;
pub use game::{
    CompactCodec, ContractSurface, Game, GameAuthoring, GameKernel, ObservationModel,
    OracleProjection,
};
pub use policy::{FirstLegalPolicy, FnPolicy, Policy, RandomPolicy, ScriptedPolicy};
pub use rng::{DeterministicRng, SplitMix64};
pub use session::{
    DynamicHistory, FixedHistory, HistorySnapshot, HistoryStore, InteractiveSession, Session,
    SessionKernel,
};
pub use types::{
    DynamicReplayTrace, JointActionProfile, JointActionProfileError, KernelOutcome, PlayerAction,
    PlayerId, PlayerReward, ReplayTrace, Reward, Seed, SessionStepRecord, Termination, Tick,
    stable_hash,
};
