use crate::hash::iter_hashes;
use crate::set_membership::SetMembership;
use fixedbitset::FixedBitSet;
use std::f64::consts::LN_2;
use std::fmt::{Debug, Formatter};
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct BloomFilter<T, H> {
    bits: FixedBitSet,
    num_hashes: usize,
    build_hasher: H,
    _phantom: PhantomData<T>,
}

impl<T, H> BloomFilter<T, H> {
    pub fn new(num_bits: usize, num_hashes: usize, build_hasher: H) -> Self {
        assert!(num_bits > 0, "num_bits must be > 0");
        assert!(num_hashes > 0, "num_hashes must be > 0");
        Self {
            bits: FixedBitSet::with_capacity(num_bits),
            num_hashes,
            build_hasher,
            _phantom: PhantomData,
        }
    }

    pub fn with_probability(num_items: usize, probability: f64, build_hasher: H) -> Self {
        assert!(num_items > 0, "num_items must be > 0");
        assert!(
            0. < probability && probability < 1.,
            "probability must be in the range (0, 1)"
        );
        let bits = (-1. * num_items as f64 * probability / (LN_2 * LN_2)).ceil() as usize;
        let num_hashes = (-1. * probability / LN_2).ceil() as usize;
        Self::new(bits, num_hashes, build_hasher)
    }

    pub fn bits(&self) -> usize {
        self.bits.len()
    }

    pub fn num_hashes(&self) -> usize {
        self.num_hashes
    }

    pub fn len(&self) -> usize {
        let m = self.bits.len() as f64;
        let k = self.num_hashes as f64;
        let ones = self.bits.count_ones(..) as f64;
        (-m / k * (1. - ones / m).ln()) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    pub fn clear(&mut self) {
        self.bits.clear();
    }
}

impl<T, H> SetMembership<T> for BloomFilter<T, H>
where
    T: Hash,
    H: BuildHasher,
{
    fn contains(&self, item: &T) -> bool {
        iter_hashes(item, &self.build_hasher)
            .take(self.num_hashes)
            .all(|h| self.bits.contains(h as usize % self.bits.len()))
    }

    fn insert(&mut self, item: &T) -> bool {
        !iter_hashes(item, &self.build_hasher)
            .take(self.num_hashes)
            .fold(true, |acc, h| {
                acc & self.bits.put(h as usize % self.bits.len())
            })
    }
}

impl<T, H> Debug for BloomFilter<T, H> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BloomFilter {{ num_bits: {}, num_hashes: {} }}",
            self.bits.len(),
            self.num_hashes
        )
    }
}
