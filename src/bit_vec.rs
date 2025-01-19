use num_traits::{AsPrimitive, FromPrimitive, PrimInt, Unsigned};
use std::marker::PhantomData;

pub(crate) struct BitVec<T, const N: usize> {
    buf: Vec<u8>,
    size: usize,
    _phantom: PhantomData<T>,
}

impl<T, const N: usize> BitVec<T, N>
where
    T: AsPrimitive<u8> + FromPrimitive + PrimInt + UnboundedShift + Unsigned,
{
    const PACKED_LENGTH_OK: () = assert!(
        0 < N && N <= 8 * size_of::<T>(),
        "N-bit words must fit into T"
    );

    pub fn new(size: usize) -> Self {
        // Add a binding to enforce a compile-time assertion.
        #[allow(clippy::let_unit_value)]
        let _ = Self::PACKED_LENGTH_OK;

        assert!(size > 0, "size must be > 0");
        // Allocate 1 extra byte for safe indexing byte pairs.
        let num_bytes = (N * size).div_ceil(8) + 1;

        Self {
            buf: vec![0; num_bytes],
            size,
            _phantom: PhantomData,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        (0..self.size).map(move |index| {
            // SAFETY: `index` is bound by the size of vec
            unsafe { self.get_unchecked(index) }
        })
    }

    pub fn get(&self, index: usize) -> T {
        assert!(index < self.size, "index out of bounds");
        // SAFETY: just checked that `index` is in bounds
        unsafe { self.get_unchecked(index) }
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> T {
        let (byte_index, offset) = Self::index_and_offset(index);
        let mut value = T::zero();

        for i in 0..N.div_ceil(8) {
            value = value
                | (T::from_u8(*self.buf.get_unchecked(byte_index + i)).unwrap() >> offset)
                    .ushl(8 * i as u32);
            value = value
                | (T::from_u8(*self.buf.get_unchecked(byte_index + i + 1))
                    .unwrap()
                    .ushl(8 * i as u32 + (8 - offset) as u32));
        }

        value & Self::lsb_mask()
    }

    pub fn set(&mut self, index: usize, value: T) {
        assert!(index < self.size, "index out of bounds");
        // SAFETY: just checked that `index` is in bounds
        unsafe { self.set_unchecked(index, value) }
    }

    pub unsafe fn set_unchecked(&mut self, index: usize, value: T) {
        let value = value & Self::lsb_mask();
        let (byte_index, offset) = Self::index_and_offset(index);

        for i in 0..N.div_ceil(8) {
            let value = value >> (8 * i);
            let mask = Self::lsb_mask() >> (8 * i);

            let lsb = self.buf.get_unchecked_mut(byte_index + i);
            *lsb = (*lsb & !(mask.as_() << offset)) | (value.as_() << offset);

            let msb = self.buf.get_unchecked_mut(byte_index + i + 1);
            *msb = (*msb & !(mask.as_() >> (8 - offset))) | (value.as_() >> (8 - offset));
        }
    }

    fn lsb_mask() -> T {
        (T::one() << N) - T::one()
    }

    fn index_and_offset(index: usize) -> (usize, usize) {
        (N * index / 8, N * index % 8)
    }
}

// TODO: Replace this once uXX::unbounded_shl stabilizes and num-traits provides corresponding trait.
pub trait UnboundedShift {
    fn ushl(self, rhs: u32) -> Self;
}

macro_rules! impl_shift {
    ($t:ty) => {
        impl UnboundedShift for $t {
            fn ushl(self, rhs: u32) -> Self {
                self.unbounded_shl(rhs)
            }
        }
    };
}

impl_shift!(u8);
impl_shift!(u16);
impl_shift!(u32);
impl_shift!(u64);
impl_shift!(u128);
impl_shift!(usize);

#[cfg(test)]
#[allow(clippy::unusual_byte_groupings)]
mod tests {
    use super::*;

    impl<T, const N: usize> BitVec<T, N> {
        pub fn with_buf_and_size(buf: Vec<u8>, size: usize) -> Self {
            Self {
                buf,
                size,
                _phantom: PhantomData,
            }
        }
    }

    fn make_bit_vec_u8() -> BitVec<u8, 5> {
        let buf = vec![0b101_11000, 0b1_10001_00, 0b1011_0010, 0b1, 0];
        BitVec::with_buf_and_size(buf, 5)
    }

    fn make_bit_vec_u32() -> BitVec<u32, 17> {
        let buf = vec![
            0b11010110,
            0b01000110,
            0b0010111_0,
            0b10111111,
            0b100010_00,
            0b00011110,
            0b01111_001,
            0b01011001,
            0b0001_1110,
            0b10011001,
            0b10111,
            0,
        ];
        BitVec::with_buf_and_size(buf, 5)
    }

    #[test]
    fn test_buffer_size() {
        assert_eq!(BitVec::<u8, 5>::new(1).buf.len(), 2);
        assert_eq!(BitVec::<u8, 5>::new(6).buf.len(), 5);
        assert_eq!(BitVec::<u8, 5>::new(8).buf.len(), 6);
        assert_eq!(BitVec::<u8, 6>::new(10).buf.len(), 9);
        assert_eq!(BitVec::<u32, 20>::new(3).buf.len(), 9);
        assert_eq!(BitVec::<u32, 20>::new(8).buf.len(), 21);
    }

    #[test]
    fn test_get_u8() {
        let bv = make_bit_vec_u8();

        assert_eq!(bv.get(0), 0b11000);
        assert_eq!(bv.get(1), 0b00101);
        assert_eq!(bv.get(2), 0b10001);
        assert_eq!(bv.get(3), 0b00101);
        assert_eq!(bv.get(4), 0b11011);
    }

    #[test]
    fn test_get_u32() {
        let bv = make_bit_vec_u32();

        assert_eq!(bv.get(0), 0b00100011011010110);
        assert_eq!(bv.get(1), 0b00101111110010111);
        assert_eq!(bv.get(2), 0b00100011110100010);
        assert_eq!(bv.get(3), 0b11100101100101111);
        assert_eq!(bv.get(4), 0b10111100110010001);
    }

    #[test]
    fn test_set_single_word_u8() {
        let mut bv = make_bit_vec_u8();
        let expected = vec![0b101_11000, 0b1_01011_00, 0b1011_0010, 0b1, 0];

        bv.set(2, 0b01011);

        assert_eq!(bv.buf, expected);
    }

    #[test]
    fn test_set_word_boundary_u8() {
        let mut bv = make_bit_vec_u8();
        let expected = vec![0b011_11000, 0b1_10001_01, 0b1011_0010, 0b1, 0];

        bv.set(1, 0b01011);

        assert_eq!(bv.buf, expected);
    }

    #[test]
    fn test_set_u32() {
        let mut bv = make_bit_vec_u32();
        let expected = vec![
            0b11010110,
            0b01000110,
            0b0010111_0,
            0b10111111,
            0b100000_00,
            0b00100001,
            0b01111_101,
            0b01011001,
            0b0001_1110,
            0b10011001,
            0b10111,
            0,
        ];

        bv.set(2, 0b10100100001100000);

        assert_eq!(bv.buf, expected);
    }

    #[test]
    fn test_iter_u8() {
        let bv = make_bit_vec_u8();

        assert_eq!(
            bv.iter().collect::<Vec<_>>(),
            vec![0b11000, 0b00101, 0b10001, 0b00101, 0b11011]
        );
    }

    #[test]
    fn test_iter_u32() {
        let bv = make_bit_vec_u32();

        assert_eq!(
            bv.iter().collect::<Vec<_>>(),
            vec![
                0b00100011011010110,
                0b00101111110010111,
                0b00100011110100010,
                0b11100101100101111,
                0b10111100110010001,
            ]
        );
    }
}
