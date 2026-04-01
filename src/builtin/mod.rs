//! Builtin game implementations shipped with the engine.

pub mod blackjack;
#[cfg(feature = "physics")]
pub mod platformer;
pub mod tictactoe;

pub use blackjack::*;
#[cfg(feature = "physics")]
pub use platformer::*;
pub use tictactoe::*;
