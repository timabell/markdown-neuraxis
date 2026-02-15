use super::types::ContainerFrame;

/// Tracks the current container nesting path during block parsing.
///
/// As lines are processed, containers are pushed/popped to match
/// the line's structure (blockquote prefixes, list indentation, etc.)
#[derive(Debug, Default, Clone)]
pub struct ContainerPath(pub Vec<ContainerFrame>);

impl ContainerPath {
    /// Updates the blockquote nesting depth.
    ///
    /// Removes any existing BlockQuote frame and adds a new one if depth > 0.
    /// This simplified model will be refined when lists are added.
    pub fn set_blockquote_depth(&mut self, depth: u8) {
        self.0
            .retain(|f| !matches!(f, ContainerFrame::BlockQuote { .. }));
        if depth > 0 {
            self.0.push(ContainerFrame::BlockQuote { depth });
        }
    }
}
