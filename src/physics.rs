use crate::buffer::FixedVec;
use crate::math::{Aabb2, StrictF64, Vec2};
use crate::types::Tick;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BodyKind {
    #[default]
    Static,
    Kinematic,
    Trigger,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct PhysicsBody2d {
    pub id: u16,
    pub kind: BodyKind,
    pub position: Vec2<StrictF64>,
    pub half_extents: Vec2<StrictF64>,
    pub active: bool,
}

impl PhysicsBody2d {
    pub fn aabb(&self) -> Aabb2<StrictF64> {
        Aabb2::new(
            self.position - self.half_extents,
            self.position + self.half_extents,
        )
    }

    pub fn invariant(&self) -> bool {
        self.position.x.is_finite()
            && self.position.y.is_finite()
            && self.half_extents.x.is_finite()
            && self.half_extents.y.is_finite()
            && self.half_extents.x.to_f64() >= 0.0
            && self.half_extents.y.to_f64() >= 0.0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Contact2d {
    pub a: u16,
    pub b: u16,
}

pub trait PhysicsOracleView2d {
    fn bounds(&self) -> Aabb2<StrictF64>;
    fn tick(&self) -> Tick;
    fn bodies(&self) -> &[PhysicsBody2d];
    fn contacts(&self) -> &[Contact2d];
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PhysicsWorld2d<const BODIES: usize, const CONTACTS: usize> {
    pub bounds: Aabb2<StrictF64>,
    pub bodies: FixedVec<PhysicsBody2d, BODIES>,
    pub contacts: FixedVec<Contact2d, CONTACTS>,
    pub tick: Tick,
}

impl<const BODIES: usize, const CONTACTS: usize> PhysicsWorld2d<BODIES, CONTACTS> {
    pub fn new(bounds: Aabb2<StrictF64>) -> Self {
        Self {
            bounds,
            bodies: FixedVec::default(),
            contacts: FixedVec::default(),
            tick: 0,
        }
    }

    pub fn invariant(&self) -> bool {
        if !self.bounds.min.x.is_finite()
            || !self.bounds.min.y.is_finite()
            || !self.bounds.max.x.is_finite()
            || !self.bounds.max.y.is_finite()
            || self.bounds.min.x > self.bounds.max.x
            || self.bounds.min.y > self.bounds.max.y
        {
            return false;
        }

        let bodies = self.bodies.as_slice();
        let mut index = 0usize;
        while index < bodies.len() {
            let body = &bodies[index];
            if !body.invariant() {
                return false;
            }
            if index > 0 && bodies[index - 1].id >= body.id {
                return false;
            }
            let aabb = body.aabb();
            if aabb.min.x < self.bounds.min.x
                || aabb.max.x > self.bounds.max.x
                || aabb.min.y < self.bounds.min.y
                || aabb.max.y > self.bounds.max.y
            {
                return false;
            }
            index += 1;
        }

        let contacts = self.contacts.as_slice();
        let mut contact_index = 0usize;
        while contact_index < contacts.len() {
            let contact = contacts[contact_index];
            if contact.a >= contact.b {
                return false;
            }
            contact_index += 1;
        }
        true
    }

    pub fn add_body(&mut self, body: PhysicsBody2d) {
        self.add_body_deferred(body);
        self.refresh_contacts();
    }

    pub(crate) fn add_body_deferred(&mut self, body: PhysicsBody2d) {
        assert!(body.invariant());
        if let Some(last) = self.bodies.as_slice().last() {
            assert!(
                last.id < body.id,
                "physics bodies must be inserted in ascending id order"
            );
        }
        self.bodies
            .push(body)
            .expect("physics body capacity exceeded");
        self.clamp_body(body.id);
    }

    pub fn body(&self, id: u16) -> Option<&PhysicsBody2d> {
        let bodies = self.bodies.as_slice();
        let mut index = 0usize;
        while index < bodies.len() {
            let body = &bodies[index];
            if body.id == id {
                return Some(body);
            }
            index += 1;
        }
        None
    }

    pub fn require_body(&self, id: u16) -> &PhysicsBody2d {
        self.body(id).expect("missing physics body")
    }

    pub fn body_mut(&mut self, id: u16) -> Option<&mut PhysicsBody2d> {
        let bodies = self.bodies.as_mut_slice();
        let mut index = 0usize;
        while index < bodies.len() {
            if bodies[index].id == id {
                return Some(&mut bodies[index]);
            }
            index += 1;
        }
        None
    }

    pub fn set_body_active(&mut self, id: u16, active: bool) {
        self.set_body_active_deferred(id, active);
        self.refresh_contacts();
    }

    pub(crate) fn set_body_active_deferred(&mut self, id: u16, active: bool) {
        if let Some(body) = self.body_mut(id) {
            body.active = active;
        }
    }

    pub fn set_body_position(&mut self, id: u16, position: Vec2<StrictF64>) {
        self.set_body_position_deferred(id, position);
        self.refresh_contacts();
    }

    pub(crate) fn set_body_position_deferred(&mut self, id: u16, position: Vec2<StrictF64>) {
        if let Some(body) = self.body_mut(id) {
            body.position = position;
        }
        self.clamp_body(id);
    }

    pub fn translate_body(&mut self, id: u16, delta: Vec2<StrictF64>) {
        self.translate_body_deferred(id, delta);
        self.refresh_contacts();
    }

    pub(crate) fn translate_body_deferred(&mut self, id: u16, delta: Vec2<StrictF64>) {
        if let Some(body) = self.body_mut(id) {
            body.position += delta;
        }
        self.clamp_body(id);
    }

    pub fn step(&mut self) {
        self.tick += 1;
        self.refresh_contacts();
    }

    pub fn has_contact(&self, a: u16, b: u16) -> bool {
        let (left, right) = if a <= b { (a, b) } else { (b, a) };
        let contacts = self.contacts.as_slice();
        let mut index = 0usize;
        while index < contacts.len() {
            let contact = contacts[index];
            if contact.a == left && contact.b == right {
                return true;
            }
            index += 1;
        }
        false
    }

    fn clamp_body(&mut self, id: u16) {
        let bounds = self.bounds;
        if let Some(body) = self.body_mut(id) {
            let min_x = bounds.min.x + body.half_extents.x;
            let max_x = bounds.max.x - body.half_extents.x;
            let min_y = bounds.min.y + body.half_extents.y;
            let max_y = bounds.max.y - body.half_extents.y;
            body.position.x = body.position.x.clamp(min_x, max_x);
            body.position.y = body.position.y.clamp(min_y, max_y);
        }
    }

    pub(crate) fn refresh_contacts(&mut self) {
        self.contacts.clear();
        let bodies = self.bodies.as_slice();
        for left in 0..bodies.len() {
            for right in (left + 1)..bodies.len() {
                if !bodies[left].active || !bodies[right].active {
                    continue;
                }
                if intersects(bodies[left].aabb(), bodies[right].aabb()) {
                    self.contacts
                        .push(Contact2d {
                            a: bodies[left].id,
                            b: bodies[right].id,
                        })
                        .expect("physics contact capacity exceeded");
                }
            }
        }
    }
}

impl<const BODIES: usize, const CONTACTS: usize> PhysicsOracleView2d
    for PhysicsWorld2d<BODIES, CONTACTS>
{
    fn bounds(&self) -> Aabb2<StrictF64> {
        self.bounds
    }

    fn tick(&self) -> Tick {
        self.tick
    }

    fn bodies(&self) -> &[PhysicsBody2d] {
        self.bodies.as_slice()
    }

    fn contacts(&self) -> &[Contact2d] {
        self.contacts.as_slice()
    }
}

fn intersects(left: Aabb2<StrictF64>, right: Aabb2<StrictF64>) -> bool {
    left.min.x <= right.max.x
        && left.max.x >= right.min.x
        && left.min.y <= right.max.y
        && left.max.y >= right.min.y
}

#[cfg(kani)]
mod proofs {
    use super::{BodyKind, PhysicsBody2d, PhysicsOracleView2d, PhysicsWorld2d};
    use crate::math::{Aabb2, StrictF64, Vec2};

    #[kani::proof]
    #[kani::unwind(8)]
    fn clamping_keeps_body_in_bounds() {
        let bounds = Aabb2::new(
            Vec2::new(StrictF64::new(0.0), StrictF64::new(0.0)),
            Vec2::new(StrictF64::new(12.0), StrictF64::new(3.0)),
        );
        let mut world = PhysicsWorld2d::<1, 0>::new(bounds);
        world.add_body(PhysicsBody2d {
            id: 1,
            kind: BodyKind::Kinematic,
            position: Vec2::new(StrictF64::new(50.0), StrictF64::new(-10.0)),
            half_extents: Vec2::new(StrictF64::new(0.5), StrictF64::new(0.5)),
            active: true,
        });
        assert!(world.invariant());
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn oracle_view_matches_world_storage() {
        let bounds = Aabb2::new(
            Vec2::new(StrictF64::new(0.0), StrictF64::new(0.0)),
            Vec2::new(StrictF64::new(12.0), StrictF64::new(3.0)),
        );
        let mut world = PhysicsWorld2d::<3, 3>::new(bounds);
        world.add_body(PhysicsBody2d {
            id: 1,
            kind: BodyKind::Kinematic,
            position: Vec2::new(StrictF64::new(1.0), StrictF64::new(1.0)),
            half_extents: Vec2::new(StrictF64::new(0.5), StrictF64::new(0.5)),
            active: true,
        });
        world.add_body(PhysicsBody2d {
            id: 2,
            kind: BodyKind::Trigger,
            position: Vec2::new(StrictF64::new(1.0), StrictF64::new(1.0)),
            half_extents: Vec2::new(StrictF64::new(0.0), StrictF64::new(0.0)),
            active: true,
        });
        assert_eq!(world.bounds(), bounds);
        assert_eq!(world.tick(), 0);
        assert_eq!(world.bodies(), world.bodies.as_slice());
        assert_eq!(world.contacts(), world.contacts.as_slice());
    }
}

#[cfg(test)]
mod tests {
    use super::{BodyKind, Contact2d, PhysicsBody2d, PhysicsOracleView2d, PhysicsWorld2d};
    use crate::math::{Aabb2, StrictF64, Vec2};

    fn sample_body(id: u16, x: f64, y: f64) -> PhysicsBody2d {
        PhysicsBody2d {
            id,
            kind: BodyKind::Kinematic,
            position: Vec2::new(StrictF64::new(x), StrictF64::new(y)),
            half_extents: Vec2::new(StrictF64::new(0.5), StrictF64::new(0.5)),
            active: true,
        }
    }

    #[test]
    fn refresh_contacts_orders_and_filters_pairs() {
        let bounds = Aabb2::new(
            Vec2::new(StrictF64::new(0.0), StrictF64::new(0.0)),
            Vec2::new(StrictF64::new(8.0), StrictF64::new(8.0)),
        );
        let mut world = PhysicsWorld2d::<4, 6>::new(bounds);
        world.add_body(sample_body(1, 1.0, 1.0));
        world.add_body(sample_body(2, 1.4, 1.0));
        world.add_body(sample_body(3, 4.0, 4.0));
        world.add_body(sample_body(4, 4.4, 4.0));
        world.set_body_active(4, false);
        assert_eq!(world.contacts.as_slice(), &[Contact2d { a: 1, b: 2 }]);
    }

    #[test]
    fn oracle_view_exposes_world_without_allocation() {
        let bounds = Aabb2::new(
            Vec2::new(StrictF64::new(0.0), StrictF64::new(0.0)),
            Vec2::new(StrictF64::new(6.0), StrictF64::new(6.0)),
        );
        let mut world = PhysicsWorld2d::<2, 1>::new(bounds);
        world.add_body(sample_body(1, 1.0, 1.0));
        world.add_body(sample_body(2, 1.4, 1.0));
        world.step();

        assert_eq!(PhysicsOracleView2d::bounds(&world), bounds);
        assert_eq!(PhysicsOracleView2d::tick(&world), 1);
        assert_eq!(PhysicsOracleView2d::bodies(&world).len(), 2);
        assert_eq!(
            PhysicsOracleView2d::contacts(&world),
            &[Contact2d { a: 1, b: 2 }]
        );
    }
}
