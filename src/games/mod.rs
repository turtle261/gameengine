pub mod blackjack;
pub mod platformer;
pub mod tictactoe;

pub use blackjack::{
    Blackjack, BlackjackAction, BlackjackObservation, BlackjackSpectatorObservation,
};
pub use platformer::{Platformer, PlatformerAction, PlatformerObservation};
pub use tictactoe::{TicTacToe, TicTacToeAction, TicTacToeObservation};
