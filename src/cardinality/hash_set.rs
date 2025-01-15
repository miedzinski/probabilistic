use crate::cardinality::Cardinality;
use std::collections::HashSet;
use std::hash::Hash;

impl<T> Cardinality<T> for HashSet<T>
where
    T: Clone + Eq + Hash,
{
    fn count(&self) -> f64 {
        self.len() as f64
    }

    fn insert(&mut self, item: &T) {
        HashSet::<T>::insert(self, item.clone());
    }
}
