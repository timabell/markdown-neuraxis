#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceSig {
    Backticks,
    Tildes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceKind {
    Backticks,
    Tildes,
}

pub struct CodeFence;

impl CodeFence {
    pub const BACKTICKS: &'static str = "```";
    pub const TILDES: &'static str = "~~~";

    pub fn sig(remainder: &str) -> Option<FenceSig> {
        let t = remainder.trim_end_matches(['\r', '\n']);
        if t.starts_with(Self::BACKTICKS) {
            Some(FenceSig::Backticks)
        } else if t.starts_with(Self::TILDES) {
            Some(FenceSig::Tildes)
        } else {
            None
        }
    }

    pub fn kind(sig: FenceSig) -> FenceKind {
        match sig {
            FenceSig::Backticks => FenceKind::Backticks,
            FenceSig::Tildes => FenceKind::Tildes,
        }
    }

    pub fn closes(kind: FenceKind, sig: Option<FenceSig>) -> bool {
        matches!(
            (kind, sig),
            (FenceKind::Backticks, Some(FenceSig::Backticks))
                | (FenceKind::Tildes, Some(FenceSig::Tildes))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_backtick_fence() {
        assert_eq!(CodeFence::sig("```rust"), Some(FenceSig::Backticks));
    }

    #[test]
    fn detect_tilde_fence() {
        assert_eq!(CodeFence::sig("~~~"), Some(FenceSig::Tildes));
    }

    #[test]
    fn no_fence() {
        assert_eq!(CodeFence::sig("hello"), None);
    }

    #[test]
    fn closes_matching_fence() {
        assert!(CodeFence::closes(
            FenceKind::Backticks,
            Some(FenceSig::Backticks)
        ));
        assert!(CodeFence::closes(FenceKind::Tildes, Some(FenceSig::Tildes)));
    }

    #[test]
    fn does_not_close_mismatched_fence() {
        assert!(!CodeFence::closes(
            FenceKind::Backticks,
            Some(FenceSig::Tildes)
        ));
        assert!(!CodeFence::closes(
            FenceKind::Tildes,
            Some(FenceSig::Backticks)
        ));
    }
}
