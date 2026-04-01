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
