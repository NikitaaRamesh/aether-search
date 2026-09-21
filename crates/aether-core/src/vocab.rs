use parking_lot::RwLock;

use crate::FastHashMap;

pub struct VocabularyMap {
    map: RwLock<FastHashMap<String, usize>>,
    max_terms: usize,
}

impl VocabularyMap {
    pub fn new(max_terms: usize) -> Self {
        Self {
            map: RwLock::new(FastHashMap::default()),
            max_terms,
        }
    }

    pub fn get_or_register(&self, term: &str) -> Option<usize> {
        if let Some(index) = self.map.read().get(term) {
            return Some(*index);
        }

        let mut map = self.map.write();
        if let Some(index) = map.get(term) {
            return Some(*index);
        }
        if map.len() >= self.max_terms {
            return None;
        }

        let new_idx = map.len();
        map.insert(term.to_owned(), new_idx);
        Some(new_idx)
    }

    pub fn get_term_row(&self, term: &str) -> Option<usize> {
        self.map.read().get(term).copied()
    }

    pub fn map_tokens_to_rows<'a, I>(&self, tokens: I) -> Vec<usize>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut rows = Vec::new();
        for token in tokens {
            if let Some(row_idx) = self.get_or_register(token)
                && !rows.contains(&row_idx)
            {
                rows.push(row_idx);
            }
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::VocabularyMap;

    #[test]
    fn test_registration_and_capacity() {
        let vocabulary = VocabularyMap::new(2);

        assert_eq!(vocabulary.get_or_register("term1"), Some(0));
        assert_eq!(vocabulary.get_or_register("term2"), Some(1));
        assert_eq!(vocabulary.get_or_register("term3"), None);
    }

    #[test]
    fn test_read_path_does_not_register() {
        let vocabulary = VocabularyMap::new(2);

        assert_eq!(vocabulary.get_term_row("unknown"), None);
        assert_eq!(vocabulary.map.read().len(), 0);
    }

    #[test]
    fn test_deduplication() {
        let vocabulary = VocabularyMap::new(2);

        let rows = vocabulary.map_tokens_to_rows(["fn:new", "struct:Shard", "fn:new"]);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows, vec![0, 1]);
    }
}
