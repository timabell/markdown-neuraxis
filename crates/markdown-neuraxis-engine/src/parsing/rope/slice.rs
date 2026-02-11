use xi_rope::Rope;

use super::span::Span;

pub fn slice_to_string(rope: &Rope, sp: Span) -> String {
    rope.slice_to_cow(sp.start..sp.end).into_owned()
}

pub fn preview(rope: &Rope, sp: Span, max: usize) -> String {
    let mut s = slice_to_string(rope, sp);
    if s.len() > max {
        s.truncate(max);
        s.push_str("...");
    }
    s
}
