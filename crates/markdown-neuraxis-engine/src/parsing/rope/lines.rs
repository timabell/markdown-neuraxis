use xi_rope::Rope;

use super::span::Span;

#[derive(Debug, Clone)]
pub struct LineRef {
    pub span: Span,   // includes newline if present
    pub text: String, // scaffold: will be replaced with zero-copy later
}

/// Returns an iterator over lines with their byte spans.
/// Uses lines_raw to preserve newline characters.
pub fn lines_with_spans(rope: &Rope) -> impl Iterator<Item = LineRef> + '_ {
    let mut offset = 0usize;
    rope.lines_raw(..).map(move |line| {
        let start = offset;
        let len = line.len();
        offset += len;
        LineRef {
            span: Span { start, end: offset },
            text: line.into_owned(),
        }
    })
}
