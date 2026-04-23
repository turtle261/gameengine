//! Builtin biased rock-paper-scissor environment.

use crate::buffer::FixedVec;
use crate::compact::{CompactSpec, decode_enum_action, encode_enum_action};
use crate::core::single_player::{self, SinglePlayerRewardBuf};
use crate::game::OracleProjection;
use crate::rng::DeterministicRng;
use crate::types::{KernelOutcome, Reward, Seed, Termination};
use crate::verification::reward_and_terminal_postcondition;

const BIASED_RPS_ACTION_ORDER: [BiasedRockPaperScissorAction; 3] = [
    BiasedRockPaperScissorAction::Rock,
    BiasedRockPaperScissorAction::Paper,
    BiasedRockPaperScissorAction::Scissors,
];

/// Agent action in rock-paper-scissor.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BiasedRockPaperScissorAction {
    /// Rock.
    #[default]
    Rock,
    /// Paper.
    Paper,
    /// Scissors.
    Scissors,
}

/// Full environment state.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BiasedRockPaperScissorState {
    /// Last opponent action, also emitted as observation.
    pub opponent_action: BiasedRockPaperScissorAction,
    /// Last reward.
    pub reward: Reward,
}

/// Player observation exposing opponent action.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BiasedRockPaperScissorObservation {
    /// Last opponent action.
    pub opponent_action: BiasedRockPaperScissorAction,
}

/// Full world/debug view type.
pub type BiasedRockPaperScissorWorldView = BiasedRockPaperScissorState;

/// Builtin biased rock-paper-scissor environment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BiasedRockPaperScissor;

impl BiasedRockPaperScissor {
    fn action_code(action: BiasedRockPaperScissorAction) -> u64 {
        match action {
            BiasedRockPaperScissorAction::Rock => 0,
            BiasedRockPaperScissorAction::Paper => 1,
            BiasedRockPaperScissorAction::Scissors => 2,
        }
    }

    fn sample_uniform_action(rng: &mut DeterministicRng) -> BiasedRockPaperScissorAction {
        match rng.gen_range(3) {
            0 => BiasedRockPaperScissorAction::Rock,
            1 => BiasedRockPaperScissorAction::Paper,
            _ => BiasedRockPaperScissorAction::Scissors,
        }
    }

    fn beats(left: BiasedRockPaperScissorAction, right: BiasedRockPaperScissorAction) -> bool {
        matches!(
            (left, right),
            (
                BiasedRockPaperScissorAction::Rock,
                BiasedRockPaperScissorAction::Scissors
            ) | (
                BiasedRockPaperScissorAction::Paper,
                BiasedRockPaperScissorAction::Rock
            ) | (
                BiasedRockPaperScissorAction::Scissors,
                BiasedRockPaperScissorAction::Paper
            )
        )
    }

    fn opponent_action_for_step(
        state: &BiasedRockPaperScissorState,
        rng: &mut DeterministicRng,
    ) -> BiasedRockPaperScissorAction {
        if state.opponent_action == BiasedRockPaperScissorAction::Rock && state.reward == -1 {
            BiasedRockPaperScissorAction::Rock
        } else {
            Self::sample_uniform_action(rng)
        }
    }

    fn reward_for_round(
        agent: Option<BiasedRockPaperScissorAction>,
        opponent: BiasedRockPaperScissorAction,
    ) -> Reward {
        let Some(agent_action) = agent else {
            return -1;
        };
        if agent_action == opponent {
            0
        } else if Self::beats(agent_action, opponent) {
            1
        } else {
            -1
        }
    }

    fn model_step(
        state: &mut BiasedRockPaperScissorState,
        action: Option<BiasedRockPaperScissorAction>,
        rng: &mut DeterministicRng,
    ) -> Reward {
        let opponent_action = Self::opponent_action_for_step(state, rng);
        state.reward = Self::reward_for_round(action, opponent_action);
        state.opponent_action = opponent_action;
        state.reward
    }
}

impl single_player::SinglePlayerGame for BiasedRockPaperScissor {
    type Params = ();
    type State = BiasedRockPaperScissorState;
    type Action = BiasedRockPaperScissorAction;
    type Obs = BiasedRockPaperScissorObservation;
    type ActionBuf = FixedVec<BiasedRockPaperScissorAction, 3>;
    type WordBuf = FixedVec<u64, 1>;

    fn name(&self) -> &'static str {
        "biased_rock_paper_scissor"
    }

    fn init_with_params(&self, _seed: Seed, _params: &Self::Params) -> Self::State {
        BiasedRockPaperScissorState {
            opponent_action: BiasedRockPaperScissorAction::Paper,
            reward: 0,
        }
    }

    fn is_terminal(&self, _state: &Self::State) -> bool {
        false
    }

    fn legal_actions(&self, _state: &Self::State, out: &mut Self::ActionBuf) {
        out.clear();
        out.push(BiasedRockPaperScissorAction::Rock).unwrap();
        out.push(BiasedRockPaperScissorAction::Paper).unwrap();
        out.push(BiasedRockPaperScissorAction::Scissors).unwrap();
    }

    fn observe_player(&self, state: &Self::State) -> Self::Obs {
        BiasedRockPaperScissorObservation {
            opponent_action: state.opponent_action,
        }
    }

    fn step_in_place(
        &self,
        state: &mut Self::State,
        action: Option<Self::Action>,
        rng: &mut DeterministicRng,
        out: &mut KernelOutcome<SinglePlayerRewardBuf>,
    ) {
        let reward = Self::model_step(state, action, rng);
        single_player::push_reward(&mut out.rewards, reward);
        out.termination = Termination::Ongoing;
    }

    fn compact_spec_for(&self, _params: &Self::Params) -> CompactSpec {
        CompactSpec {
            action_count: 3,
            observation_bits: 2,
            observation_stream_len: 1,
            reward_bits: 2,
            min_reward: -1,
            max_reward: 1,
            reward_offset: 1,
        }
    }

    fn encode_action(&self, action: &Self::Action) -> u64 {
        encode_enum_action(*action, &BIASED_RPS_ACTION_ORDER)
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        decode_enum_action(encoded, &BIASED_RPS_ACTION_ORDER)
    }

    fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        out.clear();
        out.push(Self::action_code(observation.opponent_action))
            .unwrap();
    }

    fn state_invariant(&self, state: &Self::State) -> bool {
        (-1..=1).contains(&state.reward)
    }

    fn oracle_world_view_invariant(&self, state: &Self::State) -> bool {
        let world: <Self as OracleProjection>::WorldView =
            <Self as OracleProjection>::world_view(self, state);
        <Self as OracleProjection>::world_view_invariant(self, state, &world)
    }

    fn transition_postcondition(
        &self,
        _pre: &Self::State,
        _action: Option<Self::Action>,
        _post: &Self::State,
        outcome: &KernelOutcome<SinglePlayerRewardBuf>,
    ) -> bool {
        reward_and_terminal_postcondition(outcome.reward_for(0), -1, 1, false, false)
            && !outcome.is_terminal()
    }
}

impl OracleProjection for BiasedRockPaperScissor {
    type WorldView = BiasedRockPaperScissorWorldView;

    fn world_view(&self, state: &Self::State) -> Self::WorldView {
        *state
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs;
