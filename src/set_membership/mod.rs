pub mod bloom;
pub mod cuckoo;
pub mod hash_set;

pub trait SetMembership<T> {
    type InsertError;

    fn contains(&self, item: &T) -> bool;
    fn insert(&mut self, item: &T) -> Result<(), Self::InsertError>;
}
