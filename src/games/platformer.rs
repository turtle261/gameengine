use crate::buffer::{FixedVec, default_array};
use crate::compact::{CompactGame, CompactSpec};
use crate::game::Game;
use crate::math::{Aabb2, StrictF64, Vec2};
use crate::physics::{BodyKind, Contact2d, PhysicsBody2d, PhysicsOracleView2d, PhysicsWorld2d};
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Reward, Seed, StepOutcome, Termination};

const BERRY_COUNT: usize = 6;
const PLAYER_BODY_ID: u16 = 1;
const FIRST_BERRY_BODY_ID: u16 = 10;
const PLATFORMER_BODIES: usize = 1 + BERRY_COUNT;
const PLATFORMER_CONTACTS: usize = PLATFORMER_BODIES * (PLATFORMER_BODIES - 1) / 2;
const ALL_BERRIES_MASK: u8 = 0b00_111111;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PlatformerAction {
    #[default]
    Stay,
    Left,
    Right,
    Jump,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PlatformerConfig {
    pub width: u8,
    pub height: u8,
    pub player_width: u8,
    pub player_height: u8,
    pub jump_delta: u8,
    pub berry_y: u8,
    pub berry_xs: [u8; BERRY_COUNT],
    pub sprain_numerator: u64,
    pub sprain_denominator: u64,
    pub berry_reward: Reward,
    pub finish_bonus: Reward,
}

impl Default for PlatformerConfig {
    fn default() -> Self {
        Self {
            width: 12,
            height: 3,
            player_width: 1,
            player_height: 1,
            jump_delta: 1,
            berry_y: 2,
            berry_xs: [1, 3, 5, 7, 9, 11],
            sprain_numerator: 1,
            sprain_denominator: 10,
            berry_reward: 1,
            finish_bonus: 10,
        }
    }
}

impl PlatformerConfig {
    pub fn arena_bounds(self) -> Aabb2<StrictF64> {
        Aabb2::new(
            Vec2::new(StrictF64::new(0.0), StrictF64::new(0.0)),
            Vec2::new(
                StrictF64::new(self.width as f64),
                StrictF64::new(self.height as f64),
            ),
        )
    }

    pub fn player_half_extents(self) -> Vec2<StrictF64> {
        Vec2::new(
            StrictF64::new(self.player_width as f64 / 2.0),
            StrictF64::new(self.player_height as f64 / 2.0),
        )
    }

    pub fn player_center(self, x: u8, y: u8) -> Vec2<StrictF64> {
        Vec2::new(
            StrictF64::new(x as f64 + self.player_width as f64 / 2.0),
            StrictF64::new(y as f64 + self.player_height as f64 / 2.0),
        )
    }

    pub fn berry_center(self, index: usize) -> Vec2<StrictF64> {
        Vec2::new(
            StrictF64::new(self.berry_xs[index] as f64 + 0.5),
            StrictF64::new(self.berry_y as f64),
        )
    }

    pub fn invariant(self) -> bool {
        if self.width == 0
            || self.height == 0
            || self.player_width == 0
            || self.player_height == 0
            || self.player_width > self.width
            || self.player_height > self.height
            || self.jump_delta >= self.height
            || self.sprain_denominator == 0
            || self.sprain_numerator > self.sprain_denominator
            || self.berry_y >= self.height
        {
            return false;
        }

        let mut index = 1usize;
        while index < self.berry_xs.len() {
            if self.berry_xs[index - 1] >= self.berry_xs[index]
                || self.berry_xs[index] >= self.width
            {
                return false;
            }
            index += 1;
        }

        true
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PlatformerState {
    pub world: PhysicsWorld2d<PLATFORMER_BODIES, PLATFORMER_CONTACTS>,
    pub remaining_berries: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct PlatformerObservation {
    pub x: u8,
    pub y: u8,
    pub remaining_berries: u8,
    pub terminal: bool,
    pub winner: Option<PlayerId>,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BerryView {
    pub id: u16,
    pub x: u8,
    pub y: u8,
    pub collected: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PlatformerWorldView {
    pub config: PlatformerConfig,
    pub physics: PhysicsWorld2d<PLATFORMER_BODIES, PLATFORMER_CONTACTS>,
    pub berries: [BerryView; BERRY_COUNT],
}

impl Default for PlatformerState {
    fn default() -> Self {
        Platformer::default().init(0)
    }
}

impl Default for PlatformerWorldView {
    fn default() -> Self {
        Platformer::default().world_view(&Platformer::default().init(0))
    }
}

impl PhysicsOracleView2d for PlatformerWorldView {
    fn bounds(&self) -> Aabb2<StrictF64> {
        self.physics.bounds()
    }

    fn tick(&self) -> u64 {
        self.physics.tick()
    }

    fn bodies(&self) -> &[PhysicsBody2d] {
        self.physics.bodies()
    }

    fn contacts(&self) -> &[Contact2d] {
        self.physics.contacts()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Platformer {
    pub config: PlatformerConfig,
}

impl Platformer {
    pub fn new(config: PlatformerConfig) -> Self {
        assert!(config.invariant(), "invalid platformer config");
        Self { config }
    }

    fn player_body(state: &PlatformerState) -> &PhysicsBody2d {
        state.world.require_body(PLAYER_BODY_ID)
    }

    fn player_position(&self, state: &PlatformerState) -> (u8, u8) {
        let player = Self::player_body(state);
        let min = player.aabb().min;
        let x = min.x.to_f64();
        let y = min.y.to_f64();
        debug_assert!(x >= 0.0 && y >= 0.0);
        (x as u8, y as u8)
    }

    fn is_terminal_state(state: &PlatformerState) -> bool {
        state.remaining_berries == 0
    }

    fn winner(state: &PlatformerState) -> Option<PlayerId> {
        Self::is_terminal_state(state).then_some(0)
    }

    fn sync_berries(&self, state: &mut PlatformerState) {
        for index in 0..BERRY_COUNT {
            let berry_id = FIRST_BERRY_BODY_ID + index as u16;
            state
                .world
                .set_body_active_deferred(berry_id, state.remaining_berries & (1u8 << index) != 0);
        }
    }

    fn collect_berries_from_contacts(&self, state: &mut PlatformerState) -> Reward {
        let mut reward = 0;
        for index in 0..BERRY_COUNT {
            let berry_bit = 1u8 << index;
            let berry_id = FIRST_BERRY_BODY_ID + index as u16;
            if state.remaining_berries & berry_bit != 0
                && state.world.has_contact(PLAYER_BODY_ID, berry_id)
            {
                state.remaining_berries &= !berry_bit;
                state.world.set_body_active(berry_id, false);
                reward += self.config.berry_reward;
            }
        }
        if state.remaining_berries == 0 {
            reward += self.config.finish_bonus;
        }
        reward
    }

    fn observation_from_state(&self, state: &PlatformerState) -> PlatformerObservation {
        let (x, y) = self.player_position(state);
        PlatformerObservation {
            x,
            y,
            remaining_berries: state.remaining_berries,
            terminal: Self::is_terminal_state(state),
            winner: Self::winner(state),
        }
    }

    fn build_world(&self) -> PhysicsWorld2d<PLATFORMER_BODIES, PLATFORMER_CONTACTS> {
        let mut world = PhysicsWorld2d::new(self.config.arena_bounds());
        world.add_body_deferred(PhysicsBody2d {
            id: PLAYER_BODY_ID,
            kind: BodyKind::Kinematic,
            position: self.config.player_center(0, 0),
            half_extents: self.config.player_half_extents(),
            active: true,
        });
        for index in 0..BERRY_COUNT {
            world.add_body_deferred(PhysicsBody2d {
                id: FIRST_BERRY_BODY_ID + index as u16,
                kind: BodyKind::Trigger,
                position: self.config.berry_center(index),
                half_extents: Vec2::new(StrictF64::new(0.0), StrictF64::new(0.0)),
                active: true,
            });
        }
        world.refresh_contacts();
        world
    }
}

impl Game for Platformer {
    type State = PlatformerState;
    type Action = PlatformerAction;
    type PlayerObservation = PlatformerObservation;
    type SpectatorObservation = PlatformerObservation;
    type WorldView = PlatformerWorldView;
    type PlayerBuf = FixedVec<PlayerId, 1>;
    type ActionBuf = FixedVec<PlatformerAction, 4>;
    type JointActionBuf = FixedVec<PlayerAction<PlatformerAction>, 1>;
    type RewardBuf = FixedVec<PlayerReward, 1>;
    type WordBuf = FixedVec<u64, 1>;

    fn name(&self) -> &'static str {
        "platformer"
    }

    fn player_count(&self) -> usize {
        1
    }

    fn init(&self, _seed: Seed) -> Self::State {
        assert!(self.config.invariant());
        PlatformerState {
            world: self.build_world(),
            remaining_berries: ALL_BERRIES_MASK,
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        Self::is_terminal_state(state)
    }

    fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf) {
        out.clear();
        if !self.is_terminal(state) {
            out.push(0).unwrap();
        }
    }

    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf) {
        out.clear();
        if player != 0 || self.is_terminal(state) {
            return;
        }
        out.push(PlatformerAction::Stay).unwrap();
        out.push(PlatformerAction::Left).unwrap();
        out.push(PlatformerAction::Right).unwrap();
        out.push(PlatformerAction::Jump).unwrap();
    }

    fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::PlayerObservation {
        self.observation_from_state(state)
    }

    fn observe_spectator(&self, state: &Self::State) -> Self::SpectatorObservation {
        self.observation_from_state(state)
    }

    fn world_view(&self, state: &Self::State) -> Self::WorldView {
        let mut berries = default_array::<BerryView, BERRY_COUNT>();
        let mut index = 0usize;
        while index < BERRY_COUNT {
            berries[index] = BerryView {
                id: FIRST_BERRY_BODY_ID + index as u16,
                x: self.config.berry_xs[index],
                y: self.config.berry_y,
                collected: (state.remaining_berries & (1u8 << index)) == 0,
            };
            index += 1;
        }
        PlatformerWorldView {
            config: self.config,
            physics: state.world.clone(),
            berries,
        }
    }

    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<Self::RewardBuf>,
    ) {
        let actions = joint_actions.as_slice();
        let mut action = PlatformerAction::Stay;
        let mut action_index = 0usize;
        while action_index < actions.len() {
            let candidate = &actions[action_index];
            if candidate.player == 0 {
                action = candidate.action;
                break;
            }
            action_index += 1;
        }

        let mut reward = 0;
        if self.is_terminal(state) {
            out.termination = Termination::Terminal {
                winner: Self::winner(state),
            };
        } else {
            let (current_x, _) = self.player_position(state);
            let (x, y) = match action {
                PlatformerAction::Stay => (current_x, 0),
                PlatformerAction::Left => (current_x.saturating_sub(1), 0),
                PlatformerAction::Right => (
                    if current_x + self.config.player_width < self.config.width {
                        current_x + 1
                    } else {
                        current_x
                    },
                    0,
                ),
                PlatformerAction::Jump => {
                    if rng.gen_bool_ratio(
                        self.config.sprain_numerator,
                        self.config.sprain_denominator,
                    ) {
                        reward -= 1;
                    }
                    (current_x, self.config.jump_delta)
                }
            };

            state
                .world
                .set_body_position_deferred(PLAYER_BODY_ID, self.config.player_center(x, y));
            state.world.refresh_contacts();
            reward += self.collect_berries_from_contacts(state);
            self.sync_berries(state);
            state.world.step();

            out.termination = if self.is_terminal(state) {
                Termination::Terminal {
                    winner: Self::winner(state),
                }
            } else {
                Termination::Ongoing
            };
        }

        out.rewards
            .push(PlayerReward { player: 0, reward })
            .unwrap();
    }

    fn state_invariant(&self, state: &Self::State) -> bool {
        if !self.config.invariant()
            || state.remaining_berries & !ALL_BERRIES_MASK != 0
            || !state.world.invariant()
            || state.world.bodies.len() != PLATFORMER_BODIES
        {
            return false;
        }

        let player = Self::player_body(state);
        if player.kind != BodyKind::Kinematic
            || !player.active
            || player.half_extents != self.config.player_half_extents()
        {
            return false;
        }

        let (x, y) = self.player_position(state);
        if x + self.config.player_width > self.config.width || y > self.config.jump_delta {
            return false;
        }

        for index in 0..BERRY_COUNT {
            let berry = state.world.require_body(FIRST_BERRY_BODY_ID + index as u16);
            let expected_active = state.remaining_berries & (1u8 << index) != 0;
            if berry.kind != BodyKind::Trigger
                || berry.position != self.config.berry_center(index)
                || berry.active != expected_active
            {
                return false;
            }
        }

        true
    }

    fn player_observation_invariant(
        &self,
        state: &Self::State,
        _player: PlayerId,
        observation: &Self::PlayerObservation,
    ) -> bool {
        observation == &self.observation_from_state(state)
    }

    fn spectator_observation_invariant(
        &self,
        state: &Self::State,
        observation: &Self::SpectatorObservation,
    ) -> bool {
        observation == &self.observation_from_state(state)
    }

    fn world_view_invariant(&self, state: &Self::State, world: &Self::WorldView) -> bool {
        if world.config != self.config || world.physics != state.world {
            return false;
        }

        let mut index = 0usize;
        while index < world.berries.len() {
            let berry = world.berries[index];
            if berry.id != FIRST_BERRY_BODY_ID + index as u16
                || berry.x != self.config.berry_xs[index]
                || berry.y != self.config.berry_y
                || berry.collected != ((state.remaining_berries & (1u8 << index)) == 0)
            {
                return false;
            }
            index += 1;
        }

        true
    }

    fn transition_postcondition(
        &self,
        _pre: &Self::State,
        _actions: &Self::JointActionBuf,
        post: &Self::State,
        outcome: &StepOutcome<Self::RewardBuf>,
    ) -> bool {
        matches!(outcome.reward_for(0), -1..=11)
            && (post.remaining_berries == 0) == outcome.is_terminal()
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

    fn encode_player_observation(
        &self,
        observation: &Self::PlayerObservation,
        out: &mut Self::WordBuf,
    ) {
        out.clear();
        let packed = u64::from(observation.x)
            | (u64::from(observation.y) << 4)
            | (u64::from(observation.remaining_berries) << 5)
            | ((observation.terminal as u64) << 11);
        out.push(packed).unwrap();
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
    fn movement_clamps_at_walls() {
        let game = Platformer::default();
        let mut state = game.init(1);
        let mut rng = DeterministicRng::from_seed_and_stream(1, 1);
        let mut outcome = StepOutcome::<FixedVec<PlayerReward, 1>>::default();
        let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: PlatformerAction::Left,
            })
            .unwrap();
        game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
        assert_eq!(game.observe_spectator(&state).x, 0);

        state
            .world
            .set_body_position(PLAYER_BODY_ID, game.config.player_center(11, 0));
        outcome.clear();
        actions.clear();
        actions
            .push(PlayerAction {
                player: 0,
                action: PlatformerAction::Right,
            })
            .unwrap();
        game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
        assert_eq!(game.observe_spectator(&state).x, 11);
    }

    #[test]
    fn berry_collection_is_idempotent() {
        let game = Platformer::default();
        let mut state = game.init(1);
        state
            .world
            .set_body_position(PLAYER_BODY_ID, game.config.player_center(1, 0));
        let mut rng = DeterministicRng::from_seed_and_stream(1, 1);
        let mut outcome = StepOutcome::<FixedVec<PlayerReward, 1>>::default();
        let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: PlatformerAction::Jump,
            })
            .unwrap();

        game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
        let remaining = state.remaining_berries;
        outcome.clear();
        game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
        assert_eq!(state.remaining_berries, remaining);
    }

    #[test]
    fn final_berry_terminates_with_bonus() {
        let game = Platformer::default();
        let mut state = game.init(9);
        state.remaining_berries = 1u8 << 5;
        game.sync_berries(&mut state);
        state
            .world
            .set_body_position(PLAYER_BODY_ID, game.config.player_center(11, 0));
        let mut rng = DeterministicRng::from_seed_and_stream(9, 1);
        let mut outcome = StepOutcome::<FixedVec<PlayerReward, 1>>::default();
        let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: PlatformerAction::Jump,
            })
            .unwrap();
        game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
        assert!(game.is_terminal(&state));
        assert!(outcome.reward_for(0) >= 10);
    }

    #[test]
    fn seeded_sessions_replay_exactly() {
        let mut left = Session::new(Platformer::default(), 3);
        let mut right = Session::new(Platformer::default(), 3);
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

    #[test]
    fn verification_helpers_hold_for_jump() {
        let game = Platformer::default();
        let state = game.init(3);
        let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: PlatformerAction::Jump,
            })
            .unwrap();
        assert_transition_contracts(&game, &state, &actions, 3);
        assert_observation_contracts(&game, &state);
        assert_compact_roundtrip(&game, &PlatformerAction::Jump);
    }

    #[test]
    fn physics_world_tracks_actor_and_berries() {
        let state = Platformer::default().init(3);
        let world = Platformer::default().world_view(&state);
        assert_eq!(world.physics.bodies.len(), PLATFORMER_BODIES);
        assert!(world.physics.invariant());
    }
}

