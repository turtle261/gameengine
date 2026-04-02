//! Builtin deterministic platformer environment backed by fixed-capacity physics.

use crate::buffer::{Buffer, FixedVec};
use crate::compact::{CompactSpec, decode_enum_action, encode_enum_action};
use crate::core::single_player::{self, SinglePlayerRewardBuf};
use crate::math::{Aabb2, StrictF64, Vec2};
use crate::physics::{
    BodyKind, PhysicsBody2d, PhysicsWorld2d, collect_actor_trigger_contacts,
    set_trigger_mask_deferred,
};
use crate::rng::DeterministicRng;
use crate::types::{PlayerId, Reward, Seed, StepOutcome, Termination};
use crate::verification::reward_and_terminal_postcondition;

const BERRY_COUNT: usize = 6;
const PLAYER_BODY_ID: u16 = 1;
const FIRST_BERRY_BODY_ID: u16 = 10;
const PLATFORMER_BODIES: usize = 1 + BERRY_COUNT;
const PLATFORMER_CONTACTS: usize = PLATFORMER_BODIES * (PLATFORMER_BODIES - 1) / 2;
const ALL_BERRIES_MASK: u8 = 0b00_111111;
const PLATFORMER_Y_SHIFT: u8 = 8;
const PLATFORMER_REMAINING_BERRIES_SHIFT: u8 = 16;
const PLATFORMER_TERMINAL_SHIFT: u8 = 22;
const PLATFORMER_OBSERVATION_BITS: u8 = PLATFORMER_TERMINAL_SHIFT + 1;
const PLATFORMER_ACTION_ORDER: [PlatformerAction; 4] = [
    PlatformerAction::Stay,
    PlatformerAction::Left,
    PlatformerAction::Right,
    PlatformerAction::Jump,
];

mod world;
use world::berry_views;
pub use world::{BerryView, PlatformerWorldView};

/// Player action in the platformer world.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PlatformerAction {
    /// Keep current horizontal position.
    #[default]
    Stay,
    /// Move left by one tile if possible.
    Left,
    /// Move right by one tile if possible.
    Right,
    /// Jump upward by configured jump delta.
    Jump,
}

