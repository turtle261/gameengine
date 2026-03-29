use crate::types::Seed;

const ZERO_STATE_REPLACEMENT: u64 = 0xCAFEBABEDEADBEEF;
const STREAM_XOR: u64 = 0x9E3779B97F4A7C15;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub const fn new(seed: Seed) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(STREAM_XOR);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeterministicRng {
    root_seed: Seed,
    state: u64,
}

impl DeterministicRng {
    pub fn from_seed(seed: Seed) -> Self {
        Self::from_seed_and_stream(seed, 0)
    }

    pub fn from_seed_and_stream(seed: Seed, stream_id: u64) -> Self {
        let mut mixer = SplitMix64::new(seed ^ stream_id.wrapping_mul(STREAM_XOR));
        let state = sanitize_state(mixer.next_u64());
        Self {
            root_seed: seed,
            state,
        }
    }

    pub const fn root_seed(self) -> Seed {
        self.root_seed
    }

    pub const fn raw_state(self) -> u64 {
        self.state
    }

    pub fn fork(&self, stream_id: u64) -> Self {
        Self::from_seed_and_stream(self.root_seed, stream_id)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn gen_range(&mut self, end: usize) -> usize {
        if end <= 1 {
            return 0;
        }
        let end = end as u64;
        let zone = u64::MAX - u64::MAX % end;
        loop {
            let candidate = self.next_u64();
            if candidate < zone {
                return (candidate % end) as usize;
            }
        }
    }

    pub fn gen_bool_ratio(&mut self, numerator: u64, denominator: u64) -> bool {
        debug_assert!(denominator > 0);
        if numerator == 0 {
            return false;
        }
        if numerator >= denominator {
            return true;
        }
        (self.next_u64() % denominator) < numerator
    }

    pub fn gen_unit_f64(&mut self) -> f64 {
        let value = self.next_u64() >> 11;
        (value as f64) * (1.0 / 9007199254740992.0)
    }

    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for index in (1..slice.len()).rev() {
            let swap_index = self.gen_range(index + 1);
            slice.swap(index, swap_index);
        }
    }
}

fn sanitize_state(state: u64) -> u64 {
    if state == 0 {
        ZERO_STATE_REPLACEMENT
    } else {
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_replays_exactly() {
        let mut left = DeterministicRng::from_seed(7);
        let mut right = DeterministicRng::from_seed(7);
        for _ in 0..128 {
            assert_eq!(left.next_u64(), right.next_u64());
        }
    }

    #[test]
    fn forked_streams_are_stable() {
        let rng = DeterministicRng::from_seed(42);
        let mut a = rng.fork(11);
        let mut b = rng.fork(11);
        let mut c = rng.fork(12);
        assert_eq!(a.next_u64(), b.next_u64());
        assert_ne!(a.next_u64(), c.next_u64());
    }
}
