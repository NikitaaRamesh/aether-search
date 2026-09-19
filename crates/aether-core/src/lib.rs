pub type FastHashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;
pub type Mutex<T> = parking_lot::Mutex<T>;
pub type ArrayQueue<T> = crossbeam_queue::ArrayQueue<T>;

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_test_core() {
        assert!(std::hint::black_box(true));
    }
}
