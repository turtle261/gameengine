//! Core engine helpers shared across environments and adapters.

pub mod cards;
pub mod env;
pub mod observe;
pub mod serialization;
pub mod single_player;
pub mod stepper;

pub use crate::buffer::{BitWords, Buffer, CapacityError, FixedVec};
pub use crate::compact::CompactSpec;
pub use crate::core::env::{
    ActionToken, ActionTokenError, AixiEnvironment, DefaultEnvironment, EnvError, Environment,
    Percept,
};
pub use crate::core::serialization::{
    BitStream, BitStreamBits, serialize_action, serialize_history, serialize_percept,
};
pub use crate::core::single_player::SinglePlayerGame;
pub use crate::game::{
    CompactCodec, ContractSurface, Game, GameAuthoring, GameKernel, ObservationModel,
    OracleProjection,
};
pub use crate::rng::{DeterministicRng, SplitMix64};
pub use crate::session::{
    DynamicHistory, FixedHistory, HistorySnapshot, HistoryStore, InteractiveSession, Session,
    SessionKernel,
};
pub use crate::types::{
    DynamicReplayTrace, JointActionProfile, JointActionProfileError, PlayerAction, PlayerId,
    PlayerReward, SessionStepRecord, ReplayTrace, Reward, Seed, KernelOutcome, Termination, Tick,
    stable_hash,
};
