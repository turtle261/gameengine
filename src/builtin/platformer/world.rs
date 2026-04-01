//! Platformer world/debug view types and physics oracle adapter.

use crate::math::{Aabb2, StrictF64};
use crate::physics::{Contact2d, PhysicsBody2d, PhysicsOracleView2d, PhysicsWorld2d};
use crate::game::Game;

use super::{
    BERRY_COUNT, FIRST_BERRY_BODY_ID, PLATFORMER_BODIES, PLATFORMER_CONTACTS, Platformer,
    PlatformerConfig,
};

/// Render/debug view of one berry.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BerryView {
    /// Stable body id in the physics world.
    pub id: u16,
    /// Berry x tile coordinate.
    pub x: u8,
    /// Berry y tile coordinate.
    pub y: u8,
    /// Whether this berry has already been collected.
    pub collected: bool,
}

/// World-level debug view combining config, physics and berry metadata.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PlatformerWorldView {
    /// Environment configuration used for this world.
    pub config: PlatformerConfig,
    /// Physics snapshot.
    pub physics: PhysicsWorld2d<PLATFORMER_BODIES, PLATFORMER_CONTACTS>,
    /// Berry metadata for rendering and inspection.
    pub berries: [BerryView; BERRY_COUNT],
}

pub(super) fn berry_views(config: PlatformerConfig, remaining_berries: u8) -> [BerryView; BERRY_COUNT] {
    let mut berries = [BerryView::default(); BERRY_COUNT];
    let mut index = 0usize;
    while index < BERRY_COUNT {
        berries[index] = BerryView {
            id: FIRST_BERRY_BODY_ID + index as u16,
            x: config.berry_xs[index],
            y: config.berry_y,
            collected: (remaining_berries & (1u8 << index)) == 0,
        };
        index += 1;
    }
    berries
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