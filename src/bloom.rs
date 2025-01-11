use fixedbitset::FixedBitSet;
use std::f64::consts::LN_2;
use std::fmt::{Debug, Formatter};
use std::hash::{BuildHasher, Hash, Hasher};
use std::iter::{once, successors};
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

    pub fn from_probability(num_items: usize, probability: f64, build_hasher: H) -> Self {
        assert!(num_items > 0, "num_items must be > 0");
        assert!(
            0. < probability && probability < 1.,
            "probability must be in range (0, 1)"
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

    pub fn clear(&mut self) {
        self.bits.clear();
    }
}

impl<T, H> BloomFilter<T, H>
where
    T: Hash,
    H: BuildHasher,
{
    pub fn contains(&self, item: &T) -> bool {
        self.iter_hashes(item).all(|bit| self.bits.contains(bit))
    }

    pub fn insert(&mut self, item: &T) -> bool {
        !self
            .iter_hashes(item)
            .fold(true, |acc, bit| acc & self.bits.put(bit))
    }

    fn iter_hashes(&self, item: &T) -> impl Iterator<Item = usize> {
        let h1 = self.base_hash(item, 1);
        let h2 = self.base_hash(item, 2);
        let num_hashes = self.num_hashes as u64;

        once(h1)
            .chain(once(h2))
            .chain(
                successors(Some((h1, h2, 3u64)), |(a, b, i): &(u64, u64, u64)| {
                    Some((*b, a.wrapping_add(b.wrapping_mul(*i)), i + 1))
                })
                .skip(1)
                .map(|(_, h, _)| h),
            )
            .map(move |h| (h % num_hashes) as usize)
            .take(self.num_hashes)
    }

    fn base_hash(&self, item: &T, i: usize) -> u64 {
        let mut hasher = self.build_hasher.build_hasher();
        hasher.write_usize(i);
        item.hash(&mut hasher);
        hasher.finish()
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
