use std::hash::{BuildHasher, Hash, Hasher};

pub(crate) struct Hashes<'a, T, H> {
    h1: Option<u64>,
    h2: Option<u64>,
    i: usize,
    item: &'a T,
    modulus: u64,
    rounds: usize,
    build_hasher: &'a H,
}

impl<'a, T, H> Hashes<'a, T, H> {
    pub fn new(item: &'a T, modulus: u64, rounds: usize, build_hasher: &'a H) -> Self {
        Self {
            h1: None,
            h2: None,
            i: 1,
            item,
            modulus,
            rounds,
            build_hasher,
        }
    }
}

impl<T, H> Iterator for Hashes<'_, T, H>
where
    T: Hash,
    H: BuildHasher,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i > self.rounds {
            return None;
        }
        let hash = if self.i < 3 {
            let mut hasher = self.build_hasher.build_hasher();
            hasher.write_usize(self.i);
            self.item.hash(&mut hasher);
            let hash = hasher.finish();
            if self.i == 1 {
                self.h1 = Some(hash);
            } else {
                self.h2 = Some(hash);
            }
            Some(hash)
        } else {
            let (a, b) = (self.h1.unwrap(), self.h2.unwrap());
            self.h1 = Some(b);
            self.h2 = Some(a.wrapping_add(b.wrapping_mul(self.i as u64)));
            self.h2
        };
        self.i += 1;
        hash.map(|hash| (hash % self.modulus) as usize)
    }
}
