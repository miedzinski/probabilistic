use crate::cardinality::Cardinality;
use std::fmt::{Debug, Formatter};
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

pub struct HyperLogLog<T, H> {
    registers: Registers<6>,
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
            registers: Registers::new(1 << precision),
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
        let m = self.registers.count as f64;
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
        self.registers.update_max(index, zeros as u8 + 1);
    }
}

impl<T, H> Debug for HyperLogLog<T, H> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "HyperLogLog {{ precision: {} }}", self.precision)
    }
}

struct Registers<const N: usize> {
    buf: Vec<u8>,
    count: usize,
}

impl<const N: usize> Registers<N> {
    const REGISTER_LENGTH_OK: () = assert!(0 < N && N <= 8);
    const MASK: u8 = (1 << N) - 1;

    fn new(count: usize) -> Self {
        // Add a binding to enforce a compile-time assertion.
        #[allow(clippy::let_unit_value)]
        let _ = Self::REGISTER_LENGTH_OK;

        assert!(count > 0, "count must be > 0");
        // Allocate 1 extra byte for safe indexing byte pairs.
        let num_bytes = (((N * count) as f64) / 8f64).ceil() as usize + 1;

        Self {
            buf: vec![0; num_bytes],
            count,
        }
    }

    fn count(&self) -> usize {
        self.count
    }

    fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        (0..self.count).map(move |index| {
            // SAFETY: `index` is bound by registers count
            unsafe { self.get_unchecked(index) }
        })
    }

    fn update_max(&mut self, index: usize, value: u8) {
        assert!(index < self.count, "index out of bounds");
        // SAFETY: just checked that `index` is in bounds
        unsafe {
            let current = self.get_unchecked(index);
            if value > current {
                self.set_unchecked(index, value);
            }
        }
    }

    unsafe fn get_unchecked(&self, index: usize) -> u8 {
        let (byte_index, offset) = Self::index_and_offset(index);
        let (first, second) = (
            self.buf.get_unchecked(byte_index),
            self.buf.get_unchecked(byte_index + 1),
        );
        let first_shifted = first >> offset;
        // TODO: Replace u16 cast with u8::unbounded_shl once it stabilizes.
        let second_shifted = ((*second as u16) << (8 - offset)) as u8;
        (first_shifted | second_shifted) & Self::MASK
    }

    unsafe fn set_unchecked(&mut self, index: usize, value: u8) {
        let (byte_index, offset) = Self::index_and_offset(index);
        let value_masked = value & Self::MASK;
        {
            let first = self.buf.get_unchecked_mut(byte_index);
            let first_cleared = *first & !(Self::MASK << offset);
            *first = first_cleared | (value_masked << offset);
        }
        let second = self.buf.get_unchecked_mut(byte_index + 1);
        let second_cleared = *second & !(Self::MASK >> (8 - offset));
        *second = second_cleared | (value_masked >> (8 - offset));
    }

    fn index_and_offset(index: usize) -> (usize, usize) {
        (N * index / 8, N * index % 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<const N: usize> Registers<N> {
        pub fn with_buf_and_count(buf: Vec<u8>, count: usize) -> Self {
            Self { buf, count }
        }
    }

    fn make_registers() -> Registers<5> {
        let buf = vec![0b101_11000, 0b1_10001_00, 0b1011_0010, 0b1, 0];
        Registers::with_buf_and_count(buf, 5)
    }

    #[test]
    fn test_buffer_size() {
        assert_eq!(Registers::<5>::new(1).buf.len(), 2);
        assert_eq!(Registers::<5>::new(6).buf.len(), 5);
        assert_eq!(Registers::<5>::new(8).buf.len(), 6);
        assert_eq!(Registers::<6>::new(10).buf.len(), 9);
    }

    #[test]
    fn test_iter() {
        let registers = make_registers();

        assert_eq!(
            registers.iter().collect::<Vec<_>>(),
            vec![0b11000, 0b00101, 0b10001, 0b00101, 0b11011]
        );
    }

    #[test]
    fn test_update_max() {
        let mut registers = make_registers();
        let expected = vec![0b011_11000, 0b1_10001_01, 0b1011_0010, 0b1, 0];

        registers.update_max(1, 0b01011);
        assert_eq!(registers.buf, expected);

        registers.update_max(3, 0b00011);
        assert_eq!(registers.buf, expected);
    }
}
