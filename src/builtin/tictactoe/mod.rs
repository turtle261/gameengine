//! Builtin deterministic tic-tac-toe environment and compact encoding.

use crate::buffer::FixedVec;
use crate::compact::CompactSpec;
use crate::core::single_player::{self, SinglePlayerRewardBuf};
use crate::rng::DeterministicRng;
use crate::types::{PlayerId, Seed, StepOutcome, Termination};
use crate::verification::reward_and_terminal_postcondition;

const WIN_LINES: [(usize, usize, usize); 8] = [
    (0, 1, 2),
    (3, 4, 5),
    (6, 7, 8),
    (0, 3, 6),
    (1, 4, 7),
    (2, 5, 8),
    (0, 4, 8),
    (2, 4, 6),
];

/// Cell state on the 3x3 board.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TicTacToeCell {
    /// Empty cell.
    #[default]
    Empty,
    /// Player-controlled mark.
    Player,
    /// Opponent mark.
    Opponent,
}

/// Compact action selecting one board cell.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TicTacToeAction(pub u8);

/// Complete deterministic game state.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TicTacToeState {
    /// Board occupancy.
    pub board: [TicTacToeCell; 9],
    /// Terminal-state flag.
    pub terminal: bool,
    /// Winner id when terminal with a winner.
    pub winner: Option<PlayerId>,
}

/// Canonical tic-tac-toe observation type.
pub type TicTacToeObservation = TicTacToeState;
/// World/debug view type.
pub type TicTacToeWorldView = TicTacToeState;

/// Builtin deterministic tic-tac-toe environment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TicTacToe;

impl TicTacToe {
    fn find_winner(board: &[TicTacToeCell; 9]) -> Option<PlayerId> {
        for (a, b, c) in WIN_LINES {
            let cells = (board[a], board[b], board[c]);
            if cells
                == (
                    TicTacToeCell::Player,
                    TicTacToeCell::Player,
                    TicTacToeCell::Player,
                )
            {
                return Some(0);
            }
            if cells
                == (
                    TicTacToeCell::Opponent,
                    TicTacToeCell::Opponent,
                    TicTacToeCell::Opponent,
                )
            {
                return Some(1);
            }
        }
        None
    }

    fn is_full(board: &[TicTacToeCell; 9]) -> bool {
        let mut index = 0usize;
        while index < board.len() {
            if board[index] == TicTacToeCell::Empty {
                return false;
            }
            index += 1;
        }
        true
    }

    fn decode_action_index(action: Option<TicTacToeAction>) -> Option<usize> {
        action.map(|action: TicTacToeAction| action.0 as usize)
    }

    fn action_is_legal(state: &TicTacToeState, index: usize) -> bool {
        index < state.board.len() && state.board[index] == TicTacToeCell::Empty
    }

    fn apply_mark(
        state: &mut TicTacToeState,
        index: usize,
        mark: TicTacToeCell,
    ) -> Option<Option<PlayerId>> {
        state.board[index] = mark;
        let winner = Self::find_winner(&state.board);
        if winner.is_some() || Self::is_full(&state.board) {
            state.terminal = true;
            state.winner = winner;
            Some(winner)
        } else {
            None
        }
    }

    fn sample_opponent_action(state: &TicTacToeState, rng: &mut DeterministicRng) -> usize {
        let mut empty_positions = [0usize; 9];
        let mut empty_len = 0usize;
        let mut index = 0usize;
        while index < state.board.len() {
            if state.board[index] == TicTacToeCell::Empty {
                empty_positions[empty_len] = index;
                empty_len += 1;
            }
            index += 1;
        }
        empty_positions[rng.gen_range(empty_len)]
    }

    fn reward_from_terminal_winner(winner: Option<PlayerId>) -> i64 {
        match winner {
            Some(0) => 2,
            Some(_) => -2,
            None => 1,
        }
    }

