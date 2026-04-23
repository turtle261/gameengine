//! Builtin extended-tiger POMDP environment.

use crate::buffer::FixedVec;
use crate::compact::{CompactSpec, decode_enum_action, encode_enum_action};
use crate::core::single_player::{self, SinglePlayerRewardBuf};
use crate::game::OracleProjection;
use crate::rng::DeterministicRng;
use crate::types::{KernelOutcome, Reward, Seed, Termination};
use crate::verification::reward_and_terminal_postcondition;

const EXTENDED_TIGER_ACTION_ORDER: [ExtendedTigerAction; 4] = [
    ExtendedTigerAction::Stand,
    ExtendedTigerAction::Listen,
    ExtendedTigerAction::OpenDoorOne,
    ExtendedTigerAction::OpenDoorTwo,
];

/// Agent action in extended tiger.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ExtendedTigerAction {
    /// Stand up.
    #[default]
    Stand,
    /// Listen for door hint.
    Listen,
    /// Open door 1.
    OpenDoorOne,
    /// Open door 2.
    OpenDoorTwo,
}

/// Posture state.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TigerPosture {
    /// Sitting posture.
    #[default]
    Sitting,
    /// Standing posture.
    Standing,
}

/// Complete extended tiger environment state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ExtendedTigerState {
    /// Current posture.
    pub posture: TigerPosture,
    /// Hidden tiger door id.
    pub tiger_door: u8,
    /// Hidden gold door id.
    pub gold_door: u8,
    /// Current observation symbol.
    pub observation: u8,
    /// Last reward.
    pub reward: Reward,
}

impl Default for ExtendedTigerState {
    fn default() -> Self {
        Self {
            posture: TigerPosture::Sitting,
            tiger_door: 2,
            gold_door: 1,
            observation: 0,
            reward: 0,
        }
    }
}

/// Player observation stream element.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ExtendedTigerObservation {
    /// Observation symbol in `[0, 7]`.
    pub value: u8,
}

/// Full world/debug view type.
pub type ExtendedTigerWorldView = ExtendedTigerState;

/// Builtin extended tiger environment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ExtendedTiger;

impl ExtendedTiger {
    fn reset_doors(state: &mut ExtendedTigerState, rng: &mut DeterministicRng) {
        state.gold_door = if rng.gen_bool_ratio(1, 2) { 1 } else { 2 };
        state.tiger_door = if state.gold_door == 1 { 2 } else { 1 };
    }

    fn is_standing(state: &ExtendedTigerState) -> bool {
        matches!(state.posture, TigerPosture::Standing)
    }

    fn model_step(
        state: &mut ExtendedTigerState,
        action: Option<ExtendedTigerAction>,
        rng: &mut DeterministicRng,
    ) -> Reward {
        let Some(action) = action else {
            state.reward = -100;
            return state.reward;
        };

        match action {
            ExtendedTigerAction::Stand => {
                if Self::is_standing(state) {
                    state.reward = -1;
                } else {
                    state.posture = TigerPosture::Standing;
                    state.reward = -1;
                    if state.observation < 4 {
                        state.observation += 4;
                    }
                }
            }
            ExtendedTigerAction::Listen => {
                if Self::is_standing(state) || state.observation != 0 {
                    state.reward = -1;
                    state.observation = 0;
                } else {
                    state.observation = if rng.gen_bool_ratio(17, 20) {
                        state.tiger_door
                    } else {
                        state.gold_door
                    };
                    state.reward = -1;
                }
            }
            ExtendedTigerAction::OpenDoorOne => {
                if matches!(state.posture, TigerPosture::Sitting) {
                    state.reward = -100;
                } else {
                    state.reward = if state.gold_door == 1 { 30 } else { -100 };
                    state.observation = 0;
                    state.posture = TigerPosture::Sitting;
                    Self::reset_doors(state, rng);
                }
            }
            ExtendedTigerAction::OpenDoorTwo => {
                if matches!(state.posture, TigerPosture::Sitting) {
                    state.reward = -100;
                } else {
                    state.reward = if state.gold_door == 2 { 30 } else { -100 };
                    state.observation = 0;
                    state.posture = TigerPosture::Sitting;
                    Self::reset_doors(state, rng);
                }
            }
        }

        state.reward
    }
}

impl single_player::SinglePlayerGame for ExtendedTiger {
    type Params = ();
    type State = ExtendedTigerState;
    type Action = ExtendedTigerAction;
    type Obs = ExtendedTigerObservation;
    type ActionBuf = FixedVec<ExtendedTigerAction, 4>;
    type WordBuf = FixedVec<u64, 1>;

    fn name(&self) -> &'static str {
        "extended_tiger"
    }

    fn init_with_params(&self, seed: Seed, _params: &Self::Params) -> Self::State {
        let mut rng = DeterministicRng::from_seed_and_stream(seed, 0);
        let mut state = ExtendedTigerState::default();
        Self::reset_doors(&mut state, &mut rng);
        state
    }

    fn is_terminal(&self, _state: &Self::State) -> bool {
        false
    }

    fn legal_actions(&self, _state: &Self::State, out: &mut Self::ActionBuf) {
        out.clear();
        out.push(ExtendedTigerAction::Stand).unwrap();
        out.push(ExtendedTigerAction::Listen).unwrap();
        out.push(ExtendedTigerAction::OpenDoorOne).unwrap();
        out.push(ExtendedTigerAction::OpenDoorTwo).unwrap();
    }

    fn observe_player(&self, state: &Self::State) -> Self::Obs {
        ExtendedTigerObservation {
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
            action_count: 4,
            observation_bits: 3,
            observation_stream_len: 1,
            reward_bits: 8,
            min_reward: -100,
            max_reward: 30,
            reward_offset: 100,
        }
    }

    fn encode_action(&self, action: &Self::Action) -> u64 {
        encode_enum_action(*action, &EXTENDED_TIGER_ACTION_ORDER)
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        decode_enum_action(encoded, &EXTENDED_TIGER_ACTION_ORDER)
    }

    fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        out.clear();
        out.push(u64::from(observation.value)).unwrap();
    }

    fn state_invariant(&self, state: &Self::State) -> bool {
        (state.gold_door == 1 || state.gold_door == 2)
            && state.tiger_door == if state.gold_door == 1 { 2 } else { 1 }
            && state.observation <= 7
            && (-100..=30).contains(&state.reward)
    }

    fn player_observation_invariant(&self, _state: &Self::State, observation: &Self::Obs) -> bool {
        observation.value <= 7
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
        reward_and_terminal_postcondition(outcome.reward_for(0), -100, 30, false, false)
            && !outcome.is_terminal()
    }
}

impl OracleProjection for ExtendedTiger {
    type WorldView = ExtendedTigerWorldView;

    fn world_view(&self, state: &Self::State) -> Self::WorldView {
        *state
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs;
