pub mod hll;
pub mod linear_count;

pub trait Cardinality<T> {
    fn count(&self) -> f64;
    fn insert(&mut self, item: &T);
}
