use num_traits::{PrimInt, Unsigned};

pub(crate) struct BitVec<T, const N: usize> {
    buf: Vec<T>,
    size: usize,
}

impl<T, const N: usize> BitVec<T, N>
where
    T: PrimInt + UShl + Unsigned,
{
    const WORD_SIZE: usize = 8 * size_of::<T>();
    const PACKED_LENGTH_OK: () = assert!(0 < N && N <= Self::WORD_SIZE);

    pub fn new(size: usize) -> Self {
        // Add a binding to enforce a compile-time assertion.
        #[allow(clippy::let_unit_value)]
        let _ = Self::PACKED_LENGTH_OK;

        assert!(size > 0, "size must be > 0");
        // Allocate 1 extra word for safe indexing word pairs.
        let num_words = (((N * size) as f64) / Self::WORD_SIZE as f64).ceil() as usize + 1;

        Self {
            buf: vec![T::zero(); num_words],
            size,
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
        let (word_index, offset) = Self::index_and_offset(index);
        let (first, second) = (
            *self.buf.get_unchecked(word_index),
            *self.buf.get_unchecked(word_index + 1),
        );
        let first_shifted = first >> offset;
        let second_shifted = second.ushl((Self::WORD_SIZE - offset) as u32);
        (first_shifted | second_shifted) & Self::lsb_mask()
    }

    pub fn set(&mut self, index: usize, value: T) {
        assert!(index < self.size, "index out of bounds");
        unsafe { self.set_unchecked(index, value) }
    }

    pub unsafe fn set_unchecked(&mut self, index: usize, value: T) {
        let (word_index, offset) = Self::index_and_offset(index);
        let value_masked = value & Self::lsb_mask();
        {
            let first = self.buf.get_unchecked_mut(word_index);
            let first_cleared = *first & !(Self::lsb_mask() << offset);
            *first = first_cleared | (value_masked << offset);
        }
        let second = self.buf.get_unchecked_mut(word_index + 1);
        let second_cleared = *second & !(Self::lsb_mask() >> (Self::WORD_SIZE - offset));
        *second = second_cleared | (value_masked >> (Self::WORD_SIZE - offset));
    }

    fn lsb_mask() -> T {
        (T::one() << N) - T::one()
    }

    fn index_and_offset(index: usize) -> (usize, usize) {
        (N * index / Self::WORD_SIZE, N * index % Self::WORD_SIZE)
    }
}

// TODO: Replace this once uXX::unbounded_shl stabilizes and num-traits provides corresponding trait.
pub trait UShl {
    fn ushl(self, rhs: u32) -> Self;
}

macro_rules! impl_ushl {
    ($t:ty) => {
        impl UShl for $t {
            fn ushl(self, rhs: u32) -> Self {
                self.unbounded_shl(rhs)
            }
        }
    };
}

impl_ushl!(u8);
impl_ushl!(u16);
impl_ushl!(u32);
impl_ushl!(u64);
impl_ushl!(u128);
impl_ushl!(usize);

#[cfg(test)]
#[allow(clippy::unusual_byte_groupings)]
mod tests {
    use super::*;

    impl<T, const N: usize> BitVec<T, N> {
        pub fn with_buf_and_size(buf: Vec<T>, size: usize) -> Self {
            Self { buf, size }
        }
    }

    fn make_bit_vec_u8() -> BitVec<u8, 5> {
        let buf = vec![0b101_11000, 0b1_10001_00, 0b1011_0010, 0b1, 0];
        BitVec::with_buf_and_size(buf, 5)
    }

    fn make_bit_vec_u32() -> BitVec<u32, 20> {
        let buf = vec![
            0b111100010000_01111001000000100110,
            0b1111_00010000010010001001_00110010,
            0b0010001101011001_0011001001100101,
            0b1111,
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
        assert_eq!(BitVec::<u32, 20>::new(2).buf.len(), 3);
        assert_eq!(BitVec::<u32, 20>::new(8).buf.len(), 6);
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

        assert_eq!(bv.get(0), 0b01111001000000100110);
        assert_eq!(bv.get(1), 0b00110010111100010000);
        assert_eq!(bv.get(2), 0b00010000010010001001);
        assert_eq!(bv.get(3), 0b00110010011001011111);
        assert_eq!(bv.get(4), 0b11110010001101011001);
    }

    #[test]
    fn test_set_single_word_u8() {
        let mut bv = make_bit_vec_u8();
        let expected = vec![0b101_11000, 0b1_01011_00, 0b1011_0010, 0b1, 0];

        bv.set(2, 0b01011);

        assert_eq!(bv.buf, expected);
    }

    #[test]
    fn test_set_single_word_u32() {
        let mut bv = make_bit_vec_u32();
        let expected = vec![
            0b111100010000_01111001000000100110,
            0b1111_10100100001100000000_00110010,
            0b0010001101011001_0011001001100101,
            0b1111,
            0,
        ];

        bv.set(2, 0b10100100001100000000);

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
    fn test_set_word_boundary_u32() {
        let mut bv = make_bit_vec_u32();
        let expected = vec![
            0b001100000000_01111001000000100110,
            0b1111_00010000010010001001_10100100,
            0b0010001101011001_0011001001100101,
            0b1111,
            0,
        ];

        bv.set(1, 0b10100100001100000000);

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
                0b01111001000000100110,
                0b00110010111100010000,
                0b00010000010010001001,
                0b00110010011001011111,
                0b11110010001101011001,
            ]
        );
    }
}
