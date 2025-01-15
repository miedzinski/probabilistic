use crate::set_membership::SetMembership;
use std::collections::HashSet;
use std::hash::Hash;

impl<T> SetMembership<T> for HashSet<T>
where
    T: Clone + Eq + Hash,
{
    fn contains(&self, item: &T) -> bool {
        HashSet::<T>::contains(self, item)
    }

    fn insert(&mut self, item: &T) -> bool {
        HashSet::<T>::contains(self, item)
    }
}
