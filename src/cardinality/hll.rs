use crate::bit_vec::BitVec;
use crate::cardinality::Cardinality;
use std::fmt::{Debug, Formatter};
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

pub struct HyperLogLog<T, H> {
    registers: BitVec<6>,
    precision: usize,
    build_hasher: H,
    _phantom: PhantomData<T>,
}

impl<T, H> HyperLogLog<T, H> {
    pub fn new(precision: usize, build_hasher: H) -> Self {
        assert!(
            (4..=18).contains(&precision),
            "precision must be in the range [4, 18]"
        );
        Self {
            registers: BitVec::new(1 << precision),
            precision,
            build_hasher,
            _phantom: PhantomData,
        }
    }

    pub fn with_error(epsilon: f64, build_hasher: H) -> Self {
        assert!(
            0.0 < epsilon && epsilon < 1.0,
            "epsilon must be in the range (0, 1)"
        );
        let m = (1.04 / epsilon).powi(2);
        let precision = m.log2().ceil() as usize;
        Self::new(precision, build_hasher)
    }

    pub fn precision(&self) -> usize {
        self.precision
    }

    fn alpha(&self) -> f64 {
        let m = self.registers.count();
        if m >= 128 {
            0.7213 / (1. + 1.079 / m as f64)
        } else if m == 64 {
            0.709
        } else if m == 32 {
            0.697
        } else {
            0.673
        }
    }
}

impl<T, H> Cardinality<T> for HyperLogLog<T, H>
where
    T: Hash,
    H: BuildHasher,
{
    fn count(&self) -> f64 {
        let (v, z) = self.registers.iter().fold((0, 0.), |(v, z), register| {
            (
                v + if register == 0 { 1 } else { 0 },
                z + 1. / (1 << register) as f64,
            )
        });
        let m = self.registers.count() as f64;
        let estimate = self.alpha() * m * m * z;
        let two_pow_32 = (1u64 << 32) as f64;

        if estimate <= 2.5 * m && v > 0 {
            m * (m / v as f64).ln()
        } else if estimate > two_pow_32 / 30f64 {
            -two_pow_32 * (1. - (estimate / two_pow_32)).ln()
        } else {
            estimate
        }
    }

    fn insert(&mut self, item: &T) {
        let hash = self.build_hasher.hash_one(item);
        let index = (hash >> (64 - self.precision)) as usize;
        let zeros = ((hash << self.precision) | (1 << (self.precision - 1))).leading_zeros();
        let rho = (zeros as u8) + 1;
        let current = self.registers.get(index);
        if current < rho {
            self.registers.set(index, rho);
        }
    }
}

impl<T, H> Debug for HyperLogLog<T, H> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "HyperLogLog {{ precision: {} }}", self.precision)
    }
}
