pub struct StructuralLexer<'a> {
    cursor: usize,
    data: &'a [u8],
}

impl<'a> StructuralLexer<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { cursor: 0, data }
    }
}

impl<'a> Iterator for StructuralLexer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.data.len() {
            let rest = &self.data[self.cursor..];
            let keyword_len = if rest.starts_with(b"fn")
                && rest.get(2).is_some_and(|byte| byte.is_ascii_whitespace())
            {
                2
            } else if rest.starts_with(b"struct")
                && rest.get(6).is_some_and(|byte| byte.is_ascii_whitespace())
            {
                6
            } else if rest.starts_with(b"impl")
                && rest.get(4).is_some_and(|byte| byte.is_ascii_whitespace())
            {
                4
            } else {
                self.cursor += 1;
                continue;
            };

            let mut identifier_start = self.cursor + keyword_len;
            while identifier_start < self.data.len()
                && self.data[identifier_start].is_ascii_whitespace()
            {
                identifier_start += 1;
            }

            let mut identifier_end = identifier_start;
            while identifier_end < self.data.len()
                && (self.data[identifier_end].is_ascii_alphanumeric()
                    || self.data[identifier_end] == b'_')
            {
                identifier_end += 1;
            }
            self.cursor = identifier_end;

            if identifier_start == identifier_end {
                continue;
            }

            match std::str::from_utf8(&self.data[identifier_start..identifier_end]) {
                Ok(valid_str) => return Some(valid_str),
                Err(_) => continue,
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::StructuralLexer;

    #[test]
    fn test_extracts_identifiers() {
        let payload =
            b"pub fn match_documents() { ... } \n struct IndexShard { ... } \n impl IndexManager";

        let identifiers: Vec<&str> = StructuralLexer::new(payload).collect();

        assert_eq!(
            identifiers,
            vec!["match_documents", "IndexShard", "IndexManager"]
        );
    }

    #[test]
    fn test_lexer_edge_cases() {
        let payload =
            b"fn\tspaced_func() {}\nstruct\n\nFastStruct {} impl\t\x80\x81BadUtf8 {} fn last_func() {}";

        let identifiers: Vec<&str> = StructuralLexer::new(payload).collect();

        assert_eq!(identifiers, vec!["spaced_func", "FastStruct", "last_func"]);
    }
}
