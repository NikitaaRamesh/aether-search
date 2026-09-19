pub use aether_core as core;

pub type FastHashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_test_query() {
        assert!(std::hint::black_box(true));
    }
}
