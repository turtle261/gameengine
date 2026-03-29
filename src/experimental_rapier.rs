use rapier2d_f64::na::Vector2;
use rapier2d_f64::prelude::{ColliderBuilder, ColliderSet, RigidBodyBuilder, RigidBodySet};

use crate::physics::{BodyKind, PhysicsWorld2d};

#[derive(Debug)]
pub struct RapierWorldSnapshot {
    pub rigid_bodies: RigidBodySet,
    pub colliders: ColliderSet,
}

pub fn snapshot_from_world<const BODIES: usize, const CONTACTS: usize>(
    world: &PhysicsWorld2d<BODIES, CONTACTS>,
) -> RapierWorldSnapshot {
    let mut rigid_bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();

    for body in world.bodies.as_slice() {
        if !body.active {
            continue;
        }

        let translation = Vector2::new(body.position.x.to_f64(), body.position.y.to_f64());
        let rigid_body = match body.kind {
            BodyKind::Static | BodyKind::Trigger => {
                RigidBodyBuilder::fixed().translation(translation.into())
            }
            BodyKind::Kinematic => {
                RigidBodyBuilder::kinematic_position_based().translation(translation.into())
            }
        };
        let handle = rigid_bodies.insert(rigid_body.build());

        let collider = if body.kind == BodyKind::Trigger {
            ColliderBuilder::ball(0.0).sensor(true)
        } else {
            ColliderBuilder::cuboid(body.half_extents.x.to_f64(), body.half_extents.y.to_f64())
        };
        colliders.insert_with_parent(collider.build(), handle, &mut rigid_bodies);
    }

    RapierWorldSnapshot {
        rigid_bodies,
        colliders,
    }
}
