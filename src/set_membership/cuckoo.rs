use crate::bit_vec::BitVec;
use crate::set_membership::SetMembership;
use rand::Rng;
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

const MAX_EVICTIONS: u32 = 500;

pub struct CuckooFilter<T, const FINGERPRINT_SIZE: usize, H, R> {
    table: BitVec<u32, FINGERPRINT_SIZE>,
    num_buckets: usize,
    bucket_size: usize,
    build_hasher: H,
    rng: R,
    _phantom: PhantomData<T>,
}

impl<T, const FINGERPRINT_SIZE: usize, H, R> CuckooFilter<T, FINGERPRINT_SIZE, H, R> {
    pub fn new(num_buckets: usize, bucket_size: usize, build_hasher: H, rng: R) -> Self {
        assert!(num_buckets > 1, "num_buckets must be > 1");
        assert!(
            num_buckets.is_power_of_two(),
            "num_buckets must be a power of two"
        );
        assert!(bucket_size > 0, "bucket_size must be > 0");

        Self {
            table: BitVec::<u32, FINGERPRINT_SIZE>::new(num_buckets * bucket_size),
            num_buckets,
            bucket_size,
            build_hasher,
            rng,
            _phantom: PhantomData,
        }
    }
}

impl<T, const FINGERPRINT_SIZE: usize, H, R> CuckooFilter<T, FINGERPRINT_SIZE, H, R>
where
    T: Hash,
    H: BuildHasher,
    R: Rng,
{
    fn index_and_tag(&self, item: &T) -> (usize, u32) {
        let hash = self.build_hasher.hash_one(item);
        let index = (hash >> 32) as usize & (self.num_buckets - 1);
        let tag = hash as u32 & ((1 << FINGERPRINT_SIZE) - 1);
        (index, tag + (tag == 0) as u32)
    }

    fn alt_index(&self, index: usize, tag: u32) -> usize {
        // Quick-n-dirty way from the original implementation,
        // i.e. multiply by the hash constant from MurmurHash2.
        (index ^ (tag as usize).wrapping_mul(0x5bd1e995)) & (self.num_buckets - 1)
    }

    fn contains_hashed(&self, i1: usize, i2: usize, tag: u32) -> bool {
        [i1, i2].iter().any(|&index| {
            (0..self.bucket_size)
                .map(|entry| index * self.bucket_size + entry)
                .any(|address| self.table.get(address) == tag)
        })
    }

    fn try_insert(&mut self, index: usize, tag: u32) -> Result<(), ()> {
        (0..self.bucket_size)
            .map(|entry| index * self.bucket_size + entry)
            .find(|&address| self.table.get(address) == 0)
            .inspect(|&address| self.table.set(address, tag))
            .map(|_| ())
            .ok_or(())
    }

    fn maybe_evict_and_insert(&mut self, index: usize, tag: u32) -> Option<u32> {
        if self.try_insert(index, tag).is_ok() {
            return None;
        }

        let random_entry = self.rng.gen::<usize>() % self.bucket_size;
        let address = index * self.bucket_size + random_entry;
        let old = self.table.get(address);

        debug_assert_ne!(old, 0, "evicted entry was 0");
        self.table.set(address, tag);

        Some(old)
    }
}

impl<T, const FINGERPRINT_SIZE: usize, H, R> SetMembership<T>
    for CuckooFilter<T, FINGERPRINT_SIZE, H, R>
where
    T: Hash,
    H: BuildHasher,
    R: Rng,
{
    type InsertError = NotEnoughSpace;

    fn contains(&self, item: &T) -> bool {
        let (i1, tag) = self.index_and_tag(item);
        let i2 = self.alt_index(i1, tag);
        debug_assert_eq!(i1, self.alt_index(i2, tag));

        self.contains_hashed(i1, i2, tag)
    }

    fn insert(&mut self, item: &T) -> Result<(), Self::InsertError> {
        let (i1, tag) = self.index_and_tag(item);
        let i2 = self.alt_index(i1, tag);
        debug_assert_eq!(i1, self.alt_index(i2, tag));

        if self.contains_hashed(i1, i2, tag) || self.try_insert(i1, tag).is_ok() {
            return Ok(());
        }

        let mut index = i1;

        for _ in 0..MAX_EVICTIONS {
            index = self.alt_index(index, tag);
            if self.maybe_evict_and_insert(index, tag).is_none() {
                return Ok(());
            }
        }

        Err(NotEnoughSpace)
    }
}

impl<T, const FINGERPRINT_SIZE: usize, H, R> std::fmt::Debug
    for CuckooFilter<T, FINGERPRINT_SIZE, H, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CuckooFilter {{ fingerprint_size: {}, num_buckets: {}, bucket_size: {} }}",
            FINGERPRINT_SIZE, self.num_buckets, self.bucket_size
        )
    }
}

#[derive(Debug, Clone)]
pub struct NotEnoughSpace;

impl std::fmt::Display for NotEnoughSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "not enough space")
    }
}

impl std::error::Error for NotEnoughSpace {}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::ThreadRng;
    use std::hash::{BuildHasherDefault, DefaultHasher};

    fn make_filter<const FINGERPRINT_SIZE: usize>(
        num_buckets: usize,
        bucket_size: usize,
    ) -> CuckooFilter<i32, FINGERPRINT_SIZE, BuildHasherDefault<DefaultHasher>, ThreadRng> {
        let build_hasher = BuildHasherDefault::<DefaultHasher>::default();
        let rng = rand::thread_rng();
        CuckooFilter::<_, FINGERPRINT_SIZE, _, _>::new(num_buckets, bucket_size, build_hasher, rng)
    }

    #[test]
    #[should_panic(expected = "num_buckets must be > 1")]
    fn test_num_buckets_too_small() {
        make_filter::<4>(1, 10);
    }

    #[test]
    #[should_panic(expected = "num_buckets must be a power of two")]
    fn test_num_buckets_power_of_two() {
        make_filter::<4>(100, 10);
    }

    #[test]
    #[should_panic(expected = "bucket_size must be > 0")]
    fn test_bucket_size_too_small() {
        make_filter::<4>(32, 0);
    }

    #[test]
    fn test_contains_empty() {
        let cf = make_filter::<4>(64, 4);

        for i in 0..100 {
            assert!(!cf.contains(&i));
        }
    }

    #[test]
    fn test_contains_inserted() {
        let mut cf = make_filter::<4>(64, 4);

        for i in 0..100 {
            cf.insert(&i).unwrap();
            assert!(cf.contains(&i));
        }
    }

    #[test]
    fn test_not_enough_space() {
        let mut cf = make_filter::<4>(2, 1);

        cf.insert(&1).unwrap();
        cf.insert(&2).unwrap();

        assert!(cf.insert(&3).is_err());
    }
}
