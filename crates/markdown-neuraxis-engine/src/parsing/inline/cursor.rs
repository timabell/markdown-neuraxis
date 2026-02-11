#[derive(Clone)]
pub struct Cursor<'a> {
    pub s: &'a str,
    pub base: usize,
    pub i: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(s: &'a str, base: usize) -> Self {
        Self { s, base, i: 0 }
    }

    pub fn pos(&self) -> usize {
        self.base + self.i
    }

    pub fn eof(&self) -> bool {
        self.i >= self.s.len()
    }

    pub fn peek(&self) -> Option<u8> {
        self.s.as_bytes().get(self.i).copied()
    }

    pub fn starts_with(&self, pat: &[u8]) -> bool {
        self.s.as_bytes()[self.i..].starts_with(pat)
    }

    pub fn bump(&mut self) -> Option<u8> {
        let b = self.s.as_bytes().get(self.i).copied()?;
        self.i += 1;
        Some(b)
    }

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
