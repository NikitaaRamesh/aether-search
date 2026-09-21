use std::sync::{Arc, atomic::Ordering};

use parking_lot::RwLock;

use crate::{DOCS_PER_BLOCK, IndexShard};

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
                shard
                    .active_docs
                    .store((local_idx + 1) as u16, Ordering::Release);
                drop(manifest);

                for row in term_rows {
                    shard.rows[*row].set_bit(local_idx);
                }
                break;
            }

            drop(manifest);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

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
}
