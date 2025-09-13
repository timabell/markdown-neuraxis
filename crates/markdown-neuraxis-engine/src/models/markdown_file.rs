use relative_path::{RelativePath, RelativePathBuf};

/// Represents a markdown file with a relative path and display-friendly name
#[derive(Debug, Clone, PartialEq)]
pub struct MarkdownFile {
    relative_path: RelativePathBuf,
    display_name: String,
    display_path: String,
}

impl MarkdownFile {
    /// Create a new MarkdownFile from a relative path
    pub fn new(relative_path: RelativePathBuf) -> Self {
        let display_name = Self::extract_display_name(&relative_path);
        let display_path = {
            let path_str = relative_path.as_str();
            // Strip .md extension from the full relative path
            path_str.strip_suffix(".md").unwrap_or(path_str).to_string()
        };

        Self {
            relative_path,
            display_name,
            display_path,
        }
    }

    /// Create from a relative path string
    pub fn from_relative_str(path: &str) -> Self {
        Self::new(RelativePathBuf::from(path))
    }

    /// Get the relative path
    pub fn relative_path(&self) -> &RelativePath {
        &self.relative_path
    }

    /// Get the display name (without .md extension)
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Get the display path (relative path without .md extension, for use in titles)
    pub fn display_path(&self) -> &str {
        &self.display_path
    }

    /// Extract display name from a relative path (strips .md extension)
    fn extract_display_name(path: &RelativePath) -> String {
        path.file_name()
            .map(|name| name.strip_suffix(".md").unwrap_or(name))
            .unwrap_or("Untitled")
            .to_string()
    }
}

impl From<RelativePathBuf> for MarkdownFile {
    fn from(path: RelativePathBuf) -> Self {
        Self::new(path)
    }
}

impl From<&str> for MarkdownFile {
    fn from(path: &str) -> Self {
        Self::from_relative_str(path)
    }
}
