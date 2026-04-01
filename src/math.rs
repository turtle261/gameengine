//! Deterministic math primitives used by simulation and rendering layers.

use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

/// 2D vector.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Vec2<T> {
    /// X coordinate.
    pub x: T,
    /// Y coordinate.
    pub y: T,
}

impl<T> Vec2<T> {
    /// Creates a 2D vector.
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T> Add for Vec2<T>
where
    T: Add<Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<T> AddAssign for Vec2<T>
where
    T: AddAssign,
{
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T> Sub for Vec2<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<T> SubAssign for Vec2<T>
where
    T: SubAssign,
{
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

/// 3D vector.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Vec3<T> {
    /// X coordinate.
    pub x: T,
    /// Y coordinate.
    pub y: T,
    /// Z coordinate.
    pub z: T,
}

impl<T> Vec3<T> {
    /// Creates a 3D vector.
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

impl<T> Add for Vec3<T>
where
    T: Add<Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<T> Sub for Vec3<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

/// Axis-aligned bounding box in 2D.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Aabb2<T> {
    /// Minimum corner.
    pub min: Vec2<T>,
    /// Maximum corner.
    pub max: Vec2<T>,
}

impl<T> Aabb2<T>
where
    T: Copy + Ord,
{
    /// Creates a 2D AABB.
    pub const fn new(min: Vec2<T>, max: Vec2<T>) -> Self {
        Self { min, max }
    }

    /// Returns whether `point` is inside or on bounds.
    pub fn contains(&self, point: Vec2<T>) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

            /// Returns whether this AABB intersects `other`.
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
}

/// Axis-aligned bounding box in 3D.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Aabb3<T> {
    /// Minimum corner.
    pub min: Vec3<T>,
    /// Maximum corner.
    pub max: Vec3<T>,
}

impl<T> Aabb3<T>
where
    T: Copy + Ord,
{
    /// Creates a 3D AABB.
    pub const fn new(min: Vec3<T>, max: Vec3<T>) -> Self {
        Self { min, max }
    }

    /// Returns whether `point` is inside or on bounds.
    pub fn contains(&self, point: Vec3<T>) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

            /// Returns whether this AABB intersects `other`.
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }
}

/// Fixed-point numeric wrapper with `FRACTION_BITS` fractional bits.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Fixed<const FRACTION_BITS: u32> {
    raw: i64,
}

impl<const FRACTION_BITS: u32> Fixed<FRACTION_BITS> {
    /// Creates a fixed-point value from raw representation.
    pub const fn from_raw(raw: i64) -> Self {
        Self { raw }
    }

    /// Creates a fixed-point value from integer input.
    pub const fn from_int(value: i64) -> Self {
        Self {
            raw: value << FRACTION_BITS,
        }
    }

    /// Returns raw fixed-point representation.
    pub const fn raw(self) -> i64 {
        self.raw
    }

    /// Floors value toward negative infinity and returns integer part.
    pub const fn floor_to_int(self) -> i64 {
        self.raw >> FRACTION_BITS
    }

    /// Converts to `f64`.
    pub fn to_f64(self) -> f64 {
        self.raw as f64 / ((1u64 << FRACTION_BITS) as f64)
    }
}

impl<const FRACTION_BITS: u32> Add for Fixed<FRACTION_BITS> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from_raw(self.raw + rhs.raw)
    }
}

impl<const FRACTION_BITS: u32> Sub for Fixed<FRACTION_BITS> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::from_raw(self.raw - rhs.raw)
    }
}

impl<const FRACTION_BITS: u32> Mul for Fixed<FRACTION_BITS> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let value = ((self.raw as i128) * (rhs.raw as i128)) >> FRACTION_BITS;
        Self::from_raw(value as i64)
    }
}

impl<const FRACTION_BITS: u32> Div for Fixed<FRACTION_BITS> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let value = ((self.raw as i128) << FRACTION_BITS) / (rhs.raw as i128);
        Self::from_raw(value as i64)
    }
}

/// `f32` wrapper with deterministic bitwise equality/hash semantics.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StrictF32 {
    bits: u32,
}

impl StrictF32 {
    /// Creates from raw IEEE-754 bits.
    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    /// Creates from floating value by preserving raw bits.
    pub fn new(value: f32) -> Self {
        Self {
            bits: value.to_bits(),
        }
    }

    /// Returns raw IEEE-754 bits.
    pub const fn to_bits(self) -> u32 {
        self.bits
    }

    /// Converts to `f32`.
    pub fn to_f32(self) -> f32 {
        f32::from_bits(self.bits)
    }
}

/// `f64` wrapper with deterministic total ordering and bitwise equality.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct StrictF64 {
    bits: u64,
}

impl StrictF64 {
    /// Creates from raw IEEE-754 bits.
    pub const fn from_bits(bits: u64) -> Self {
        Self { bits }
    }

    /// Creates from floating value by preserving raw bits.
    pub fn new(value: f64) -> Self {
        Self {
            bits: value.to_bits(),
        }
    }

    /// Returns raw IEEE-754 bits.
    pub const fn to_bits(self) -> u64 {
        self.bits
    }

    /// Converts to `f64`.
    pub fn to_f64(self) -> f64 {
        f64::from_bits(self.bits)
    }

    /// Returns whether value is finite.
    pub fn is_finite(self) -> bool {
        self.to_f64().is_finite()
    }

    /// Clamps this value to `[min, max]`.
    pub fn clamp(self, min: Self, max: Self) -> Self {
        let value = self.to_f64().clamp(min.to_f64(), max.to_f64());
        Self::new(value)
    }
}

impl Add for StrictF32 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.to_f32() + rhs.to_f32())
    }
}

impl Sub for StrictF32 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.to_f32() - rhs.to_f32())
    }
}

impl Mul for StrictF32 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.to_f32() * rhs.to_f32())
    }
}

impl Div for StrictF32 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.to_f32() / rhs.to_f32())
    }
}

impl PartialOrd for StrictF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StrictF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_f64().total_cmp(&other.to_f64())
    }
}

impl Add for StrictF64 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.to_f64() + rhs.to_f64())
    }
}

impl AddAssign for StrictF64 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for StrictF64 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.to_f64() - rhs.to_f64())
    }
}

impl SubAssign for StrictF64 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for StrictF64 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.to_f64() * rhs.to_f64())
    }
}

impl Div for StrictF64 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.to_f64() / rhs.to_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_math_is_exact_for_small_values() {
        type F = Fixed<8>;
        let a = F::from_int(3);
        let b = F::from_int(2);
        assert_eq!((a + b).floor_to_int(), 5);
        assert_eq!((a - b).floor_to_int(), 1);
        assert_eq!((a * b).floor_to_int(), 6);
    }
}
