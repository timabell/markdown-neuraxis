/// Paragraph block type (marker struct).
///
/// Paragraphs have no delimiters - they are the default leaf block
/// when no other block opener matches. Inline parsing is applied
/// to paragraph content.
pub struct Paragraph;