    fn resolve_turn(
        state: &mut TicTacToeState,
        action_index: usize,
        rng: &mut DeterministicRng,
    ) -> i64 {
        if let Some(winner) = Self::apply_mark(state, action_index, TicTacToeCell::Player) {
            return Self::reward_from_terminal_winner(winner);
        }

        let opponent_index = Self::sample_opponent_action(state, rng);
        if let Some(winner) = Self::apply_mark(state, opponent_index, TicTacToeCell::Opponent) {
            return Self::reward_from_terminal_winner(winner);
        }

        0
    }

    fn termination_from_state(state: &TicTacToeState) -> Termination {
        if state.terminal {
            Termination::Terminal {
                winner: state.winner,
            }
        } else {
            Termination::Ongoing
        }
    }

    /// Packs board cells into a two-bit-per-cell `u64` representation.
    pub fn packed_board(board: &[TicTacToeCell; 9]) -> u64 {
        let mut packed = 0u64;
        let mut index = 0usize;
        while index < board.len() {
            let value = match board[index] {
                TicTacToeCell::Empty => 0,
                TicTacToeCell::Player => 1,
                TicTacToeCell::Opponent => 2,
            };
            packed |= value << (index * 2);
            index += 1;
        }
        packed
    }
}

impl single_player::SinglePlayerGame for TicTacToe {
    type Params = ();
    type State = TicTacToeState;
    type Action = TicTacToeAction;
    type Obs = TicTacToeObservation;
    type WorldView = TicTacToeWorldView;
    type ActionBuf = FixedVec<TicTacToeAction, 9>;
    type WordBuf = FixedVec<u64, 1>;

    fn name(&self) -> &'static str {
        "tictactoe"
    }

    fn init_with_params(&self, _seed: Seed, _params: &Self::Params) -> Self::State {
        TicTacToeState::default()
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        state.terminal
    }

    fn legal_actions(&self, state: &Self::State, out: &mut Self::ActionBuf) {
        out.clear();
        if state.terminal {
            return;
        }
        let mut index = 0usize;
        while index < state.board.len() {
            if state.board[index] == TicTacToeCell::Empty {
                out.push(TicTacToeAction(index as u8)).unwrap();
            }
            index += 1;
        }
    }

    fn observe_player(&self, state: &Self::State) -> Self::Obs {
        *state
    }

    fn world_view(&self, state: &Self::State) -> Self::WorldView {
        *state
    }

    fn step_in_place(
        &self,
        state: &mut Self::State,
        action: Option<Self::Action>,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<SinglePlayerRewardBuf>,
    ) {
        let reward = if state.terminal {
            0
        } else {
            match Self::decode_action_index(action) {
                Some(index) if Self::action_is_legal(state, index) => {
                    Self::resolve_turn(state, index, rng)
                }
                _ => -3,
            }
        };

        single_player::push_reward(&mut out.rewards, reward);
        out.termination = Self::termination_from_state(state);
    }

    fn compact_spec(&self) -> CompactSpec {
        CompactSpec {
            action_count: 9,
            observation_bits: 18,
            observation_stream_len: 1,
            reward_bits: 3,
            min_reward: -3,
            max_reward: 2,
            reward_offset: 3,
        }
    }

    fn encode_action(&self, action: &Self::Action) -> u64 {
        u64::from(action.0)
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        (encoded < 9).then_some(TicTacToeAction(encoded as u8))
    }

    fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        out.clear();
        out.push(Self::packed_board(&observation.board)).unwrap();
    }

    fn state_invariant(&self, state: &Self::State) -> bool {
        let winner = Self::find_winner(&state.board);
        let full = Self::is_full(&state.board);
        state.terminal == (winner.is_some() || full)
            && (state.winner == winner || (winner.is_none() && state.winner.is_none()))
    }

    fn action_invariant(&self, action: &Self::Action) -> bool {
        action.0 < 9
    }

    fn transition_postcondition(
        &self,
        pre: &Self::State,
        _action: Option<Self::Action>,
        post: &Self::State,
        outcome: &StepOutcome<SinglePlayerRewardBuf>,
    ) -> bool {
        if pre.terminal {
            return post == pre && outcome.reward_for(0) == 0 && outcome.is_terminal();
        }
        reward_and_terminal_postcondition(
            outcome.reward_for(0),
            -3,
            2,
            post.terminal,
            outcome.is_terminal(),
        )
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs;
