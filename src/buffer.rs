//! Fixed-capacity buffer utilities used to avoid heap allocations in core loops.

use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Deref, DerefMut};

/// Error returned when attempting to push past fixed capacity.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CapacityError {
    /// Maximum capacity of the destination buffer.
    pub capacity: usize,
}

/// Minimal fixed-capacity buffer interface.
pub trait Buffer {
    /// Item stored by this buffer.
    type Item;

    /// Maximum number of items this buffer can hold.
    const CAPACITY: usize;

    /// Removes all items from the buffer.
    fn clear(&mut self);
    /// Returns the current number of items.
    fn len(&self) -> usize;
    /// Appends one item when capacity permits.
    fn push(&mut self, item: Self::Item) -> Result<(), CapacityError>;
    /// Returns the populated immutable slice.
    fn as_slice(&self) -> &[Self::Item];
    /// Returns the populated mutable slice.
    fn as_mut_slice(&mut self) -> &mut [Self::Item];

    /// Returns whether the buffer has zero items.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Extends the buffer by cloning all items from `items`.
    fn extend_from_slice(&mut self, items: &[Self::Item]) -> Result<(), CapacityError>
    where
        Self::Item: Clone,
    {
        for item in items {
            self.push(item.clone())?;
        }
        Ok(())
    }
}

/// Array-backed fixed-capacity vector.
#[derive(Clone)]
pub struct FixedVec<T, const N: usize> {
    data: [T; N],
    len: usize,
}

pub(crate) fn default_array<T: Default, const N: usize>() -> [T; N] {
    core::array::from_fn(|_| T::default())
}

impl<T, const N: usize> FixedVec<T, N> {
    /// Clears all elements.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns current length.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns compile-time capacity.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Returns `true` when `len == 0`.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the populated immutable slice.
    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.len]
    }

    /// Returns the populated mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data[..self.len]
    }

    /// Returns the first element when present.
    pub fn first(&self) -> Option<&T> {
        self.as_slice().first()
    }

    /// Returns an immutable element reference by index.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_slice().get(index)
    }

    /// Returns a mutable element reference by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.as_mut_slice().get_mut(index)
    }

    /// Iterates over populated elements.
    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.as_slice().iter()
    }
}

impl<T: Default, const N: usize> FixedVec<T, N> {
    /// Creates an empty fixed-capacity vector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes one element when capacity permits.
    pub fn push(&mut self, item: T) -> Result<(), CapacityError> {
        if self.len == N {
            return Err(CapacityError { capacity: N });
        }
        self.data[self.len] = item;
        self.len += 1;
        Ok(())
    }
}

impl<T: Default, const N: usize> Default for FixedVec<T, N> {
    fn default() -> Self {
        Self {
            data: default_array(),
            len: 0,
        }
    }
}

impl<T: Default, const N: usize> Buffer for FixedVec<T, N> {
    type Item = T;

    const CAPACITY: usize = N;

    fn clear(&mut self) {
        self.clear();
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn push(&mut self, item: Self::Item) -> Result<(), CapacityError> {
        self.push(item)
    }

    fn as_slice(&self) -> &[Self::Item] {
        self.as_slice()
    }

    fn as_mut_slice(&mut self) -> &mut [Self::Item] {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> Deref for FixedVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> DerefMut for FixedVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for FixedVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

impl<T: PartialEq, const N: usize> PartialEq for FixedVec<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq, const N: usize> Eq for FixedVec<T, N> {}

impl<T: Hash, const N: usize> Hash for FixedVec<T, N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len.hash(state);
        self.as_slice().hash(state);
    }
}

impl<T: PartialEq, const N: usize> FixedVec<T, N> {
    /// Returns whether `value` exists in the populated slice.
    pub fn contains(&self, value: &T) -> bool {
        self.as_slice().contains(value)
    }
}

/// Fixed-size bitset backed by `N` machine words.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BitWords<const N: usize> {
    words: [u64; N],
}

impl<const N: usize> BitWords<N> {
    /// Returns immutable access to backing words.
    pub const fn words(&self) -> &[u64; N] {
        &self.words
    }

    /// Clears all bits.
    pub fn clear_all(&mut self) {
        self.words.fill(0);
    }

    /// Sets `bit` when it falls within capacity.
    pub fn set_bit(&mut self, bit: usize) {
        let word = bit / 64;
        let offset = bit % 64;
        if word < N {
            self.words[word] |= 1u64 << offset;
        }
    }

    /// Clears `bit` when it falls within capacity.
    pub fn clear_bit(&mut self, bit: usize) {
        let word = bit / 64;
        let offset = bit % 64;
        if word < N {
            self.words[word] &= !(1u64 << offset);
        }
    }

    /// Tests whether `bit` is set.
    pub fn test_bit(&self, bit: usize) -> bool {
        let word = bit / 64;
        let offset = bit % 64;
        word < N && (self.words[word] & (1u64 << offset)) != 0
    }
}

impl<const N: usize> Default for BitWords<N> {
    fn default() -> Self {
        Self { words: [0; N] }
    }
}

#[cfg(kani)]
mod proofs {
    use super::{BitWords, FixedVec};

    #[kani::proof]
    fn fixed_vec_push_preserves_prefix_order() {
        let a: u8 = kani::any();
        let b: u8 = kani::any();
        let mut vec = FixedVec::<u8, 2>::default();
        assert!(vec.push(a).is_ok());
        assert!(vec.push(b).is_ok());
        assert_eq!(vec.as_slice(), &[a, b]);
        assert!(vec.push(0).is_err());
    }

    #[kani::proof]
    fn bit_words_round_trip() {
        let bit: usize = kani::any();
        kani::assume(bit < 128);
        let mut words = BitWords::<2>::default();
        words.set_bit(bit);
        assert!(words.test_bit(bit));
        words.clear_bit(bit);
        assert!(!words.test_bit(bit));
    }
}
