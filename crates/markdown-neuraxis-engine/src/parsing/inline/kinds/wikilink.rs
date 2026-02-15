/// WikiLink inline type with owned delimiter constants.
///
/// Supports `[[target]]` and `[[target|alias]]` syntax.
/// Per ADR-0012's knowledge ownership principle, all delimiters live here.
pub struct WikiLink;

impl WikiLink {
    /// Opening delimiter for wikilinks.
    pub const OPEN: &'static [u8; 2] = b"[[";
    /// Closing delimiter for wikilinks.
    pub const CLOSE: &'static [u8; 2] = b"]]";
    /// Separator between target and alias.
    pub const ALIAS: u8 = b'|';
}
