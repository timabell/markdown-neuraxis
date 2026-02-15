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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rope_produces_no_lines() {
        let rope = Rope::from("");
        let lines: Vec<_> = lines_with_spans(&rope).collect();
        assert!(lines.is_empty());
    }

    #[test]
    fn single_line_without_newline() {
        let rope = Rope::from("hello");
        let lines: Vec<_> = lines_with_spans(&rope).collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "hello");
        assert_eq!(lines[0].span, Span { start: 0, end: 5 });
    }

    #[test]
    fn single_line_with_newline() {
        let rope = Rope::from("hello\n");
        let lines: Vec<_> = lines_with_spans(&rope).collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "hello\n");
        assert_eq!(lines[0].span, Span { start: 0, end: 6 });
    }

    #[test]
    fn multiple_lines() {
        let rope = Rope::from("one\ntwo\nthree");
        let lines: Vec<_> = lines_with_spans(&rope).collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "one\n");
        assert_eq!(lines[0].span, Span { start: 0, end: 4 });
        assert_eq!(lines[1].text, "two\n");
        assert_eq!(lines[1].span, Span { start: 4, end: 8 });
        assert_eq!(lines[2].text, "three");
        assert_eq!(lines[2].span, Span { start: 8, end: 13 });
    }

    #[test]
    fn trailing_newline_handling() {
        let rope = Rope::from("one\ntwo\n");
        let lines: Vec<_> = lines_with_spans(&rope).collect();
        // xi_rope::lines_raw includes trailing newline in lines but does not
        // produce an empty line after a trailing newline
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "one\n");
        assert_eq!(lines[1].text, "two\n");
        assert_eq!(lines[1].span, Span { start: 4, end: 8 });
    }
}
