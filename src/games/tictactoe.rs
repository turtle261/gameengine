use crate::buffer::FixedVec;
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

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TicTacToeCell {
    #[default]
    Empty,
    Player,
    Opponent,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TicTacToeAction(pub u8);

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TicTacToeState {
    pub board: [TicTacToeCell; 9],
    pub terminal: bool,
    pub winner: Option<PlayerId>,
}

pub type TicTacToeObservation = TicTacToeState;
pub type TicTacToeWorldView = TicTacToeState;

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
        out.clear();
        if !state.terminal {
            out.push(0).unwrap();
        }
    }

    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf) {
        out.clear();
        if player != 0 || state.terminal {
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
        let mut action = None;
        let actions = joint_actions.as_slice();
        let mut action_index = 0usize;
        while action_index < actions.len() {
            let candidate = &actions[action_index];
            if candidate.player == 0 {
                action = Some(candidate.action.0 as usize);
                break;
            }
            action_index += 1;
        }

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

        out.rewards
            .push(PlayerReward { player: 0, reward })
            .unwrap();
        if !state.terminal {
            out.termination = Termination::Ongoing;
        }
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
        let reward = outcome.reward_for(0);
        matches!(reward, -3..=2) && (!post.terminal || outcome.is_terminal())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;
    use crate::verification::{
        assert_compact_roundtrip, assert_observation_contracts, assert_transition_contracts,
    };

    #[test]
    fn illegal_move_preserves_state_and_penalizes() {
        let mut session = Session::new(TicTacToe, 7);
        session.step(&[PlayerAction {
            player: 0,
            action: TicTacToeAction(0),
        }]);
        let before = *session.state();
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
            let mut legal = FixedVec::<TicTacToeAction, 9>::default();
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
            assert_eq!(
                legal.as_slice(),
                expected.as_slice(),
                "encoded board state {encoded}"
            );
            assert_observation_contracts(&game, &state);
        }
    }

    #[test]
    fn verification_helpers_hold_for_opening_move() {
        let game = TicTacToe;
        let state = game.init(7);
        let mut actions = FixedVec::<PlayerAction<TicTacToeAction>, 1>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: TicTacToeAction(0),
            })
            .unwrap();
        assert_transition_contracts(&game, &state, &actions, 7);
        assert_compact_roundtrip(&game, &TicTacToeAction(0));
    }
}

#[cfg(kani)]
mod proofs {
    use super::{TicTacToe, TicTacToeAction, TicTacToeCell, TicTacToeState};
    use crate::buffer::FixedVec;
    use crate::game::Game;
    use crate::session::{FixedHistory, SessionKernel};
    use crate::types::PlayerAction;

    #[kani::proof]
    #[kani::unwind(16)]
    fn legal_actions_are_exactly_empty_cells() {
        let encoded: u32 = kani::any();
        kani::assume(encoded < 3u32.pow(9));
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
        let mut legal = FixedVec::<TicTacToeAction, 9>::default();
        TicTacToe.legal_actions(&state, 0, &mut legal);
        let mut legal_count = 0usize;
        let mut legal_index = 0usize;
        while legal_index < legal.len() {
            let action = legal.as_slice()[legal_index];
            assert_eq!(state.board[action.0 as usize], TicTacToeCell::Empty);
            legal_count += 1;
            legal_index += 1;
        }

        let mut empty_count = 0usize;
        let mut board_index = 0usize;
        while board_index < state.board.len() {
            if state.board[board_index] == TicTacToeCell::Empty {
                if !terminal {
                    assert!(
                        legal
                            .as_slice()
                            .contains(&TicTacToeAction(board_index as u8))
                    );
                }
                empty_count += 1;
            }
            board_index += 1;
        }
        assert_eq!(legal_count, if terminal { 0 } else { empty_count });
    }

    #[kani::proof]
    #[kani::unwind(16)]
    fn invalid_move_never_mutates_board() {
        type ProofSession = SessionKernel<TicTacToe, FixedHistory<TicTacToe, 8, 2, 1>>;

        let mut session = ProofSession::new(TicTacToe, 1);
        session.step(&[PlayerAction {
            player: 0,
            action: TicTacToeAction(0),
        }]);
        let mut actions = FixedVec::<PlayerAction<TicTacToeAction>, 1>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: TicTacToeAction(0),
            })
            .unwrap();
        let before = *session.state();
        session.step_with_joint_actions(&actions);
        assert_eq!(*session.state(), before);
    }
}
