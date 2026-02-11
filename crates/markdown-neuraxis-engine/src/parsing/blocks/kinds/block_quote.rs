pub struct BlockQuote;

impl BlockQuote {
    pub const PREFIX: char = '>';

    /// Returns (depth, byte index into `s` after stripping prefixes).
    /// Intentionally small and self-contained.
    pub fn strip_prefixes(s: &str) -> (u8, usize) {
        let b = s.as_bytes();
        let mut i = 0usize;
        let mut depth = 0u8;

        loop {
            while i < b.len() && b[i] == b' ' {
                i += 1;
            }
            if i < b.len() && b[i] == (Self::PREFIX as u8) {
                depth = depth.saturating_add(1);
                i += 1;
                if i < b.len() && b[i] == b' ' {
                    i += 1;
                }
            } else {
                break;
            }
        }
        (depth, i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_no_quote() {
        assert_eq!(BlockQuote::strip_prefixes("hello"), (0, 0));
    }

    #[test]
    fn strip_single_quote() {
        assert_eq!(BlockQuote::strip_prefixes("> hello"), (1, 2));
    }

    #[test]
    fn strip_double_quote() {
        assert_eq!(BlockQuote::strip_prefixes("> > hello"), (2, 4));
    }

    #[test]
    fn strip_nested_quote_no_space() {
        assert_eq!(BlockQuote::strip_prefixes(">> hello"), (2, 3));
    }
}
