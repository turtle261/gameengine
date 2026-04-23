//! Builtin single-step Kuhn poker environment.

use crate::buffer::FixedVec;
use crate::compact::{CompactSpec, decode_enum_action, encode_enum_action};
use crate::core::single_player::{self, SinglePlayerRewardBuf};
use crate::game::OracleProjection;
use crate::rng::DeterministicRng;
use crate::types::{KernelOutcome, Reward, Seed, Termination};
use crate::verification::reward_and_terminal_postcondition;

const KUHN_POKER_ACTION_ORDER: [KuhnPokerAction; 2] = [KuhnPokerAction::Bet, KuhnPokerAction::Pass];
const BET_PROB_KING_NUM: u64 = 7;
const BET_PROB_KING_DEN: u64 = 10;
const BET_PROB_JACK_NUM: u64 = 7;
const BET_PROB_JACK_DEN: u64 = 30;
const BET_PROB_QUEEN_NUM: u64 = 17;
const BET_PROB_QUEEN_DEN: u64 = 30;

/// Agent action in Kuhn poker.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum KuhnPokerAction {
    /// Bet/call.
    #[default]
    Bet,
    /// Pass/fold.
    Pass,
}

/// Card rank in the reduced three-card deck.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum KuhnPokerCard {
    /// Jack (lowest rank).
    #[default]
    Jack,
    /// Queen.
    Queen,
    /// King (highest rank).
    King,
}

impl KuhnPokerCard {
    fn code(self) -> u8 {
        match self {
            Self::Jack => 0,
            Self::Queen => 1,
            Self::King => 2,
        }
    }

    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Jack,
            1 => Self::Queen,
            _ => Self::King,
        }
    }
}

/// Full environment state.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct KuhnPokerState {
    /// Private agent card.
    pub agent_card: KuhnPokerCard,
    /// Hidden opponent card.
    pub opponent_card: KuhnPokerCard,
    /// Opponent opening action.
    pub opponent_action: KuhnPokerAction,
    /// Encoded player observation for the current round.
    pub observation: u8,
    /// Reward from the previous action.
    pub reward: Reward,
}

/// Player observation stream element.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct KuhnPokerObservation {
    /// Encoded observation value in `[0, 7]`.
    pub value: u8,
}

/// Full world/debug view type.
pub type KuhnPokerWorldView = KuhnPokerState;

/// Builtin Kuhn poker environment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct KuhnPoker;

impl KuhnPoker {
    fn random_card(rng: &mut DeterministicRng) -> KuhnPokerCard {
        KuhnPokerCard::from_index(rng.gen_range(3))
    }

    fn opening_action(opponent_card: KuhnPokerCard, rng: &mut DeterministicRng) -> KuhnPokerAction {
        match opponent_card {
            KuhnPokerCard::Jack => {
                if rng.gen_bool_ratio(BET_PROB_JACK_NUM, BET_PROB_JACK_DEN) {
                    KuhnPokerAction::Bet
                } else {
                    KuhnPokerAction::Pass
                }
            }
            KuhnPokerCard::Queen => KuhnPokerAction::Pass,
            KuhnPokerCard::King => {
                if rng.gen_bool_ratio(BET_PROB_KING_NUM, BET_PROB_KING_DEN) {
                    KuhnPokerAction::Bet
                } else {
                    KuhnPokerAction::Pass
                }
            }
        }
    }

    fn encode_observation(agent_card: KuhnPokerCard, opponent_action: KuhnPokerAction) -> u8 {
        let action_code = match opponent_action {
            KuhnPokerAction::Bet => 0u8,
            KuhnPokerAction::Pass => 4u8,
        };
        action_code + agent_card.code()
    }

    fn reset_round(state: &mut KuhnPokerState, rng: &mut DeterministicRng) {
        state.agent_card = Self::random_card(rng);
        state.opponent_card = state.agent_card;
        while state.opponent_card == state.agent_card {
            state.opponent_card = Self::random_card(rng);
        }
        state.opponent_action = Self::opening_action(state.opponent_card, rng);
        state.observation = Self::encode_observation(state.agent_card, state.opponent_action);
    }

    fn showdown_agent_wins(state: &KuhnPokerState) -> bool {
        matches!(state.opponent_card, KuhnPokerCard::Jack)
            || (state.opponent_card == KuhnPokerCard::Queen
                && state.agent_card == KuhnPokerCard::King)
    }

