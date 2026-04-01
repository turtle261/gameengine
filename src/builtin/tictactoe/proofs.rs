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
