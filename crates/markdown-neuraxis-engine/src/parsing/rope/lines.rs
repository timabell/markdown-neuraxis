use xi_rope::Rope;

use super::span::Span;

/// A reference to a single line in the rope with its byte span.
#[derive(Debug, Clone)]
pub struct LineRef {
    /// Byte span of this line in the rope (includes newline if present).
    pub span: Span,
    /// The line text as a string. Scaffold: will be replaced with zero-copy later.
    pub text: String,
}

/// Returns an iterator over lines with their byte spans.
///
/// Uses `lines_raw` to preserve newline characters, which is important for
/// accurate span tracking during block parsing.
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
