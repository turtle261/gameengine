use crate::compact::{CompactGame, CompactSpec};
use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};

const BERRY_XS: [u8; 6] = [1, 3, 5, 7, 9, 11];
const ALL_BERRIES_MASK: u8 = 0b00_111111;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PlatformerAction {
    Stay,
    Left,
    Right,
    Jump,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PlatformerState {
    pub x: u8,
    pub y: u8,
    pub remaining_berries: u8,
    pub terminal: bool,
    pub winner: Option<PlayerId>,
}

pub type PlatformerObservation = PlatformerState;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Platformer;

impl Platformer {
    fn berry_index_at_x(x: u8) -> Option<u8> {
        BERRY_XS
            .iter()
            .position(|candidate| *candidate == x)
            .map(|index| index as u8)
    }

    fn collect_berry(state: &mut PlatformerState) -> i64 {
        if let Some(index) = Self::berry_index_at_x(state.x) {
            let bit = 1u8 << index;
            if state.remaining_berries & bit != 0 {
                state.remaining_berries &= !bit;
                let mut reward = 1;
                if state.remaining_berries == 0 {
                    state.terminal = true;
                    state.winner = Some(0);
                    reward += 10;
                }
                return reward;
            }
        }
        0
    }
}

impl Game for Platformer {
    type State = PlatformerState;
    type Action = PlatformerAction;
    type PlayerObservation = PlatformerObservation;
    type SpectatorObservation = PlatformerObservation;

    fn name(&self) -> &'static str {
        "platformer"
    }

    fn player_count(&self) -> usize {
        1
    }

    fn init(&self, _seed: Seed) -> Self::State {
        PlatformerState {
            x: 0,
            y: 0,
            remaining_berries: ALL_BERRIES_MASK,
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
        out.push(PlatformerAction::Stay);
        out.push(PlatformerAction::Left);
        out.push(PlatformerAction::Right);
        out.push(PlatformerAction::Jump);
    }

    fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::PlayerObservation {
        state.clone()
    }

    fn observe_spectator(&self, state: &Self::State) -> Self::SpectatorObservation {
        state.clone()
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
            .map(|candidate| candidate.action)
            .unwrap_or(PlatformerAction::Stay);

        let mut reward = 0i64;
        if state.terminal {
            out.termination = Termination::Terminal {
                winner: state.winner,
            };
        } else {
            match action {
                PlatformerAction::Stay => {
                    state.y = 0;
                }
                PlatformerAction::Left => {
                    state.y = 0;
                    state.x = state.x.saturating_sub(1);
                }
                PlatformerAction::Right => {
                    state.y = 0;
                    if state.x < 11 {
                        state.x += 1;
                    }
                }
                PlatformerAction::Jump => {
                    state.y = 1;
                    if rng.gen_bool_ratio(1, 10) {
                        reward -= 1;
                    }
                    reward += Self::collect_berry(state);
                }
            }

            if state.terminal {
                out.termination = Termination::Terminal {
                    winner: state.winner,
                };
            }
        }

        out.rewards.push(PlayerReward { player: 0, reward });
        if !state.terminal {
            out.termination = Termination::Ongoing;
        }
    }
}

impl CompactGame for Platformer {
    fn compact_spec(&self) -> CompactSpec {
        CompactSpec {
            action_count: 4,
            observation_bits: 12,
            observation_stream_len: 1,
            reward_bits: 4,
            min_reward: -1,
            max_reward: 11,
            reward_offset: 1,
        }
    }

    fn encode_action(&self, action: &Self::Action) -> u64 {
        match action {
            PlatformerAction::Stay => 0,
            PlatformerAction::Left => 1,
            PlatformerAction::Right => 2,
            PlatformerAction::Jump => 3,
        }
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        match encoded {
            0 => Some(PlatformerAction::Stay),
            1 => Some(PlatformerAction::Left),
            2 => Some(PlatformerAction::Right),
            3 => Some(PlatformerAction::Jump),
            _ => None,
        }
    }

    fn encode_player_observation(&self, observation: &Self::PlayerObservation, out: &mut Vec<u64>) {
        out.clear();
        let packed = u64::from(observation.x)
            | (u64::from(observation.y) << 4)
            | (u64::from(observation.remaining_berries) << 5)
            | ((observation.terminal as u64) << 11);
        out.push(packed);
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
    fn movement_clamps_at_walls() {
        let game = Platformer;
        let mut state = game.init(1);
        let mut rng = DeterministicRng::from_seed_and_stream(1, 1);
        let mut outcome = StepOutcome::with_player_capacity(1);
        game.step_in_place(
            &mut state,
            &[PlayerAction {
                player: 0,
                action: PlatformerAction::Left,
            }],
            &mut rng,
            &mut outcome,
        );
        assert_eq!(state.x, 0);

        state.x = 11;
        game.step_in_place(
            &mut state,
            &[PlayerAction {
                player: 0,
                action: PlatformerAction::Right,
            }],
            &mut rng,
            &mut outcome,
        );
        assert_eq!(state.x, 11);
    }

    #[test]
    fn berry_collection_is_idempotent() {
        let game = Platformer;
        let mut state = game.init(1);
        state.x = 1;
        let mut rng = DeterministicRng::from_seed_and_stream(1, 1);
        let mut outcome = StepOutcome::with_player_capacity(1);

        game.step_in_place(
            &mut state,
            &[PlayerAction {
                player: 0,
                action: PlatformerAction::Jump,
            }],
            &mut rng,
            &mut outcome,
        );
        let remaining = state.remaining_berries;
        game.step_in_place(
            &mut state,
            &[PlayerAction {
                player: 0,
                action: PlatformerAction::Jump,
            }],
            &mut rng,
            &mut outcome,
        );
        assert_eq!(state.remaining_berries, remaining);
    }

    #[test]
    fn final_berry_terminates_with_bonus() {
        let game = Platformer;
        let mut state = PlatformerState {
            x: 11,
            y: 0,
            remaining_berries: 1u8 << 5,
            terminal: false,
            winner: None,
        };
        let mut rng = DeterministicRng::from_seed_and_stream(9, 1);
        let mut outcome = StepOutcome::with_player_capacity(1);
        game.step_in_place(
            &mut state,
            &[PlayerAction {
                player: 0,
                action: PlatformerAction::Jump,
            }],
            &mut rng,
            &mut outcome,
        );
        assert!(state.terminal);
        assert!(outcome.reward_for(0) >= 10);
    }

    #[test]
    fn seeded_sessions_replay_exactly() {
        let mut left = Session::new(Platformer, 3);
        let mut right = Session::new(Platformer, 3);
        let actions = [
            PlayerAction {
                player: 0,
                action: PlatformerAction::Right,
            },
            PlayerAction {
                player: 0,
                action: PlatformerAction::Jump,
            },
            PlayerAction {
                player: 0,
                action: PlatformerAction::Right,
            },
        ];
        for action in actions {
            left.step(std::slice::from_ref(&action));
            right.step(std::slice::from_ref(&action));
        }
        assert_eq!(left.trace(), right.trace());
        assert_eq!(left.state(), right.state());
    }
}
