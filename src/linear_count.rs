use fixedbitset::FixedBitSet;
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

pub struct LinearCount<T, H> {
    bits: FixedBitSet,
    zeros: usize,
    build_hasher: H,
    _phantom: PhantomData<T>,
}

impl<T, H> LinearCount<T, H> {
    pub fn new(num_bits: usize, build_hasher: H) -> Self {
        Self {
            bits: FixedBitSet::with_capacity(num_bits),
            zeros: num_bits,
            build_hasher,
            _phantom: PhantomData,
        }
    }
}

impl<T, H> LinearCount<T, H>
where
    T: Hash,
    H: BuildHasher,
{
    pub fn count(&self) -> f64 {
        let m = self.bits.len() as f64;
        if self.zeros > 0 {
            -m * (self.zeros as f64 / m).ln()
        } else {
            m
        }
    }

    pub fn insert(&mut self, item: &T) {
        let hash = self.build_hasher.hash_one(item);
        let index = hash as usize % self.bits.len();
        if !self.bits.put(index) {
            self.zeros += 1;
        }
    }
}
