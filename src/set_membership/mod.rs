pub mod bloom;
mod hash_set;

pub trait SetMembership<T> {
    fn contains(&self, item: &T) -> bool;
    fn insert(&mut self, item: &T) -> bool;
}
