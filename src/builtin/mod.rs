//! Builtin game implementations shipped with the engine.

pub mod biased_coinflip;
pub mod biased_rock_paper_scissor;
pub mod blackjack;
pub mod extended_tiger;
pub mod kuhn_poker;
#[cfg(feature = "physics")]
pub mod platformer;
pub mod tictactoe;

pub use biased_coinflip::*;
pub use biased_rock_paper_scissor::*;
pub use blackjack::*;
pub use extended_tiger::*;
pub use kuhn_poker::*;
#[cfg(feature = "physics")]
pub use platformer::*;
pub use tictactoe::*;
