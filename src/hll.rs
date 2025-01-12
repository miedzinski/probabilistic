use std::fmt::{Debug, Formatter};
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

pub struct HyperLogLog<T, H> {
    registers: Registers,
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
}

impl<T, H> HyperLogLog<T, H>
where
    T: Hash,
    H: BuildHasher,
{
    pub fn count(&self) -> f64 {
        todo!()
    }

    pub fn insert(&mut self, item: &T) {
        let hash = self.build_hasher.hash_one(item);
        let index = (hash >> (64 - self.precision)) as usize;
        let rho = ((hash << self.precision).leading_zeros() + 1) as RegisterBlock;
        self.registers.update_max(index, rho);
    }
}

impl<T, H> Debug for HyperLogLog<T, H> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "HyperLogLog {{ precision: {} }}", self.precision)
    }
}

type RegisterBlock = u16;

const REGISTER_SIZE: u32 = 5;
const REGISTERS_IN_BLOCK: u32 = RegisterBlock::BITS / REGISTER_SIZE;
const MASK: RegisterBlock = (1 << REGISTER_SIZE) - 1;

struct Registers {
    blocks: Vec<RegisterBlock>,
    count: usize,
}

impl Registers {
    fn new(count: usize) -> Self {
        let num_blocks = (count as f64 / REGISTERS_IN_BLOCK as f64).ceil();
        Self {
            blocks: vec![0; num_blocks as usize],
            count,
        }
    }

    fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        self.blocks
            .iter()
            .flat_map(|block| {
                (0..REGISTERS_IN_BLOCK).map(move |i| {
                    let shift = i * REGISTER_SIZE;
                    ((block >> shift) & MASK) as u8
                })
            })
            .take(self.count)
    }

    fn update_max(&mut self, index: usize, value: RegisterBlock) {
        assert!(index < self.count, "index out of bounds");
        let (block_index, shift) = (
            index / REGISTERS_IN_BLOCK as usize,
            (REGISTER_SIZE * (index as u32 % REGISTERS_IN_BLOCK)) as RegisterBlock,
        );
        let current = (self.blocks[block_index] >> shift) & MASK;
        if value > current {
            self.blocks[block_index] =
                (self.blocks[block_index] & !(MASK << shift)) | (value as RegisterBlock) << shift;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Registers {
        fn with_blocks_and_count(blocks: Vec<RegisterBlock>, count: usize) -> Registers {
            Registers { blocks, count }
        }
    }

    #[test]
    fn test_number_of_blocks() {
        assert_eq!(Registers::new(0).blocks.len(), 0);
        assert_eq!(Registers::new(6).blocks.len(), 2);
        assert_eq!(Registers::new(7).blocks.len(), 3);
    }

    #[test]
    fn test_iter() {
        let blocks = vec![0b10001_00101_11000, 0b00000_11011_00101];
        let registers = Registers::with_blocks_and_count(blocks, 5);

        assert_eq!(
            registers.iter().collect::<Vec<_>>(),
            vec![0b11000, 0b00101, 0b10001, 0b00101, 0b11011]
        );
    }

    #[test]
    fn test_update_max() {
        let blocks = vec![0b10001_00101_11000, 0b00000_11011_00101];
        let mut registers = Registers::with_blocks_and_count(blocks, 5);

        registers.update_max(1, 0b01011);
        let expected = vec![0b10001_01011_11000, 0b00000_11011_00101];
        assert_eq!(registers.blocks, expected);

        registers.update_max(3, 0b00011);
        assert_eq!(registers.blocks, expected);
    }
}
