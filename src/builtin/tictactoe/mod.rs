//! Builtin deterministic tic-tac-toe environment and compact encoding.

use crate::buffer::FixedVec;
use crate::compact::CompactSpec;
use crate::core::single_player;
use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};
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

impl Game for TicTacToe {
    type State = TicTacToeState;
    type Action = TicTacToeAction;
    type PlayerObservation = TicTacToeObservation;
    type SpectatorObservation = TicTacToeObservation;
    type WorldView = TicTacToeWorldView;
    type PlayerBuf = FixedVec<PlayerId, 1>;
    type ActionBuf = FixedVec<TicTacToeAction, 9>;
    type JointActionBuf = FixedVec<PlayerAction<TicTacToeAction>, 1>;
    type RewardBuf = FixedVec<PlayerReward, 1>;
    type WordBuf = FixedVec<u64, 1>;

    fn name(&self) -> &'static str {
        "tictactoe"
    }

    fn player_count(&self) -> usize {
        1
    }

    fn init(&self, _seed: Seed) -> Self::State {
        TicTacToeState::default()
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        state.terminal
    }

    fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf) {
        single_player::write_players_to_act(out, state.terminal);
    }

    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf) {
        out.clear();
        if !single_player::can_act(player, state.terminal) {
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

    fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::PlayerObservation {
        *state
    }

    fn observe_spectator(&self, state: &Self::State) -> Self::SpectatorObservation {
        *state
    }

    fn world_view(&self, state: &Self::State) -> Self::WorldView {
        *state
    }

    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<Self::RewardBuf>,
    ) {
        let action = single_player::first_action(joint_actions.as_slice())
            .map(|candidate: TicTacToeAction| candidate.0 as usize);

        let reward = if state.terminal {
            out.termination = Termination::Terminal {
                winner: state.winner,
            };
            0
        } else if let Some(index) = action {
            if index >= 9 || state.board[index] != TicTacToeCell::Empty {
                -3
            } else {
                state.board[index] = TicTacToeCell::Player;
                if let Some(winner) = Self::find_winner(&state.board) {
                    state.terminal = true;
                    state.winner = Some(winner);
                    out.termination = Termination::Terminal {
                        winner: state.winner,
                    };
                    2
                } else if Self::is_full(&state.board) {
                    state.terminal = true;
                    state.winner = None;
                    out.termination = Termination::Terminal { winner: None };
                    1
                } else {
                    let mut empty_positions = [0usize; 9];
                    let mut empty_len = 0usize;
                    let mut cell_index = 0usize;
                    while cell_index < state.board.len() {
                        if state.board[cell_index] == TicTacToeCell::Empty {
                            empty_positions[empty_len] = cell_index;
                            empty_len += 1;
                        }
                        cell_index += 1;
                    }
                    let opponent_index = empty_positions[rng.gen_range(empty_len)];
                    state.board[opponent_index] = TicTacToeCell::Opponent;
                    if let Some(winner) = Self::find_winner(&state.board) {
                        state.terminal = true;
                        state.winner = Some(winner);
                        out.termination = Termination::Terminal {
                            winner: state.winner,
                        };
                        -2
                    } else if Self::is_full(&state.board) {
                        state.terminal = true;
                        state.winner = None;
                        out.termination = Termination::Terminal { winner: None };
                        1
                    } else {
                        0
                    }
                }
            }
        } else {
            -3
        };

        single_player::push_reward(&mut out.rewards, reward);
        if !state.terminal {
            out.termination = Termination::Ongoing;
        }
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

    fn encode_player_observation(
        &self,
        observation: &Self::PlayerObservation,
        out: &mut Self::WordBuf,
    ) {
        out.clear();
        out.push(Self::packed_board(&observation.board)).unwrap();
    }

    fn encode_spectator_observation(
        &self,
        observation: &Self::SpectatorObservation,
        out: &mut Self::WordBuf,
    ) {
        self.encode_player_observation(observation, out);
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
        _actions: &Self::JointActionBuf,
        post: &Self::State,
        outcome: &StepOutcome<Self::RewardBuf>,
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
