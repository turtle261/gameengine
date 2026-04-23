//! Builtin biased coin-flip environment with binary prediction rewards.

use crate::buffer::FixedVec;
use crate::compact::{CompactSpec, decode_enum_action, encode_enum_action};
use crate::core::single_player::{self, SinglePlayerRewardBuf};
use crate::game::OracleProjection;
use crate::rng::DeterministicRng;
use crate::types::{KernelOutcome, Reward, Seed, Termination};
use crate::verification::reward_and_terminal_postcondition;

const BIASED_COINFLIP_ACTION_ORDER: [BiasedCoinFlipAction; 2] = [
    BiasedCoinFlipAction::GuessTails,
    BiasedCoinFlipAction::GuessHeads,
];

/// Agent guess for the next coin-flip outcome.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BiasedCoinFlipAction {
    /// Predict tail (`0`).
    #[default]
    GuessTails,
    /// Predict head (`1`).
    GuessHeads,
}

/// Parameter bundle configuring the Bernoulli bias.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BiasedCoinFlipConfig {
    /// Probability numerator for sampling head.
    pub head_numerator: u64,
    /// Probability denominator for sampling head.
    pub head_denominator: u64,
}

impl Default for BiasedCoinFlipConfig {
    fn default() -> Self {
        Self {
            head_numerator: 7,
            head_denominator: 10,
        }
    }
}

impl BiasedCoinFlipConfig {
    /// Returns true when `head_numerator / head_denominator` is valid.
    pub fn invariant(self) -> bool {
        self.head_denominator > 0 && self.head_numerator <= self.head_denominator
    }
}

/// Complete deterministic environment state.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BiasedCoinFlipState {
    /// Active immutable configuration.
    pub config: BiasedCoinFlipConfig,
    /// Current observed coin face (`0` tail, `1` head).
    pub coin_face: u8,
    /// Last reward.
    pub reward: Reward,
}

/// Player observation exposing the currently sampled coin face.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BiasedCoinFlipObservation {
    /// Current observed coin face (`0` tail, `1` head).
    pub coin_face: u8,
}

/// Full world/debug view type.
pub type BiasedCoinFlipWorldView = BiasedCoinFlipState;

/// Builtin biased coin-flip environment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BiasedCoinFlip {
    /// Default configuration used by initialization helpers.
    pub config: BiasedCoinFlipConfig,
}

impl BiasedCoinFlip {
    /// Creates a game with validated Bernoulli parameters.
    pub fn new(config: BiasedCoinFlipConfig) -> Self {
        assert!(config.invariant(), "invalid biased coin-flip config");
        Self { config }
    }

    fn sample_coin_face(config: BiasedCoinFlipConfig, rng: &mut DeterministicRng) -> u8 {
        if config.head_denominator == 0 {
            return 0;
        }
        let draw = rng.gen_range(config.head_denominator as usize) as u64;
        if draw < config.head_numerator { 1 } else { 0 }
    }

    fn action_to_coin_face(action: BiasedCoinFlipAction) -> u8 {
        match action {
            BiasedCoinFlipAction::GuessTails => 0,
            BiasedCoinFlipAction::GuessHeads => 1,
        }
    }

    fn model_step(
        state: &mut BiasedCoinFlipState,
        action: Option<BiasedCoinFlipAction>,
        rng: &mut DeterministicRng,
    ) -> Reward {
        state.coin_face = Self::sample_coin_face(state.config, rng);
        let guessed = action.map(Self::action_to_coin_face);
        state.reward = if guessed == Some(state.coin_face) {
            1
        } else {
            0
        };
        state.reward
    }
}

impl single_player::SinglePlayerGame for BiasedCoinFlip {
    type Params = BiasedCoinFlipConfig;
    type State = BiasedCoinFlipState;
    type Action = BiasedCoinFlipAction;
    type Obs = BiasedCoinFlipObservation;
    type ActionBuf = FixedVec<BiasedCoinFlipAction, 2>;
    type WordBuf = FixedVec<u64, 1>;

    fn name(&self) -> &'static str {
        "coin_flip"
    }

    fn default_params(&self) -> Self::Params {
        self.config
    }

    fn params_invariant(&self, params: &Self::Params) -> bool {
        params.invariant()
    }

    fn init_with_params(&self, seed: Seed, params: &Self::Params) -> Self::State {
        let mut rng = DeterministicRng::from_seed_and_stream(seed, 0);
        BiasedCoinFlipState {
            config: *params,
            coin_face: Self::sample_coin_face(*params, &mut rng),
            reward: 0,
        }
    }

    fn is_terminal(&self, _state: &Self::State) -> bool {
        false
    }

    fn legal_actions(&self, _state: &Self::State, out: &mut Self::ActionBuf) {
        out.clear();
        out.push(BiasedCoinFlipAction::GuessTails).unwrap();
        out.push(BiasedCoinFlipAction::GuessHeads).unwrap();
    }

    fn observe_player(&self, state: &Self::State) -> Self::Obs {
        BiasedCoinFlipObservation {
            coin_face: state.coin_face,
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
            observation_bits: 1,
            observation_stream_len: 1,
            reward_bits: 1,
            min_reward: 0,
            max_reward: 1,
            reward_offset: 0,
        }
    }

    fn encode_action(&self, action: &Self::Action) -> u64 {
        encode_enum_action(*action, &BIASED_COINFLIP_ACTION_ORDER)
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        decode_enum_action(encoded, &BIASED_COINFLIP_ACTION_ORDER)
    }

    fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        out.clear();
        out.push(u64::from(observation.coin_face)).unwrap();
    }

    fn state_invariant(&self, state: &Self::State) -> bool {
        state.config.invariant() && state.coin_face <= 1 && (0..=1).contains(&state.reward)
    }

    fn player_observation_invariant(&self, _state: &Self::State, observation: &Self::Obs) -> bool {
        observation.coin_face <= 1
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
        reward_and_terminal_postcondition(outcome.reward_for(0), 0, 1, false, false)
            && !outcome.is_terminal()
    }
}

impl OracleProjection for BiasedCoinFlip {
    type WorldView = BiasedCoinFlipWorldView;

    fn world_view(&self, state: &Self::State) -> Self::WorldView {
        *state
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs;
