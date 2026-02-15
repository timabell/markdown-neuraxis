/// Detected fence signature (what delimiter was seen on this line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceSig {
    /// Line starts with ``` (backticks).
    Backticks,
    /// Line starts with ~~~ (tildes).
    Tildes,
}

/// The kind of fence that opened a code block (used for matching closer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceKind {
    /// Opened with backticks, must close with backticks.
    Backticks,
    /// Opened with tildes, must close with tildes.
    Tildes,
}

/// Fenced code block type with owned delimiter constants.
///
/// Per ADR-0012's knowledge ownership principle, all fence-related
/// syntax knowledge lives here, not scattered in classifier/builder code.
pub struct CodeFence;

impl CodeFence {
    /// Triple backtick fence delimiter.
    pub const BACKTICKS: &'static str = "```";
    /// Triple tilde fence delimiter.
    pub const TILDES: &'static str = "~~~";

    /// Detects if a line remainder looks like a fence opener/closer.
    ///
    /// Returns the signature if the line starts with ``` or ~~~.
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

    /// Converts a fence signature to a fence kind.
    pub fn kind(sig: FenceSig) -> FenceKind {
        match sig {
            FenceSig::Backticks => FenceKind::Backticks,
            FenceSig::Tildes => FenceKind::Tildes,
        }
    }

    /// Checks if a fence signature closes a fence of the given kind.
    ///
    /// Backtick fences close with backticks, tilde fences with tildes.
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
