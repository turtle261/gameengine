use crate::compact::{CompactGame, CompactSpec};
use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};

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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TicTacToeCell {
    Empty,
    Player,
    Opponent,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TicTacToeAction(pub u8);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TicTacToeState {
    pub board: [TicTacToeCell; 9],
    pub terminal: bool,
    pub winner: Option<PlayerId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TicTacToeObservation {
    pub board: [TicTacToeCell; 9],
    pub terminal: bool,
    pub winner: Option<PlayerId>,
}

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
        board.iter().all(|cell| *cell != TicTacToeCell::Empty)
    }

    pub fn packed_board(board: &[TicTacToeCell; 9]) -> u64 {
        let mut packed = 0u64;
        for (index, cell) in board.iter().enumerate() {
            let value = match cell {
                TicTacToeCell::Empty => 0,
                TicTacToeCell::Player => 1,
                TicTacToeCell::Opponent => 2,
            };
            packed |= value << (index * 2);
        }
        packed
    }
}

impl Game for TicTacToe {
    type State = TicTacToeState;
    type Action = TicTacToeAction;
    type PlayerObservation = TicTacToeObservation;
    type SpectatorObservation = TicTacToeObservation;

    fn name(&self) -> &'static str {
        "tictactoe"
    }

    fn player_count(&self) -> usize {
        1
    }

    fn init(&self, _seed: Seed) -> Self::State {
        TicTacToeState {
            board: [TicTacToeCell::Empty; 9],
            terminal: false,
            winner: None,
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        state.terminal
    }

    fn players_to_act(&self, state: &Self::State, out: &mut Vec<PlayerId>) {
        out.clear();
        if !state.terminal {
            out.push(0);
        }
    }

    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Vec<Self::Action>) {
        out.clear();
        if player != 0 || state.terminal {
            return;
        }
        for (index, cell) in state.board.iter().enumerate() {
            if *cell == TicTacToeCell::Empty {
                out.push(TicTacToeAction(index as u8));
            }
        }
    }

    fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::PlayerObservation {
        TicTacToeObservation {
            board: state.board,
            terminal: state.terminal,
            winner: state.winner,
        }
    }

    fn observe_spectator(&self, state: &Self::State) -> Self::SpectatorObservation {
        self.observe_player(state, 0)
    }

    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &[PlayerAction<Self::Action>],
        rng: &mut DeterministicRng,
        out: &mut StepOutcome,
    ) {
        let action = joint_actions
            .iter()
            .find(|candidate| candidate.player == 0)
            .map(|candidate| candidate.action.0 as usize);

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
                    for (cell_index, cell) in state.board.iter().enumerate() {
                        if *cell == TicTacToeCell::Empty {
                            empty_positions[empty_len] = cell_index;
                            empty_len += 1;
                        }
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

        out.rewards.push(PlayerReward { player: 0, reward });
        if !state.terminal {
            out.termination = Termination::Ongoing;
        }
    }
}

impl CompactGame for TicTacToe {
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
        if encoded < 9 {
            Some(TicTacToeAction(encoded as u8))
        } else {
            None
        }
    }

    fn encode_player_observation(&self, observation: &Self::PlayerObservation, out: &mut Vec<u64>) {
        out.clear();
        out.push(Self::packed_board(&observation.board));
    }

    fn encode_spectator_observation(
        &self,
        observation: &Self::SpectatorObservation,
        out: &mut Vec<u64>,
    ) {
        self.encode_player_observation(observation, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    #[test]
    fn illegal_move_preserves_state_and_penalizes() {
        let mut session = Session::new(TicTacToe, 7);
        session.step(&[PlayerAction {
            player: 0,
            action: TicTacToeAction(0),
        }]);
        let before = session.state().clone();
        let outcome = session.step(&[PlayerAction {
            player: 0,
            action: TicTacToeAction(0),
        }]);
        assert_eq!(outcome.reward_for(0), -3);
        assert_eq!(session.state(), &before);
    }

    #[test]
    fn legal_actions_match_empty_cells_exhaustively() {
        let game = TicTacToe;
        for encoded in 0..3u32.pow(9) {
            let mut board = [TicTacToeCell::Empty; 9];
            let mut value = encoded;
            for cell in &mut board {
                *cell = match value % 3 {
                    0 => TicTacToeCell::Empty,
                    1 => TicTacToeCell::Player,
                    _ => TicTacToeCell::Opponent,
                };
                value /= 3;
            }
            let winner = TicTacToe::find_winner(&board);
            let terminal = winner.is_some() || TicTacToe::is_full(&board);
            let state = TicTacToeState {
                board,
                terminal,
                winner,
            };
            let mut legal = Vec::new();
            game.legal_actions(&state, 0, &mut legal);
            let expected: Vec<_> = if terminal {
                Vec::new()
            } else {
                state
                    .board
                    .iter()
                    .enumerate()
                    .filter_map(|(index, cell)| {
                        (*cell == TicTacToeCell::Empty).then_some(TicTacToeAction(index as u8))
                    })
                    .collect()
            };
            assert_eq!(legal, expected, "encoded board state {encoded}");
        }
    }
}
