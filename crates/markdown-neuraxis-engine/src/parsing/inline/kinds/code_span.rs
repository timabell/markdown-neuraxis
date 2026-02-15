/// Code span inline type with owned delimiter constant.
///
/// Code spans are "raw zones" - no other inline parsing occurs inside them.
/// Per ADR-0012's knowledge ownership principle, the delimiter lives here.
pub struct CodeSpan;

impl CodeSpan {
    /// The backtick character that delimits code spans.
    pub const TICK: u8 = b'`';
}
