use crate::editing::{BlockKind, Marker, RenderBlock, Snapshot};
use crate::models::MarkdownFile;
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn SnapshotMainPanel(
    file: MarkdownFile,
    snapshot: Snapshot,
    on_file_select: Option<Callback<PathBuf>>,
    on_save: Callback<()>,
) -> Element {
    let display_name = file.display_path();

    rsx! {
        div {
            class: "document-container",
            h1 { "📝 {display_name}" }
            hr {}
            if !snapshot.blocks.is_empty() {
                div {
                    class: "document-content",
                    for (index, block) in snapshot.blocks.iter().enumerate() {
                        RenderBlockComponent {
                            key: "{index}",
                            block: block.clone(),
                            on_file_select: on_file_select
                        }
                    }
                }
            } else {
                div {
                    class: "empty-document",
                    p { "This document appears to be empty." }
                    button {
                        class: "add-block-button",
                        onclick: move |_| {
                            // TODO: Implement add block functionality using editing core
                            // todo!("Add block functionality using editing core not yet implemented");
                        },
                        "Add first block +"
                    }
                }
            }
        }
    }
}

#[component]
pub fn RenderBlockComponent(
    block: RenderBlock,
    on_file_select: Option<Callback<PathBuf>>,
) -> Element {
    match block.kind {
        BlockKind::Heading { level } => {
            let class_name = format!("heading level-{level}");
            match level {
                1 => rsx! { h1 { class: "{class_name}", "{block.content}" } },
                2 => rsx! { h2 { class: "{class_name}", "{block.content}" } },
                3 => rsx! { h3 { class: "{class_name}", "{block.content}" } },
                4 => rsx! { h4 { class: "{class_name}", "{block.content}" } },
                5 => rsx! { h5 { class: "{class_name}", "{block.content}" } },
                _ => rsx! { h6 { class: "{class_name}", "{block.content}" } },
            }
        }
        BlockKind::Paragraph => {
            rsx! {
                p {
                    class: "paragraph",
                    "{block.content}"
                }
            }
        }
        BlockKind::ListItem { marker, depth } => {
            let marker_text = match marker {
                Marker::Dash => "-",
                Marker::Asterisk => "*",
                Marker::Plus => "+",
                Marker::Numbered => "1.", // TODO: Get actual number
            };

            rsx! {
                div {
                    class: "list-item",
                    // Render indent blocks for proper CSS-based indentation
                    for _ in 0..depth {
                        div { class: "indent-block" }
                    }
                    span { class: "list-marker", "{marker_text} " }
                    span { class: "list-content", "{block.content}" }
                }
            }
        }
        BlockKind::CodeFence { lang } => {
            let code_class = if let Some(ref lang_str) = lang {
                format!("language-{lang_str}")
            } else {
                "language-text".to_string()
            };

            rsx! {
                div {
                    class: "code-block",
                    if let Some(lang_str) = lang {
                        div { class: "code-language", "{lang_str}" }
                    }
                    pre {
                        code {
                            class: "{code_class}",
                            "{block.content}"
                        }
                    }
                }
            }
        }
    }
}

/// EditorBlock component for raw markdown editing when a block is focused
/// This implements the editing pattern from ADR-0004 where focused blocks
/// switch to raw markdown editing mode with controlled textarea
#[component]
pub fn EditorBlock(
    block: RenderBlock,
    content_text: String,
    on_command: Callback<crate::editing::Cmd>,
    on_cancel: Callback<()>,
) -> Element {
    use dioxus::prelude::*;

    // Signal for the textarea value (controlled input)
    let mut textarea_value = use_signal(|| content_text.clone());

    rsx! {
        div {
            class: "editor-block",

            // Render indent blocks for proper CSS-based alignment
            for _ in 0..block.depth {
                div { class: "indent-block" }
            }

            // Gutter showing the block marker/prefix
            div {
                class: "editor-gutter",
                {render_block_prefix(&block)}
            }

            // Controlled textarea for raw markdown editing
            textarea {
                class: "editor-textarea",
                value: textarea_value(),
                spellcheck: false,
                rows: calculate_textarea_rows(&textarea_value()),

                // Basic input handling - full beforeinput mapping comes later
                oninput: move |event| {
                    // For now, update the textarea value directly
                    // TODO: Replace with proper command mapping via ADR-0004
                    textarea_value.set(event.value());
                },

                // Handle special keys via keydown
                onkeydown: move |event| {
                    match event.key() {
                        Key::Tab => {
                            event.prevent_default();
                            if event.modifiers().shift() {
                                // TODO: on_command.call(Cmd::OutdentLines { range: ... });
                            } else {
                                // TODO: on_command.call(Cmd::IndentLines { range: ... });
                            }
                        },
                        Key::Enter => {
                            event.prevent_default();
                            // TODO: on_command.call(Cmd::SplitListItem { at: ... });
                        },
                        Key::Escape => {
                            on_cancel.call(());
                        },
                        Key::Backspace => {
                            event.prevent_default();
                            // TODO: Handle backspace via DeleteRange command
                            let current = textarea_value();
                            if !current.is_empty() {
                                textarea_value.set(current[..current.len()-1].to_string());
                            }
                        },
                        _ => {} // Other keys will be handled by beforeinput
                    }
                },

                // Handle focus for editor lifecycle
                onfocus: move |_| {
                    // Editor is now active - could set focus state in parent
                },

                onblur: move |_| {
                    // Could auto-commit changes or validate on blur
                },
            }
        }
    }
}

