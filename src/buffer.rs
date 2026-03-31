use core::fmt;
use core::hash::{Hash, Hasher};
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CapacityError {
    pub capacity: usize,
}

pub trait Buffer {
    type Item;

    const CAPACITY: usize;

    fn clear(&mut self);
    fn len(&self) -> usize;
    fn push(&mut self, item: Self::Item) -> Result<(), CapacityError>;
    fn as_slice(&self) -> &[Self::Item];
    fn as_mut_slice(&mut self) -> &mut [Self::Item];

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

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

#[derive(Clone)]
pub struct FixedVec<T, const N: usize> {
    data: [T; N],
    len: usize,
}

pub(crate) fn default_array<T: Default, const N: usize>() -> [T; N] {
    let mut data = [const { MaybeUninit::<T>::uninit() }; N];
    let mut index = 0usize;
    while index < N {
        data[index].write(T::default());
        index += 1;
    }
    // SAFETY:
    // Every slot in `data` is initialized exactly once in the loop above,
    // and `MaybeUninit<T>` has the same layout as `T`.
    unsafe { (&data as *const [MaybeUninit<T>; N] as *const [T; N]).read() }
}

impl<T, const N: usize> FixedVec<T, N> {
    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn capacity(&self) -> usize {
        N
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.len]
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data[..self.len]
    }

    pub fn first(&self) -> Option<&T> {
        self.as_slice().first()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_slice().get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.as_mut_slice().get_mut(index)
    }

    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.as_slice().iter()
    }
}

impl<T: Default, const N: usize> FixedVec<T, N> {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
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
    pub fn contains(&self, value: &T) -> bool {
        self.as_slice().contains(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BitWords<const N: usize> {
    words: [u64; N],
}

impl<const N: usize> BitWords<N> {
    pub const fn words(&self) -> &[u64; N] {
        &self.words
    }

    pub fn clear_all(&mut self) {
        self.words.fill(0);
    }

    pub fn set_bit(&mut self, bit: usize) {
        let word = bit / 64;
        let offset = bit % 64;
        if word < N {
            self.words[word] |= 1u64 << offset;
        }
    }

    pub fn clear_bit(&mut self, bit: usize) {
        let word = bit / 64;
        let offset = bit % 64;
        if word < N {
            self.words[word] &= !(1u64 << offset);
        }
    }

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
