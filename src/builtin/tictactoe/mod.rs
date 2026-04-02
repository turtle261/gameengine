//! Builtin deterministic tic-tac-toe environment and compact encoding.

use crate::buffer::FixedVec;
use crate::compact::CompactSpec;
use crate::core::single_player::{self, SinglePlayerRewardBuf};
use crate::proof::{
    FairnessWitness, FiniteSupportOutcome, ModelGame, ProbabilisticWitness, RefinementWitness,
    TerminationWitness, VerifiedGame,
};
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

    fn model_step(
        state: &mut TicTacToeState,
        action: Option<TicTacToeAction>,
        rng: &mut DeterministicRng,
    ) -> i64 {
        if state.terminal {
            return 0;
        }
        match Self::decode_action_index(action) {
            Some(index) if Self::action_is_legal(state, index) => {
                Self::resolve_turn(state, index, rng)
            }
            _ => -3,
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

    fn empty_cell_count(state: &TicTacToeState) -> u64 {
        let mut empty = 0u64;
        let mut index = 0usize;
        while index < state.board.len() {
            if state.board[index] == TicTacToeCell::Empty {
                empty += 1;
            }
            index += 1;
        }
        empty
    }

    fn push_support_outcome(
        out: &mut FixedVec<FiniteSupportOutcome<TicTacToeState, SinglePlayerRewardBuf>, 9>,
        state: TicTacToeState,
        reward: i64,
        weight: u64,
    ) {
        let mut rewards = SinglePlayerRewardBuf::default();
        single_player::push_reward(&mut rewards, reward);
        out.push(FiniteSupportOutcome {
            termination: Self::termination_from_state(&state),
            state,
            rewards,
            weight,
        })
        .unwrap();
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
        let reward = Self::model_step(state, action, rng);

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

impl ModelGame for TicTacToe {
    type ModelState = TicTacToeState;
    type ModelObs = TicTacToeObservation;
    type ModelWorldView = TicTacToeWorldView;

    fn model_init_with_params(&self, _seed: Seed, _params: &Self::Params) -> Self::ModelState {
        TicTacToeState::default()
    }

    fn model_is_terminal(&self, state: &Self::ModelState) -> bool {
        state.terminal
    }

    fn model_players_to_act(&self, state: &Self::ModelState, out: &mut Self::PlayerBuf) {
        out.clear();
        if !state.terminal {
            out.push(0).unwrap();
        }
    }

    fn model_legal_actions(
        &self,
        state: &Self::ModelState,
        _player: PlayerId,
        out: &mut Self::ActionBuf,
    ) {
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

    fn model_observe_player(&self, state: &Self::ModelState, _player: PlayerId) -> Self::ModelObs {
        *state
    }

    fn model_observe_spectator(&self, state: &Self::ModelState) -> Self::ModelObs {
        *state
    }

    fn model_world_view(&self, state: &Self::ModelState) -> Self::ModelWorldView {
        *state
    }

    fn model_step_in_place(
        &self,
        state: &mut Self::ModelState,
        actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<Self::RewardBuf>,
    ) {
        let action = actions
            .as_slice()
            .iter()
            .find(|candidate| candidate.player == 0)
            .map(|candidate| candidate.action);
        let reward = Self::model_step(state, action, rng);
        single_player::push_reward(&mut out.rewards, reward);
        out.termination = Self::termination_from_state(state);
    }
}

impl RefinementWitness for TicTacToe {
    fn runtime_state_to_model(&self, state: &Self::State) -> Self::ModelState {
        *state
    }

    fn runtime_observation_to_model(&self, observation: &Self::Obs) -> Self::ModelObs {
        *observation
    }

    fn runtime_world_view_to_model(&self, world: &Self::WorldView) -> Self::ModelWorldView {
        *world
    }
}

impl VerifiedGame for TicTacToe {}

impl TerminationWitness for TicTacToe {
    fn model_rank(&self, state: &Self::ModelState) -> u64 {
        if state.terminal {
            0
        } else {
            Self::empty_cell_count(state)
        }
    }
}

impl FairnessWitness for TicTacToe {}

impl ProbabilisticWitness for TicTacToe {
    type SupportBuf = FixedVec<FiniteSupportOutcome<TicTacToeState, SinglePlayerRewardBuf>, 9>;

    fn model_step_support(
        &self,
        state: &Self::ModelState,
        actions: &Self::JointActionBuf,
        out: &mut Self::SupportBuf,
    ) {
        out.clear();
        let action = actions
            .as_slice()
            .iter()
            .find(|candidate| candidate.player == 0)
            .map(|candidate| candidate.action);

        if state.terminal {
            Self::push_support_outcome(out, *state, 0, 1);
            return;
        }

        let Some(action_index) = Self::decode_action_index(action) else {
            Self::push_support_outcome(out, *state, -3, 1);
            return;
        };
        if !Self::action_is_legal(state, action_index) {
            Self::push_support_outcome(out, *state, -3, 1);
            return;
        }

        let mut player_state = *state;
        if let Some(winner) =
            Self::apply_mark(&mut player_state, action_index, TicTacToeCell::Player)
        {
            Self::push_support_outcome(
                out,
                player_state,
                Self::reward_from_terminal_winner(winner),
                1,
            );
            return;
        }

        let mut index = 0usize;
        while index < player_state.board.len() {
            if player_state.board[index] == TicTacToeCell::Empty {
                let mut branch = player_state;
                let reward = if let Some(winner) =
                    Self::apply_mark(&mut branch, index, TicTacToeCell::Opponent)
                {
                    Self::reward_from_terminal_winner(winner)
                } else {
                    0
                };
                Self::push_support_outcome(out, branch, reward, 1);
            }
            index += 1;
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs;
