use super::types::ContainerFrame;

#[derive(Debug, Default, Clone)]
pub struct ContainerPath(pub Vec<ContainerFrame>);

impl ContainerPath {
    pub fn set_blockquote_depth(&mut self, depth: u8) {
        self.0
            .retain(|f| !matches!(f, ContainerFrame::BlockQuote { .. }));
        if depth > 0 {
            self.0.push(ContainerFrame::BlockQuote { depth });
        }
    }
}
