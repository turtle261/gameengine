//! Runtime contract-check helpers for transitions, observations, and compact codecs.

use crate::buffer::Buffer;
use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::{Reward, Seed, StepOutcome};

/// Returns true when a reward stays in range and terminal flags remain consistent.
pub fn reward_and_terminal_postcondition(
    reward: Reward,
    min_reward: Reward,
    max_reward: Reward,
    post_terminal: bool,
    outcome_terminal: bool,
) -> bool {
    (min_reward..=max_reward).contains(&reward) && (post_terminal == outcome_terminal)
}

/// Asserts deterministic transition and postcondition contracts for one step.
pub fn assert_transition_contracts<G: Game>(
    game: &G,
    pre: &G::State,
    actions: &G::JointActionBuf,
    seed: Seed,
) {
    assert!(game.state_invariant(pre));
    for action in actions.as_slice() {
        assert!(game.action_invariant(&action.action));
    }

    let mut left_state = pre.clone();
    let mut right_state = pre.clone();
    let mut left_rng = DeterministicRng::from_seed_and_stream(seed, 99);
    let mut right_rng = DeterministicRng::from_seed_and_stream(seed, 99);
    let mut left_outcome = StepOutcome::<G::RewardBuf>::default();
    let mut right_outcome = StepOutcome::<G::RewardBuf>::default();

    game.step_in_place(&mut left_state, actions, &mut left_rng, &mut left_outcome);
    game.step_in_place(
        &mut right_state,
        actions,
        &mut right_rng,
        &mut right_outcome,
    );

    assert_eq!(left_state, right_state);
    assert_eq!(left_outcome, right_outcome);
    assert_eq!(left_rng, right_rng);
    assert!(game.state_invariant(&left_state));
    assert!(game.transition_postcondition(pre, actions, &left_state, &left_outcome,));
}

/// Asserts player, spectator, and world-view observation contracts.
pub fn assert_observation_contracts<G: Game>(game: &G, state: &G::State) {
    assert!(game.state_invariant(state));
    for player in 0..game.player_count() {
        let observation = game.observe_player(state, player);
        assert!(game.player_observation_invariant(state, player, &observation,));
    }
    let spectator = game.observe_spectator(state);
    assert!(game.spectator_observation_invariant(state, &spectator));
    let world = game.world_view(state);
    assert!(game.world_view_invariant(state, &world));
}

/// Asserts compact action encoding roundtrips through decode.
pub fn assert_compact_roundtrip<G: Game>(game: &G, action: &G::Action) {
    if game.compact_spec().action_count == 0 {
        return;
    }
    let encoded = game.encode_action(action);
    assert_eq!(game.decode_action(encoded), Some(*action));
}

#[cfg(test)]
mod tests {
    use super::assert_compact_roundtrip;
    use crate::buffer::FixedVec;
    use crate::compact::CompactSpec;
    use crate::game::Game;
    use crate::rng::DeterministicRng;
    use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome};

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct MinimalGame {
        compact_actions: u64,
    }

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct MinimalState;

    impl Game for MinimalGame {
        type State = MinimalState;
        type Action = u8;
        type PlayerObservation = u8;
        type SpectatorObservation = u8;
        type WorldView = u8;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 1>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn name(&self) -> &'static str {
            "minimal"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init(&self, _seed: Seed) -> Self::State {
            MinimalState
        }

        fn is_terminal(&self, _state: &Self::State) -> bool {
            false
        }

        fn players_to_act(&self, _state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            out.push(0).unwrap();
        }

        fn legal_actions(
            &self,
            _state: &Self::State,
            _player: PlayerId,
            out: &mut Self::ActionBuf,
        ) {
            out.clear();
            out.push(0).unwrap();
        }

        fn observe_player(
            &self,
            _state: &Self::State,
            _player: PlayerId,
        ) -> Self::PlayerObservation {
            0
        }

        fn observe_spectator(&self, _state: &Self::State) -> Self::SpectatorObservation {
            0
        }

        fn world_view(&self, _state: &Self::State) -> Self::WorldView {
            0
        }

        fn step_in_place(
            &self,
            _state: &mut Self::State,
            _joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut StepOutcome<Self::RewardBuf>,
        ) {
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: 0,
                })
                .unwrap();
        }

        fn compact_spec(&self) -> CompactSpec {
            CompactSpec {
                action_count: self.compact_actions,
                observation_bits: 0,
                observation_stream_len: 0,
                reward_bits: 1,
                min_reward: 0,
                max_reward: 0,
                reward_offset: 0,
            }
        }
    }

    #[test]
    fn compact_roundtrip_is_skipped_when_action_codec_is_absent() {
        let game = MinimalGame { compact_actions: 0 };
        assert_compact_roundtrip(&game, &0);
    }

    #[test]
    #[should_panic]
    fn compact_roundtrip_still_checks_declared_codec_surface() {
        let game = MinimalGame { compact_actions: 1 };
        assert_compact_roundtrip(&game, &0);
    }
}
