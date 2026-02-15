/// A cursor for character-by-character inline parsing with position tracking.
///
/// Operates over a string slice while tracking the absolute byte position
/// in the original rope (via `base` offset).
#[derive(Clone)]
pub struct Cursor<'a> {
    /// The string being parsed.
    pub s: &'a str,
    /// Base offset in the rope (added to local index for absolute positions).
    pub base: usize,
    /// Current local index into `s`.
    pub i: usize,
}

impl<'a> Cursor<'a> {
    /// Creates a new cursor at the start of `s` with the given base offset.
    pub fn new(s: &'a str, base: usize) -> Self {
        Self { s, base, i: 0 }
    }

    /// Returns the current absolute byte position (base + local index).
    pub fn pos(&self) -> usize {
        self.base + self.i
    }

    /// Returns true if at end of string.
    pub fn eof(&self) -> bool {
        self.i >= self.s.len()
    }

    /// Peeks at the current byte without advancing.
    pub fn peek(&self) -> Option<u8> {
        self.s.as_bytes().get(self.i).copied()
    }

    /// Checks if the remaining input starts with the given byte pattern.
    pub fn starts_with(&self, pat: &[u8]) -> bool {
        self.s.as_bytes()[self.i..].starts_with(pat)
    }

    /// Advances by one byte, returning the consumed byte.
    pub fn bump(&mut self) -> Option<u8> {
        let b = self.s.as_bytes().get(self.i).copied()?;
        self.i += 1;
        Some(b)
    }

    /// Advances by `n` bytes.
    pub fn bump_n(&mut self, n: usize) {
        self.i += n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_basics() {
        let mut cur = Cursor::new("hello", 10);
        assert_eq!(cur.pos(), 10);
        assert!(!cur.eof());
        assert_eq!(cur.peek(), Some(b'h'));
        assert_eq!(cur.bump(), Some(b'h'));
        assert_eq!(cur.pos(), 11);
    }

    #[test]
    fn cursor_starts_with() {
        let cur = Cursor::new("[[link]]", 0);
        assert!(cur.starts_with(b"[["));
        assert!(!cur.starts_with(b"]]"));
    }
}
