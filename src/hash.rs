use std::hash::{BuildHasher, Hash};

pub(crate) fn iter_hashes<T, H>(item: &T, build_hasher: &H) -> impl Iterator<Item = u32>
where
    T: Hash,
    H: BuildHasher,
{
    let hash = build_hasher.hash_one(item);
    let h1 = (hash >> 32) as u32;
    let h2 = hash as u32;

    (1..u32::MAX).map(move |i| {
        h1.wrapping_add(h2.wrapping_mul(i))
            .wrapping_add(i.wrapping_mul(i).wrapping_mul(i))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{BuildHasher, Hasher};

    struct DummyHasher(u64);

    impl Hasher for DummyHasher {
        fn finish(&self) -> u64 {
            self.0
        }

        fn write(&mut self, bytes: &[u8]) {
            for &x in bytes.iter().rev() {
                self.0 <<= 8;
                self.0 |= x as u64;
            }
        }
    }

    struct DummyBuildHasher;

    impl BuildHasher for DummyBuildHasher {
        type Hasher = DummyHasher;

        fn build_hasher(&self) -> Self::Hasher {
            DummyHasher(0)
        }
    }

    #[test]
    fn test_iter_hashes() {
        let hashes = iter_hashes(&4294967298u64, &DummyBuildHasher)
            .take(5)
            .collect::<Vec<_>>();
        assert_eq!(hashes, vec![4, 13, 34, 73, 136])
    }
}