    fn model_step(
        state: &mut KuhnPokerState,
        action: Option<KuhnPokerAction>,
        rng: &mut DeterministicRng,
    ) -> Reward {
        const R_BET_LOSS: Reward = -2;
        const R_PASS_LOSS: Reward = -1;
        const R_PASS_WIN: Reward = 1;
        const R_BET_WIN: Reward = 2;

        let Some(agent_action) = action else {
            state.reward = R_BET_LOSS;
            Self::reset_round(state, rng);
            return state.reward;
        };

        if agent_action == KuhnPokerAction::Pass && state.opponent_action == KuhnPokerAction::Bet {
            state.reward = R_PASS_LOSS;
            Self::reset_round(state, rng);
            return state.reward;
        }

        if agent_action == KuhnPokerAction::Bet && state.opponent_action == KuhnPokerAction::Pass {
            let opponent_calls = match state.opponent_card {
                KuhnPokerCard::Queen => rng.gen_bool_ratio(BET_PROB_QUEEN_NUM, BET_PROB_QUEEN_DEN),
                KuhnPokerCard::King => true,
                KuhnPokerCard::Jack => false,
            };
            if opponent_calls {
                state.opponent_action = KuhnPokerAction::Bet;
            } else {
                state.reward = R_PASS_WIN;
                Self::reset_round(state, rng);
                return state.reward;
            }
        }

        state.reward = if Self::showdown_agent_wins(state) {
            if state.opponent_action == KuhnPokerAction::Bet {
                R_BET_WIN
            } else {
                R_PASS_WIN
            }
        } else if agent_action == KuhnPokerAction::Bet {
            R_BET_LOSS
        } else {
            R_PASS_LOSS
        };

        Self::reset_round(state, rng);
        state.reward
    }
}

impl single_player::SinglePlayerGame for KuhnPoker {
    type Params = ();
    type State = KuhnPokerState;
    type Action = KuhnPokerAction;
    type Obs = KuhnPokerObservation;
    type ActionBuf = FixedVec<KuhnPokerAction, 2>;
    type WordBuf = FixedVec<u64, 1>;

    fn name(&self) -> &'static str {
        "kuhn_poker"
    }

    fn init_with_params(&self, seed: Seed, _params: &Self::Params) -> Self::State {
        let mut rng = DeterministicRng::from_seed_and_stream(seed, 0);
        let mut state = KuhnPokerState {
            reward: 0,
            ..Default::default()
        };
        Self::reset_round(&mut state, &mut rng);
        state
    }

    fn is_terminal(&self, _state: &Self::State) -> bool {
        false
    }

    fn legal_actions(&self, _state: &Self::State, out: &mut Self::ActionBuf) {
        out.clear();
        out.push(KuhnPokerAction::Bet).unwrap();
        out.push(KuhnPokerAction::Pass).unwrap();
    }

    fn observe_player(&self, state: &Self::State) -> Self::Obs {
        KuhnPokerObservation {
            value: state.observation,
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
            action_count: 2,
            observation_bits: 3,
            observation_stream_len: 1,
            reward_bits: 3,
            min_reward: -2,
            max_reward: 2,
            reward_offset: 2,
        }
    }

    fn encode_action(&self, action: &Self::Action) -> u64 {
        encode_enum_action(*action, &KUHN_POKER_ACTION_ORDER)
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        decode_enum_action(encoded, &KUHN_POKER_ACTION_ORDER)
    }

    fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        out.clear();
        out.push(u64::from(observation.value)).unwrap();
    }

    fn state_invariant(&self, state: &Self::State) -> bool {
        state.agent_card != state.opponent_card
            && state.observation
                == Self::encode_observation(state.agent_card, state.opponent_action)
            && (-2..=2).contains(&state.reward)
    }

    fn player_observation_invariant(&self, _state: &Self::State, observation: &Self::Obs) -> bool {
        observation.value < 8
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
        reward_and_terminal_postcondition(outcome.reward_for(0), -2, 2, false, false)
            && !outcome.is_terminal()
    }
}

impl OracleProjection for KuhnPoker {
    type WorldView = KuhnPokerWorldView;

    fn world_view(&self, state: &Self::State) -> Self::WorldView {
        *state
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs;
