/// A forward-only scanner over a borrowed webhook payload.
pub struct PayloadScanner<'a> {
    payload: &'a [u8],
    cursor: usize,
}

impl<'a> PayloadScanner<'a> {
    pub fn new(payload: &'a [u8]) -> Self {
        Self { payload, cursor: 0 }
    }

    pub fn find_string_value(&mut self, exact_key: &[u8]) -> Option<&'a str> {
        if exact_key.is_empty() {
            return None;
        }

        let remaining = self.payload.get(self.cursor..)?;
        let key_offset = remaining
            .windows(exact_key.len())
            .position(|window| window == exact_key)?;
        let value_start = self.cursor + key_offset + exact_key.len();
        self.cursor = value_start;

        let value_length = self.payload[value_start..]
            .iter()
            .position(|byte| *byte == b'"')?;
        let value_end = value_start + value_length;
        self.cursor = value_end + 1;

        std::str::from_utf8(&self.payload[value_start..value_end]).ok()
    }

    pub fn extract_push_metadata(&mut self) -> Option<(&'a str, &'a str)> {
        let repository = self.find_string_value(b"\"full_name\":\"")?;
        let commit_sha = self.find_string_value(b"\"after\":\"")?;

        Some((repository, commit_sha))
    }
}

#[cfg(test)]
mod tests {
    use super::PayloadScanner;

    const MOCK_PUSH_PAYLOAD: &[u8] = br#"{
        "repository":{"full_name":"NikitaaRamesh/aether-search"},
        "after":"a1b2c3d4e5f6"
    }"#;

    #[test]
    fn extracts_valid_metadata() {
        let mut scanner = PayloadScanner::new(MOCK_PUSH_PAYLOAD);

        assert_eq!(
            scanner.extract_push_metadata(),
            Some(("NikitaaRamesh/aether-search", "a1b2c3d4e5f6"))
        );
    }

    #[test]
    fn handles_missing_keys() {
        let mut scanner = PayloadScanner::new(br#"{"ref":"refs/heads/main"}"#);

        assert_eq!(scanner.extract_push_metadata(), None);
    }

    #[test]
    fn handles_truncated_payload() {
        let mut scanner = PayloadScanner::new(
            br#"{"repository":{"full_name":"NikitaaRamesh/aether-search"},"after":"a1b2c3d4e5f6"#,
        );

        assert_eq!(scanner.extract_push_metadata(), None);
    }
}