#[cfg(kani)]
mod proofs {
    use super::{ALL_BERRIES_MASK, PLAYER_BODY_ID, Platformer, PlatformerAction, PlatformerState};
    use crate::buffer::FixedVec;
    use crate::game::Game;
    use crate::types::PlayerAction;

    #[kani::proof]
    #[kani::unwind(64)]
    fn wall_clamps_hold_for_all_edge_positions() {
        let game = Platformer::default();
        let mut state = PlatformerState::default();
        let x: u8 = kani::any();
        kani::assume(x < game.config.width);
        state
            .world
            .set_body_position(PLAYER_BODY_ID, game.config.player_center(x, 0));
        let mut rng = crate::rng::DeterministicRng::from_seed(1);
        let mut outcome =
            crate::types::StepOutcome::<FixedVec<crate::types::PlayerReward, 1>>::default();
        let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: PlatformerAction::Left,
            })
            .unwrap();
        game.step_in_place(&mut state, &actions, &mut rng, &mut outcome);
        assert!(game.observe_spectator(&state).x < game.config.width);
    }

    #[kani::proof]
    #[kani::unwind(64)]
    fn jump_reward_is_bounded() {
        let state = Platformer::default().init(1);
        let mut actions = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: PlatformerAction::Jump,
            })
            .unwrap();
        crate::verification::assert_transition_contracts(
            &Platformer::default(),
            &state,
            &actions,
            1,
        );
    }

    #[kani::proof]
    #[kani::unwind(64)]
    fn initial_observation_and_world_contracts_hold() {
        let game = Platformer::default();
        let state = game.init(1);
        crate::verification::assert_observation_contracts(&game, &state);
    }

    #[kani::proof]
    #[kani::unwind(64)]
    fn berry_mask_tracks_trigger_activation() {
        let mut state = PlatformerState::default();
        state.remaining_berries = ALL_BERRIES_MASK ^ 0b000001;
        Platformer::default().sync_berries(&mut state);
        assert!(!state.world.require_body(super::FIRST_BERRY_BODY_ID).active);
    }
}