/// Parameter set for the deterministic platformer environment.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PlatformerConfig {
    /// Arena width in tile units.
    pub width: u8,
    /// Arena height in tile units.
    pub height: u8,
    /// Player body width in tile units.
    pub player_width: u8,
    /// Player body height in tile units.
    pub player_height: u8,
    /// Vertical displacement applied by `Jump`.
    pub jump_delta: u8,
    /// Shared berry y-coordinate.
    pub berry_y: u8,
    /// Sorted berry x-coordinates.
    pub berry_xs: [u8; BERRY_COUNT],
    /// Numerator for jump-sprain Bernoulli penalty.
    pub sprain_numerator: u64,
    /// Denominator for jump-sprain Bernoulli penalty.
    pub sprain_denominator: u64,
    /// Reward added when collecting one berry.
    pub berry_reward: Reward,
    /// Bonus reward added when all berries are collected.
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
    fn checked_step_reward(self, collected: u8, finished: bool, sprained: bool) -> Option<Reward> {
        let mut reward = i128::from(self.berry_reward) * i128::from(collected);
        if finished {
            reward += i128::from(self.finish_bonus);
        }
        if sprained {
            reward -= 1;
        }
        if reward < i128::from(Reward::MIN) || reward > i128::from(Reward::MAX) {
            return None;
        }
        Some(reward as Reward)
    }

    fn reward_bounds(self) -> Option<(Reward, Reward)> {
        let mut min_reward = Reward::MAX;
        let mut max_reward = Reward::MIN;
        let mut collected = 0u8;
        while collected <= BERRY_COUNT as u8 {
            for finished in [false, true] {
                if finished && collected == 0 {
                    continue;
                }
                for sprained in [false, true] {
                    let reward = self.checked_step_reward(collected, finished, sprained)?;
                    min_reward = min_reward.min(reward);
                    max_reward = max_reward.max(reward);
                }
            }
            collected += 1;
        }
        Some((min_reward, max_reward))
    }

    fn compact_spec(self) -> Option<CompactSpec> {
        let (min_reward, max_reward) = self.reward_bounds()?;
        let reward_span = i128::from(max_reward) - i128::from(min_reward);
        if reward_span < 0 || reward_span > i128::from(u64::MAX) {
            return None;
        }
        let reward_offset = -i128::from(min_reward);
        if reward_offset < i128::from(Reward::MIN) || reward_offset > i128::from(Reward::MAX) {
            return None;
        }

        let reward_bits = if reward_span == 0 {
            1
        } else {
            (u64::BITS - (reward_span as u64).leading_zeros()) as u8
        };

        Some(CompactSpec {
            action_count: 4,
            observation_bits: PLATFORMER_OBSERVATION_BITS,
            observation_stream_len: 1,
            reward_bits,
            min_reward,
            max_reward,
            reward_offset: reward_offset as Reward,
        })
    }

    /// Returns the axis-aligned world bounds.
    pub fn arena_bounds(self) -> Aabb2<StrictF64> {
        Aabb2::new(
            Vec2::new(StrictF64::new(0.0), StrictF64::new(0.0)),
            Vec2::new(
                StrictF64::new(self.width as f64),
                StrictF64::new(self.height as f64),
            ),
        )
    }

    /// Returns player half-extents used by physics body creation.
    pub fn player_half_extents(self) -> Vec2<StrictF64> {
        Vec2::new(
            StrictF64::new(self.player_width as f64 / 2.0),
            StrictF64::new(self.player_height as f64 / 2.0),
        )
    }

    /// Converts tile coordinates to player-center world coordinates.
    pub fn player_center(self, x: u8, y: u8) -> Vec2<StrictF64> {
        Vec2::new(
            StrictF64::new(x as f64 + self.player_width as f64 / 2.0),
            StrictF64::new(y as f64 + self.player_height as f64 / 2.0),
        )
    }

    /// Returns center position for berry `index`.
    pub fn berry_center(self, index: usize) -> Vec2<StrictF64> {
        Vec2::new(
            StrictF64::new(self.berry_xs[index] as f64 + 0.5),
            StrictF64::new(self.berry_y as f64),
        )
    }

    /// Validates internal consistency and geometric constraints.
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
            || self.compact_spec().is_none()
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

/// Full platformer state.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PlatformerState {
    /// Active immutable configuration for this episode.
    pub config: PlatformerConfig,
    /// Physics simulation world containing player and berries.
    pub world: PhysicsWorld2d<PLATFORMER_BODIES, PLATFORMER_CONTACTS>,
    /// Bitset of still-active berries.
    pub remaining_berries: u8,
}

/// Canonical player/spectator observation.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct PlatformerObservation {
    /// Player x tile coordinate.
    pub x: u8,
    /// Player y tile coordinate.
    pub y: u8,
    /// Bitset of still-active berries.
    pub remaining_berries: u8,
    /// True when all berries have been collected.
    pub terminal: bool,
    /// Winner id in terminal states.
    pub winner: Option<PlayerId>,
}

impl Default for PlatformerState {
    fn default() -> Self {
        let game = Platformer::default();
        let params = <Platformer as single_player::SinglePlayerGame>::default_params(&game);
        <Platformer as single_player::SinglePlayerGame>::init_with_params(&game, 0, &params)
    }
}

/// Builtin deterministic platformer environment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Platformer {
    /// Environment configuration.
    pub config: PlatformerConfig,
}

impl Platformer {
    /// Creates a platformer game with validated configuration.
    pub fn new(config: PlatformerConfig) -> Self {
        assert!(config.invariant(), "invalid platformer config");
        Self { config }
    }

    fn player_body(state: &PlatformerState) -> &PhysicsBody2d {
        state.world.require_body(PLAYER_BODY_ID)
    }

    fn player_position(state: &PlatformerState) -> (u8, u8) {
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
        set_trigger_mask_deferred(
            &mut state.world,
            FIRST_BERRY_BODY_ID,
            BERRY_COUNT,
            u64::from(state.remaining_berries),
        );
    }

