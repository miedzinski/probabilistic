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
