use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};

use parking_lot::RwLock;

pub type FastHashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;
pub type Mutex<T> = parking_lot::Mutex<T>;
pub type ArrayQueue<T> = crossbeam_queue::ArrayQueue<T>;

#[repr(C, align(64))]
pub struct AlignedSignatureBlock {
    pub data: [AtomicU64; 8],
}

impl AlignedSignatureBlock {
    pub fn set_bit(&self, doc_index: usize) {
        assert!(doc_index < 512, "document index must be between 0 and 511");

        let array_index = doc_index / 64;
        let bit_offset = doc_index % 64;
        let _ = self.data[array_index].fetch_or(1_u64 << bit_offset, Ordering::Relaxed);
    }
}

impl Default for AlignedSignatureBlock {
    fn default() -> Self {
        Self {
            data: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }
}

pub struct IndexShard {
    pub rows: Vec<AlignedSignatureBlock>,
    pub doc_manifest: RwLock<Vec<String>>,
    pub active_docs: AtomicU16,
}

impl IndexShard {
    pub fn new(num_rows: usize) -> Self {
        let rows = std::iter::repeat_with(AlignedSignatureBlock::default)
            .take(num_rows)
            .collect();

        Self {
            rows,
            doc_manifest: RwLock::new(Vec::new()),
            active_docs: AtomicU16::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        mem::{align_of, size_of},
        sync::atomic::Ordering,
    };

    use super::AlignedSignatureBlock;

    #[test]
    fn verify_memory_layout() {
        assert_eq!(align_of::<AlignedSignatureBlock>(), 64);
        assert_eq!(size_of::<AlignedSignatureBlock>(), 64);
    }

    #[test]
    fn verify_bit_flipping() {
        let block = AlignedSignatureBlock::default();

        block.set_bit(0);
        block.set_bit(63);
        block.set_bit(64);
        block.set_bit(511);

        assert_eq!(
            block.data[0].load(Ordering::Relaxed),
            (1_u64 << 0) | (1_u64 << 63)
        );
        assert_eq!(block.data[1].load(Ordering::Relaxed), 1);
        assert_eq!(block.data[7].load(Ordering::Relaxed), 1_u64 << 63);
    }
}
