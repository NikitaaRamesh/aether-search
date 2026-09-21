pub mod lexer;
pub mod manager;
pub mod vocab;

use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};

use parking_lot::RwLock;

pub type FastHashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;
pub type Mutex<T> = parking_lot::Mutex<T>;
pub type ArrayQueue<T> = crossbeam_queue::ArrayQueue<T>;

pub const BITS_PER_WORD: usize = 64;
pub const WORDS_PER_BLOCK: usize = 8;
pub const DOCS_PER_BLOCK: usize = BITS_PER_WORD * WORDS_PER_BLOCK;
pub const BLOCK_ALIGNMENT_BYTES: usize = 64;

/// A 512-bit transposed signature row that fits in one 64-byte L1 cache line.
///
/// Each atomic word stores the membership bits for part of a 512-document
/// shard. The explicit C representation and 64-byte alignment keep adjacent
/// rows on distinct cache lines, eliminating false sharing between concurrent
/// worker threads.
// Rust requires an integer literal in `repr(align)`; the layout test binds it
// to `BLOCK_ALIGNMENT_BYTES`.
#[repr(C, align(64))]
pub struct AlignedSignatureBlock {
    pub data: [AtomicU64; WORDS_PER_BLOCK],
}

impl AlignedSignatureBlock {
    /// Sets the bit associated with a document in this signature row.
    ///
    /// # Panics
    ///
    /// Panics when `doc_index >= DOCS_PER_BLOCK`.
    ///
    /// # Synchronization
    ///
    /// `Ordering::Relaxed` is sufficient because individual bit flips are
    /// commutative, monotonic accumulations that do not publish external
    /// memory. Reader synchronization is governed downstream by shard sealing
    /// and the `doc_manifest` barrier.
    pub fn set_bit(&self, doc_index: usize) {
        assert!(
            doc_index < DOCS_PER_BLOCK,
            "document index {} out of bounds (max {})",
            doc_index,
            DOCS_PER_BLOCK - 1
        );

        let array_index = doc_index / BITS_PER_WORD;
        let bit_offset = doc_index % BITS_PER_WORD;
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

/// A fixed partition of up to [`crate::DOCS_PER_BLOCK`] documents in the
/// orthogonal bit-matrix.
///
/// Each entry in `rows` is a transposed signature row associated with a term
/// hash. `doc_manifest` maps shard-local document indices to global repository
/// or file paths and provides the reader synchronization barrier after shard
/// sealing. `active_docs` atomically tracks the shard's current occupancy.
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

    #[allow(clippy::needless_range_loop)]
    pub fn match_documents(&self, term_rows: &[usize]) -> Vec<usize> {
        if term_rows.is_empty() {
            return Vec::new();
        }

        let active_count = self.active_docs.load(Ordering::Acquire) as usize;
        let mut result = [u64::MAX; WORDS_PER_BLOCK];
        for row_idx in term_rows {
            if *row_idx >= self.rows.len() {
                return Vec::new();
            }

            for i in 0..WORDS_PER_BLOCK {
                result[i] &= self.rows[*row_idx].data[i].load(Ordering::Relaxed);
            }
        }

        let mut matches = Vec::new();
        for (word_idx, mut word) in result.into_iter().enumerate() {
            while word != 0 {
                let bit_pos = word.trailing_zeros() as usize;
                let doc_idx = (word_idx * BITS_PER_WORD) + bit_pos;

                if doc_idx >= active_count {
                    return matches;
                }
                matches.push(doc_idx);
                word &= word - 1;
            }
        }

        matches
    }
}

#[cfg(test)]
mod tests {
    use std::{
        mem::{align_of, size_of},
        sync::atomic::Ordering,
    };

    use super::{
        AlignedSignatureBlock, BITS_PER_WORD, BLOCK_ALIGNMENT_BYTES, DOCS_PER_BLOCK, IndexShard,
        WORDS_PER_BLOCK,
    };

    #[test]
    fn verify_memory_layout() {
        assert_eq!(align_of::<AlignedSignatureBlock>(), BLOCK_ALIGNMENT_BYTES);
        assert_eq!(size_of::<AlignedSignatureBlock>(), BLOCK_ALIGNMENT_BYTES);
        assert_eq!(
            WORDS_PER_BLOCK * size_of::<u64>(),
            BLOCK_ALIGNMENT_BYTES,
            "Payload must perfectly fill the cache line without compiler padding"
        );
    }

    #[test]
    fn verify_bit_flipping() {
        let block = AlignedSignatureBlock::default();

        block.set_bit(0);
        block.set_bit(BITS_PER_WORD - 1);
        block.set_bit(BITS_PER_WORD);
        block.set_bit(DOCS_PER_BLOCK - 1);

        assert_eq!(
            block.data[0].load(Ordering::Relaxed),
            1 | (1_u64 << (BITS_PER_WORD - 1))
        );
        assert_eq!(block.data[1].load(Ordering::Relaxed), 1);
        assert_eq!(
            block.data[WORDS_PER_BLOCK - 1].load(Ordering::Relaxed),
            1_u64 << (BITS_PER_WORD - 1)
        );
    }

    #[test]
    fn set_bit_out_of_bounds_panics() {
        let at_boundary = std::panic::catch_unwind(|| {
            AlignedSignatureBlock::default().set_bit(DOCS_PER_BLOCK);
        });
        let beyond_boundary = std::panic::catch_unwind(|| {
            AlignedSignatureBlock::default().set_bit(DOCS_PER_BLOCK + 10);
        });

        assert_out_of_bounds_panic(at_boundary.unwrap_err());
        assert_out_of_bounds_panic(beyond_boundary.unwrap_err());
    }

    #[test]
    fn test_simd_intersection() {
        let shard = IndexShard::new(10);

        shard.rows[2].set_bit(0);
        shard.rows[5].set_bit(0);

        shard.rows[2].set_bit(1);
        shard.rows[5].set_bit(1);
        shard.rows[8].set_bit(1);
        shard.active_docs.store(2, Ordering::Release);

        assert_eq!(shard.match_documents(&[2, 5]), vec![0, 1]);
        assert_eq!(shard.match_documents(&[2, 5, 8]), vec![1]);
    }

    #[test]
    fn match_documents_empty_returns_empty() {
        let shard = IndexShard::new(10);

        assert_eq!(shard.match_documents(&[]), Vec::<usize>::new());
    }

    #[test]
    fn match_documents_out_of_bounds_returns_empty() {
        let shard = IndexShard::new(10);

        assert_eq!(shard.match_documents(&[10]), Vec::<usize>::new());
    }

    #[test]
    fn match_documents_stops_at_active_count() {
        let shard = IndexShard::new(1);
        shard.rows[0].set_bit(0);
        shard.rows[0].set_bit(2);
        shard.active_docs.store(1, Ordering::Release);

        assert_eq!(shard.match_documents(&[0]), vec![0]);
    }

    #[track_caller]
    fn assert_out_of_bounds_panic(payload: Box<dyn std::any::Any + Send>) {
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .expect("panic payload must be a string");

        assert!(message.contains("out of bounds"));
    }
}
