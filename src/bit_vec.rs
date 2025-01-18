pub(crate) struct BitVec<const N: usize> {
    buf: Vec<u8>,
    count: usize,
}

impl<const N: usize> BitVec<N> {
    const CHUNK_LENGTH_OK: () = assert!(0 < N && N <= 8);
    const MASK: u8 = (1 << N) - 1;

    pub fn new(count: usize) -> Self {
        // Add a binding to enforce a compile-time assertion.
        #[allow(clippy::let_unit_value)]
        let _ = Self::CHUNK_LENGTH_OK;

        assert!(count > 0, "count must be > 0");
        // Allocate 1 extra byte for safe indexing byte pairs.
        let num_bytes = (((N * count) as f64) / 8f64).ceil() as usize + 1;

        Self {
            buf: vec![0; num_bytes],
            count,
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        (0..self.count).map(move |index| {
            // SAFETY: `index` is bound by registers count
            unsafe { self.get_unchecked(index) }
        })
    }

    pub fn get(&self, index: usize) -> u8 {
        assert!(index < self.count, "index out of bounds");
        // SAFETY: just checked that `index` is in bounds
        unsafe { self.get_unchecked(index) }
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> u8 {
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

    pub fn set(&mut self, index: usize, value: u8) {
        assert!(index < self.count, "index out of bounds");
        unsafe { self.set_unchecked(index, value) }
    }

    pub unsafe fn set_unchecked(&mut self, index: usize, value: u8) {
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

    impl<const N: usize> BitVec<N> {
        pub fn with_buf_and_count(buf: Vec<u8>, count: usize) -> Self {
            Self { buf, count }
        }
    }

    fn make_bit_vec() -> BitVec<5> {
        let buf = vec![0b101_11000, 0b1_10001_00, 0b1011_0010, 0b1, 0];
        BitVec::with_buf_and_count(buf, 5)
    }

    #[test]
    fn test_buffer_size() {
        assert_eq!(BitVec::<5>::new(1).buf.len(), 2);
        assert_eq!(BitVec::<5>::new(6).buf.len(), 5);
        assert_eq!(BitVec::<5>::new(8).buf.len(), 6);
        assert_eq!(BitVec::<6>::new(10).buf.len(), 9);
    }

    #[test]
    fn test_get() {
        let bv = make_bit_vec();

        assert_eq!(bv.get(0), 0b11000);
        assert_eq!(bv.get(1), 0b00101);
        assert_eq!(bv.get(2), 0b10001);
        assert_eq!(bv.get(3), 0b00101);
        assert_eq!(bv.get(4), 0b11011);
    }

    #[test]
    fn test_set_single_byte() {
        let mut bv = make_bit_vec();
        let expected = vec![0b101_11000, 0b1_01011_00, 0b1011_0010, 0b1, 0];

        bv.set(2, 0b01011);

        assert_eq!(bv.buf, expected);
    }

    #[test]
    fn test_set_byte_boundary() {
        let mut bv = make_bit_vec();
        let expected = vec![0b011_11000, 0b1_10001_01, 0b1011_0010, 0b1, 0];

        bv.set(1, 0b01011);

        assert_eq!(bv.buf, expected);
    }

    #[test]
    fn test_iter() {
        let bv = make_bit_vec();

        assert_eq!(
            bv.iter().collect::<Vec<_>>(),
            vec![0b11000, 0b00101, 0b10001, 0b00101, 0b11011]
        );
    }
}
