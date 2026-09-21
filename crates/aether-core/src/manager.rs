use std::sync::{Arc, atomic::Ordering};

use parking_lot::RwLock;

use crate::{DOCS_PER_BLOCK, IndexShard};

const _: () = assert!(
    crate::DOCS_PER_BLOCK <= u16::MAX as usize,
    "DOCS_PER_BLOCK must fit within a 16-bit unsigned integer"
);

pub struct IndexManager {
    shards: RwLock<Vec<Arc<IndexShard>>>,
    num_rows: usize,
}

impl IndexManager {
    pub fn new(num_rows: usize) -> Self {
        Self {
            shards: RwLock::new(vec![Arc::new(IndexShard::new(num_rows))]),
            num_rows,
        }
    }

    fn get_active_shard(&self) -> Arc<IndexShard> {
        {
            let shards = self.shards.read();
            let active_shard = Arc::clone(
                shards
                    .last()
                    .expect("index manager must contain at least one shard"),
            );

            if active_shard.active_docs.load(Ordering::Acquire) < DOCS_PER_BLOCK as u16 {
                return active_shard;
            }
        }

        let mut shards = self.shards.write();
        let active_shard = Arc::clone(
            shards
                .last()
                .expect("index manager must contain at least one shard"),
        );

        if active_shard.active_docs.load(Ordering::Acquire) < DOCS_PER_BLOCK as u16 {
            return active_shard;
        }

        let new_shard = Arc::new(IndexShard::new(self.num_rows));
        shards.push(Arc::clone(&new_shard));
        new_shard
    }

    pub fn add_document(&self, doc_name: String, term_rows: &[usize]) {
        loop {
            let shard = self.get_active_shard();
            let mut manifest = shard.doc_manifest.write();
            let local_idx = manifest.len();

            if local_idx < DOCS_PER_BLOCK {
                manifest.push(doc_name);

                for row in term_rows.iter().filter(|&&r| r < shard.rows.len()) {
                    shard.rows[*row].set_bit(local_idx);
                }

                shard
                    .active_docs
                    .store((local_idx + 1) as u16, Ordering::Release);
                drop(manifest);
                break;
            }

            drop(manifest);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, atomic::Ordering};

    use super::IndexManager;

    #[test]
    fn test_shard_rollover() {
        let manager = IndexManager::new(10);

        for i in 0..513 {
            manager.add_document(format!("doc-{i}"), &[0]);
        }

        let shards = manager.shards.read();
        assert_eq!(shards.len(), 2);
        assert_eq!(shards[0].active_docs.load(Ordering::Acquire), 512);
        assert_eq!(shards[1].active_docs.load(Ordering::Acquire), 1);
    }

    #[test]
    fn test_concurrent_ingestion() {
        let manager = Arc::new(IndexManager::new(10));
        let mut handles = Vec::with_capacity(10);

        for thread_id in 0..10 {
            let manager = Arc::clone(&manager);
            handles.push(std::thread::spawn(move || {
                for document_id in 0..100 {
                    manager.add_document(format!("doc-{thread_id}-{document_id}"), &[1, 5]);
                }
            }));
        }

        for handle in handles {
            handle.join().expect("ingestion thread must not panic");
        }

        let shards = manager.shards.read();
        assert_eq!(shards.len(), 2);
        assert_eq!(shards[0].active_docs.load(Ordering::Acquire), 512);
        assert_eq!(shards[1].active_docs.load(Ordering::Acquire), 488);
    }

    #[test]
    fn test_invalid_term_rows_are_ignored() {
        let manager = IndexManager::new(10);

        manager.add_document("doc".to_owned(), &[1, 10, usize::MAX, 5]);

        let shards = manager.shards.read();
        assert_eq!(shards[0].active_docs.load(Ordering::Acquire), 1);
        assert_eq!(shards[0].match_documents(&[1, 5]), vec![0]);
    }
}
