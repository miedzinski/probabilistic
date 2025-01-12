use crate::hash::Hashes;
use num_traits::{Unsigned, WrappingAdd};
use std::f64::consts::E;
use std::fmt::{Debug, Formatter};
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct CountMinSketch<T, H, C = u32> {
    counters: Vec<C>,
    width: usize,
    depth: usize,
    build_hasher: H,
    _phantom: PhantomData<T>,
}

impl<T, H, C> CountMinSketch<T, H, C>
where
    C: Clone + Unsigned,
{
    pub fn new(width: usize, depth: usize, build_hasher: H) -> Self {
        assert!(width > 0, "width must be > 0");
        assert!(depth > 0, "depth must be > 0");
        let size = width.checked_mul(depth).expect("width * depth overflow");
        Self {
            counters: vec![C::zero(); size],
            width,
            depth,
            build_hasher,
            _phantom: PhantomData,
        }
    }

    pub fn with_error_bounds(epsilon: f64, delta: f64, build_hasher: H) -> Self {
        assert!(
            0. < epsilon && epsilon <= 1.,
            "epsilon must be in the range (0, 1]"
        );
        assert!(
            0. < delta && delta < 1.,
            "delta must be in the range (0, 1)"
        );
        let width = (E / epsilon).ceil() as usize;
        let depth = (1. / delta).ceil() as usize;
        Self::new(width, depth, build_hasher)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn clear(&mut self) {
        self.counters.fill(C::zero());
    }
}

impl<T, H, C> CountMinSketch<T, H, C>
where
    T: Hash,
    C: Clone + Ord + Unsigned + WrappingAdd,
    H: BuildHasher,
{
    pub fn count(&self, item: &T) -> C {
        Hashes::new(item, self.width as u64, self.depth, &self.build_hasher)
            .enumerate()
            .map(|(i, hash)| {
                let idx = self.width * i + hash;
                self.counters[idx].clone()
            })
            .min()
            .unwrap()
    }

    pub fn increment(&mut self, item: &T, count: &C) {
        let hashes = Hashes::new(item, self.width as u64, self.depth, &self.build_hasher);
        for (i, hash) in hashes.enumerate() {
            let idx = self.width * i + hash;
            self.counters[idx] = self.counters[idx].wrapping_add(count);
        }
    }
}

impl<T, H, C> Debug for CountMinSketch<T, H, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CountMinSketch {{ width: {}, depth: {} }}",
            self.width, self.depth
        )
    }
}