    fn collect_berries_from_contacts(state: &mut PlatformerState) -> (u8, bool) {
        let was_non_terminal = state.remaining_berries != 0;
        let mut remaining = u64::from(state.remaining_berries);
        let collected = collect_actor_trigger_contacts(
            &mut state.world,
            PLAYER_BODY_ID,
            FIRST_BERRY_BODY_ID,
            BERRY_COUNT,
            &mut remaining,
        );
        state.remaining_berries = remaining as u8;
        (collected, was_non_terminal && state.remaining_berries == 0)
    }

    fn observation_from_state(state: &PlatformerState) -> PlatformerObservation {
        let (x, y) = Self::player_position(state);
        PlatformerObservation {
            x,
            y,
            remaining_berries: state.remaining_berries,
            terminal: Self::is_terminal_state(state),
            winner: Self::winner(state),
        }
    }

    fn build_world(
        config: PlatformerConfig,
    ) -> PhysicsWorld2d<PLATFORMER_BODIES, PLATFORMER_CONTACTS> {
        let mut world = PhysicsWorld2d::new(config.arena_bounds());
        world.add_body_deferred(PhysicsBody2d {
            id: PLAYER_BODY_ID,
            kind: BodyKind::Kinematic,
            position: config.player_center(0, 0),
            half_extents: config.player_half_extents(),
            active: true,
        });
        for index in 0..BERRY_COUNT {
            world.add_body_deferred(PhysicsBody2d {
                id: FIRST_BERRY_BODY_ID + index as u16,
                kind: BodyKind::Trigger,
                position: config.berry_center(index),
                half_extents: Vec2::new(StrictF64::new(0.0), StrictF64::new(0.0)),
                active: true,
            });
        }
        world.refresh_contacts();
        world
    }
}

impl single_player::SinglePlayerGame for Platformer {
    type Params = PlatformerConfig;
    type State = PlatformerState;
    type Action = PlatformerAction;
    type Obs = PlatformerObservation;
    type WorldView = PlatformerWorldView;
    type ActionBuf = FixedVec<PlatformerAction, 4>;
    type WordBuf = FixedVec<u64, 1>;

    fn default_params(&self) -> Self::Params {
        self.config
    }

