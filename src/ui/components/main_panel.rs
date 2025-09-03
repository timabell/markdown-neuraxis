use crate::editing::{AnchorId, BlockKind, Cmd, Document, Marker, RenderBlock, Snapshot};
use crate::models::MarkdownFile;
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn SnapshotMainPanel(
    file: MarkdownFile,
    snapshot: Snapshot,
    mut document: Document,
    on_file_select: Option<Callback<PathBuf>>,
    on_save: Callback<()>,
    on_document_changed: Callback<Document>,
) -> Element {
    // Focus state management - track which block is currently focused for editing
    let mut focused_block_id = use_signal(|| None::<AnchorId>);

    // Helper to navigate to next/previous block
    let mut navigate_to_block = {
        let mut focused_block_id = focused_block_id;
        let snapshot = snapshot.clone();
        move |direction: i32| {
            let current_focus = *focused_block_id.read();
            if let Some(current_id) = current_focus {
                // Find current block index and navigate
                if let Some(current_index) = snapshot.blocks.iter().position(|b| b.id == current_id)
                {
                    let next_index = (current_index as i32 + direction).max(0) as usize;
                    if next_index < snapshot.blocks.len() {
                        focused_block_id.set(Some(snapshot.blocks[next_index].id));
                    }
                }
            } else if !snapshot.blocks.is_empty() {
                // No block focused - focus first or last depending on direction
                let index = if direction > 0 {
                    0
                } else {
                    snapshot.blocks.len() - 1
                };
                focused_block_id.set(Some(snapshot.blocks[index].id));
            }
        }
    };

    let display_name = file.display_path();

    rsx! {
        div {
            class: "document-container",
            tabindex: "0", // Make container focusable for keyboard navigation

            // Handle keyboard navigation when not in editing mode
            onkeydown: move |event| {
                // Only handle navigation when no block is being edited
                if focused_block_id.read().is_none() {
                    match event.key() {
                        Key::Tab => {
                            event.prevent_default();
                            if event.modifiers().shift() {
                                navigate_to_block(-1); // Previous block
                            } else {
                                navigate_to_block(1); // Next block
                            }
                        },
                        Key::Enter => {
                            // Enter focuses the first block if none are focused
                            if !snapshot.blocks.is_empty() {
                                focused_block_id.set(Some(snapshot.blocks[0].id));
                            }
                        },
                        Key::ArrowDown => {
                            event.prevent_default();
                            navigate_to_block(1);
                        },
                        Key::ArrowUp => {
                            event.prevent_default();
                            navigate_to_block(-1);
                        },
                        _ => {}
                    }
                }
            },

            h1 { "📝 {display_name}" }
            hr {}
            if !snapshot.blocks.is_empty() {
                div {
                    class: "document-content",
                    for (index, block) in snapshot.blocks.iter().enumerate() {
                        // Switch between pretty rendering and raw editing based on focus state
                        if focused_block_id.read().as_ref() == Some(&block.id) {
                            // Block is focused - show raw markdown editor
                            EditorBlock {
                                key: "{index}-editor",
                                block: block.clone(),
                                content_text: document.slice_to_cow(block.byte_range.clone()).to_string(),
                                on_command: {
                                    let mut document = document.clone();
                                    let on_document_changed = on_document_changed;
                                    move |cmd: Cmd| {
                                        // Apply command to document
                                        let _patch = document.apply(cmd);

                                        // Notify parent of document change
                                        on_document_changed.call(document.clone());

                                        // Important: Keep the block focused to stay in edit mode
                                        // The focused_block_id is maintained, so editing continues
                                    }
                                },
                                on_cancel: {
                                    let mut focused_block_id = focused_block_id;
                                    move |_| {
                                        // Cancel editing - return to pretty view
                                        focused_block_id.set(None);
                                    }
                                }
                            }
                        } else {
                            // Block is not focused - show pretty rendering
                            RenderBlockComponent {
                                key: "{index}-render",
                                block: block.clone(),
                                on_file_select: on_file_select,
                                on_focus: {
                                    let mut focused_block_id = focused_block_id;
                                    let block_id = block.id;
                                    move |_| {
                                        // Focus this block for editing
                                        focused_block_id.set(Some(block_id));
                                    }
                                }
                            }
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
    on_focus: Callback<()>,
) -> Element {
    match block.kind {
        BlockKind::Heading { level } => {
            let class_name = format!("heading level-{level} clickable-block");
            match level {
                1 => {
                    rsx! { h1 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{block.content}" } }
                }
                2 => {
                    rsx! { h2 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{block.content}" } }
                }
                3 => {
                    rsx! { h3 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{block.content}" } }
                }
                4 => {
                    rsx! { h4 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{block.content}" } }
                }
                5 => {
                    rsx! { h5 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{block.content}" } }
                }
                _ => {
                    rsx! { h6 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{block.content}" } }
                }
            }
        }
        BlockKind::Paragraph => {
            rsx! {
                p {
                    class: "paragraph clickable-block",
                    onclick: move |_| on_focus.call(()),
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
                    class: "list-item clickable-block",
                    onclick: move |_| on_focus.call(()),
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
                    class: "code-block clickable-block",
                    onclick: move |_| on_focus.call(()),
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
/// switch to raw markdown editing mode with contentEditable div
#[component]
pub fn EditorBlock(
    block: RenderBlock,
    content_text: String,
    on_command: Callback<crate::editing::Cmd>,
    on_cancel: Callback<()>,
) -> Element {
    use dioxus::prelude::*;

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

            // ContentEditable div for raw markdown editing
            div {
                class: "editor-content",
                contenteditable: "true",
                spellcheck: "false",
                autofocus: "true",
                // Use dangerous_inner_html to set initial text content properly
                // We escape HTML to ensure raw markdown display
                dangerous_inner_html: html_escape::encode_text(&content_text).to_string(),

                // ADR-0004: Handle input events for contentEditable
                // We need to extract the plain text content and create appropriate commands
                oninput: {
                    let on_command = on_command;
                    let block_content_range = block.content_range.clone();
                    move |event: Event<FormData>| {
                        // Get the text content from the contentEditable element
                        // Note: In a native Dioxus desktop app, we get the textContent, not innerHTML
                        let new_value = event.value();

                        // Delete the old content and insert the new
                        // First delete the existing content
                        if !block_content_range.is_empty() {
                            let delete_cmd = Cmd::DeleteRange {
                                range: block_content_range.clone(),
                            };
                            on_command.call(delete_cmd);
                        }

                        // Then insert the new content
                        if !new_value.is_empty() {
                            let insert_cmd = Cmd::InsertText {
                                at: block_content_range.start,
                                text: new_value,
                            };
                            on_command.call(insert_cmd);
                        }
                    }
                },

                // Handle special keyboard commands via keydown (Tab, Shift+Tab, Enter, Escape)
                onkeydown: {
                    let block_byte_range = block.byte_range.clone();
                    let block_content_range = block.content_range.clone();
                    let block_kind = block.kind.clone();
                    let on_command = on_command;
                    let on_cancel = on_cancel;
                    move |event: Event<KeyboardData>| {
                        match event.key() {
                            Key::Tab => {
                                event.prevent_default();

                                if event.modifiers().shift() {
                                    // Shift+Tab: Outdent lines
                                    let cmd = Cmd::OutdentLines {
                                        range: block_byte_range.clone(),
                                    };
                                    on_command.call(cmd);
                                } else {
                                    // Tab: Indent lines
                                    let cmd = Cmd::IndentLines {
                                        range: block_byte_range.clone(),
                                    };
                                    on_command.call(cmd);
                                }
                            },
                            Key::Enter => {
                                if event.modifiers().shift() {
                                    // Shift+Enter: allow default newline behavior in contentEditable
                                } else {
                                    event.prevent_default();
                                    // Enter: Split list item if in a list
                                    match block_kind {
                                        BlockKind::ListItem { .. } => {
                                            let cmd = Cmd::SplitListItem {
                                                at: block_content_range.end, // Insert at end for now
                                            };
                                            on_command.call(cmd);
                                        },
                                        _ => {
                                            // For non-list items, just insert a newline
                                            let cmd = Cmd::InsertText {
                                                at: block_content_range.end,
                                                text: "\n".to_string(),
                                            };
                                            on_command.call(cmd);
                                        }
                                    }
                                }
                            },
                            Key::Escape => {
                                event.prevent_default();
                                on_cancel.call(());
                            },
                            _ => {}
                        }
                    }
                },

                // Auto-cancel editing when focus is lost
                onblur: move |_event| {
                    // ContentEditable naturally maintains focus better than textarea
                    // Still cancel on blur for now, but this should be more stable
                    on_cancel.call(());
                },

                // Handle focus for editor lifecycle
                onfocus: move |_| {
                    // Editor is now active
                    // ContentEditable should maintain focus naturally
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
}