/// Render the block prefix/marker for the editor gutter
fn render_block_prefix(block: &RenderBlock) -> String {
    match &block.kind {
        BlockKind::ListItem { marker, .. } => {
            match marker {
                Marker::Dash => "- ".to_string(),
                Marker::Asterisk => "* ".to_string(),
                Marker::Plus => "+ ".to_string(),
                Marker::Numbered => "1. ".to_string(), // TODO: Get actual number
            }
        }
        BlockKind::Heading { level } => {
            format!("{} ", "#".repeat(*level as usize))
        }
        BlockKind::Paragraph => String::new(),
        BlockKind::CodeFence { lang: _ } => "``` ".to_string(),
    }
}

/// Calculate appropriate number of rows for textarea based on content
fn calculate_textarea_rows(content: &str) -> u32 {
    let line_count = content.lines().count().max(1);
    (line_count as u32).min(20) // Cap at 20 rows to avoid huge textareas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::{AnchorId, Marker};

    #[test]
    fn test_render_block_prefix_list_item() {
        let block = RenderBlock {
            id: AnchorId(123), // Simple test anchor ID
            kind: BlockKind::ListItem {
                marker: Marker::Dash,
                depth: 1,
            },
            byte_range: 0..10,
            content_range: 4..10,
            depth: 1,
            content: "Test content".to_string(),
        };

        let prefix = render_block_prefix(&block);
        assert_eq!(prefix, "- ");
    }

    #[test]
    fn test_render_block_prefix_heading() {
        let block = RenderBlock {
            id: AnchorId(124),
            kind: BlockKind::Heading { level: 2 },
            byte_range: 0..15,
            content_range: 3..15,
            depth: 0,
            content: "Test Heading".to_string(),
        };

        let prefix = render_block_prefix(&block);
        assert_eq!(prefix, "## ");
    }

    #[test]
    fn test_render_block_prefix_paragraph() {
        let block = RenderBlock {
            id: AnchorId(125),
            kind: BlockKind::Paragraph,
            byte_range: 0..15,
            content_range: 0..15,
            depth: 0,
            content: "Regular paragraph".to_string(),
        };

        let prefix = render_block_prefix(&block);
        assert_eq!(prefix, "");
    }

    #[test]
    fn test_render_block_prefix_different_markers() {
        let test_cases = [
            (Marker::Dash, "- "),
            (Marker::Asterisk, "* "),
            (Marker::Plus, "+ "),
            (Marker::Numbered, "1. "),
        ];

        for (i, (marker, expected_prefix)) in test_cases.iter().enumerate() {
            let block = RenderBlock {
                id: AnchorId(200 + i as u128), // Unique ID for each test case
                kind: BlockKind::ListItem {
                    marker: marker.clone(),
                    depth: 0,
                },
                byte_range: 0..10,
                content_range: 2..10,
                depth: 0,
                content: "Content".to_string(),
            };

            let prefix = render_block_prefix(&block);
            assert_eq!(prefix, *expected_prefix);
        }
    }

    #[test]
    fn test_calculate_textarea_rows() {
        // Test single line content
        assert_eq!(calculate_textarea_rows("Single line"), 1);

        // Test multi-line content
        let multi_line = "Line 1\nLine 2\nLine 3";
        assert_eq!(calculate_textarea_rows(multi_line), 3);

        // Test empty content
        assert_eq!(calculate_textarea_rows(""), 1);

        // Test very long content (should be capped at 20)
        let long_content = "Line\n".repeat(30);
        assert_eq!(calculate_textarea_rows(&long_content), 20);
    }
}