    fn name(&self) -> &'static str {
        "platformer"
    }

    fn params_invariant(&self, params: &Self::Params) -> bool {
        params.invariant()
    }

    fn init_with_params(&self, _seed: Seed, params: &Self::Params) -> Self::State {
        assert!(params.invariant());
        PlatformerState {
            config: *params,
            world: Self::build_world(*params),
            remaining_berries: ALL_BERRIES_MASK,
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        Self::is_terminal_state(state)
    }

    fn legal_actions(&self, state: &Self::State, out: &mut Self::ActionBuf) {
        out.clear();
        if self.is_terminal(state) {
            return;
        }
        out.extend_from_slice(&PLATFORMER_ACTION_ORDER).unwrap();
    }

    fn observe_player(&self, state: &Self::State) -> Self::Obs {
        Self::observation_from_state(state)
    }

    fn world_view(&self, state: &Self::State) -> Self::WorldView {
        PlatformerWorldView {
            config: state.config,
            physics: state.world.clone(),
            berries: berry_views(state.config, state.remaining_berries),
        }
    }

    fn step_in_place(
        &self,
        state: &mut Self::State,
        action: Option<Self::Action>,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<SinglePlayerRewardBuf>,
    ) {
        let action = action.unwrap_or(PlatformerAction::Stay);

        if self.is_terminal(state) {
            out.termination = Termination::Terminal {
                winner: Self::winner(state),
            };
            single_player::push_reward(&mut out.rewards, 0);
        } else {
            let config = state.config;
            let (current_x, _) = Self::player_position(state);
            let (x, y, sprained) = match action {
                PlatformerAction::Stay => (current_x, 0, false),
                PlatformerAction::Left => (current_x.saturating_sub(1), 0, false),
                PlatformerAction::Right => (
                    if current_x + config.player_width < config.width {
                        current_x + 1
                    } else {
                        current_x
                    },
                    0,
                    false,
                ),
                PlatformerAction::Jump => {
                    let sprained =
                        rng.gen_bool_ratio(config.sprain_numerator, config.sprain_denominator);
                    (current_x, config.jump_delta, sprained)
                }
            };

            state
                .world
                .set_body_position_deferred(PLAYER_BODY_ID, config.player_center(x, y));
            state.world.refresh_contacts();
            let (collected, finished) = Self::collect_berries_from_contacts(state);
            self.sync_berries(state);
            state.world.step();

            let reward = config
                .checked_step_reward(collected, finished, sprained)
                .expect("validated platformer config produced an out-of-range reward");
            single_player::push_reward(&mut out.rewards, reward);
            out.termination = if self.is_terminal(state) {
                Termination::Terminal {
                    winner: Self::winner(state),
                }
            } else {
                Termination::Ongoing
            };
        }
    }

    fn state_invariant(&self, state: &Self::State) -> bool {
        if !state.config.invariant()
            || state.remaining_berries & !ALL_BERRIES_MASK != 0
            || !state.world.invariant()
            || state.world.bodies.len() != PLATFORMER_BODIES
        {
            return false;
        }

        let player = Self::player_body(state);
        if player.kind != BodyKind::Kinematic
            || !player.active
            || player.half_extents != state.config.player_half_extents()
        {
            return false;
        }

        let (x, y) = Self::player_position(state);
        if x + state.config.player_width > state.config.width || y > state.config.jump_delta {
            return false;
        }

        for index in 0..BERRY_COUNT {
            let berry = state.world.require_body(FIRST_BERRY_BODY_ID + index as u16);
            let expected_active = state.remaining_berries & (1u8 << index) != 0;
            if berry.kind != BodyKind::Trigger
                || berry.position != state.config.berry_center(index)
                || berry.active != expected_active
            {
                return false;
            }
        }

        true
    }

    fn player_observation_invariant(&self, state: &Self::State, observation: &Self::Obs) -> bool {
        observation == &Self::observation_from_state(state)
    }

    fn spectator_observation_invariant(
        &self,
        state: &Self::State,
        observation: &Self::Obs,
    ) -> bool {
        observation == &Self::observation_from_state(state)
    }

    fn world_view_invariant(&self, state: &Self::State, world: &Self::WorldView) -> bool {
        if world.config != state.config || world.physics != state.world {
            return false;
        }

        let mut index = 0usize;
        while index < world.berries.len() {
            let berry = world.berries[index];
            if berry.id != FIRST_BERRY_BODY_ID + index as u16
                || berry.x != state.config.berry_xs[index]
                || berry.y != state.config.berry_y
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
        pre: &Self::State,
        _action: Option<Self::Action>,
        post: &Self::State,
        outcome: &StepOutcome<SinglePlayerRewardBuf>,
    ) -> bool {
        if pre.remaining_berries == 0 {
            return post == pre && outcome.reward_for(0) == 0 && outcome.is_terminal();
        }
        let Some((min_reward, max_reward)) = post.config.reward_bounds() else {
            return false;
        };
        reward_and_terminal_postcondition(
            outcome.reward_for(0),
            min_reward,
            max_reward,
            post.remaining_berries == 0,
            outcome.is_terminal(),
        )
    }

    fn compact_spec(&self) -> CompactSpec {
        self.compact_spec_for_params(&self.config)
    }

    fn compact_spec_for_params(&self, params: &Self::Params) -> CompactSpec {
        params
            .compact_spec()
            .expect("invalid platformer config cannot produce compact spec")
    }

    fn encode_action(&self, action: &Self::Action) -> u64 {
        encode_enum_action(*action, &PLATFORMER_ACTION_ORDER)
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        decode_enum_action(encoded, &PLATFORMER_ACTION_ORDER)
    }

    fn encode_player_observation(&self, observation: &Self::Obs, out: &mut Self::WordBuf) {
        out.clear();
        let packed = u64::from(observation.x)
            | (u64::from(observation.y) << PLATFORMER_Y_SHIFT)
            | (u64::from(observation.remaining_berries) << PLATFORMER_REMAINING_BERRIES_SHIFT)
            | ((observation.terminal as u64) << PLATFORMER_TERMINAL_SHIFT);
        out.push(packed).unwrap();
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs;
