use crate::editing::{
    AnchorId, BlockKind, Cmd, ContentGroup, Document, ListItem, Marker, RenderBlock, Snapshot,
};
use crate::models::MarkdownFile;
use dioxus::events::Key;
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn MainPanel(
    file: MarkdownFile,
    snapshot: Snapshot,
    document: Document,
    on_file_select: Option<Callback<PathBuf>>,
    on_save: Callback<()>,
    on_command: Callback<Cmd>,
) -> Element {
    // Focus state management - track which anchor is currently focused for editing
    let mut focused_anchor_id = use_signal(|| None::<AnchorId>);

    // Helper to navigate to next/previous block
    let mut navigate_to_block = {
        let mut focused_anchor_id = focused_anchor_id;
        let snapshot = snapshot.clone();
        move |direction: i32| {
            let current_focus = *focused_anchor_id.read();
            if let Some(current_id) = current_focus {
                // Find current block index and navigate
                if let Some(current_index) = snapshot.blocks.iter().position(|b| b.id == current_id)
                {
                    let next_index = (current_index as i32 + direction).max(0) as usize;
                    if next_index < snapshot.blocks.len() {
                        focused_anchor_id.set(Some(snapshot.blocks[next_index].id));
                    }
                }
            } else if !snapshot.blocks.is_empty() {
                // No block focused - focus first or last depending on direction
                let index = if direction > 0 {
                    0
                } else {
                    snapshot.blocks.len() - 1
                };
                focused_anchor_id.set(Some(snapshot.blocks[index].id));
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
                if focused_anchor_id.read().is_none() {
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
                                focused_anchor_id.set(Some(snapshot.blocks[0].id));
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
                {
                    // Use the pre-grouped content from the snapshot
                    let grouped_content = &snapshot.content_groups;

                    rsx! {
                        div {
                            class: "document-content",
                            for (group_index, group) in grouped_content.iter().enumerate() {
                                {
                                    match group {
                                        ContentGroup::SingleBlock(block) => {
                                            // For single blocks, check if focused and render EditorBlock or pretty view
                                            if focused_anchor_id.read().as_ref() == Some(&block.id) {
                                                rsx! {
                                                    EditorBlock {
                                                        key: "{group_index}-editor",
                                                        block: block.clone(),
                                                        content_text: document.slice_to_cow(block.byte_range.clone()).to_string(),
                                                        on_command: on_command,
                                                        on_cancel: {
                                                            let mut focused_anchor_id = focused_anchor_id;
                                                            move |_| {
                                                                // Cancel editing - return to pretty view
                                                                focused_anchor_id.set(None);
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                rsx! {
                                                    RenderBlockComponent {
                                                        key: "{group_index}-render",
                                                        block: block.clone(),
                                                        on_file_select: on_file_select,
                                                        on_focus: {
                                                            let mut focused_anchor_id = focused_anchor_id;
                                                            let block_id = block.id;
                                                            move |_| {
                                                                // Focus this block for editing
                                                                focused_anchor_id.set(Some(block_id));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                        ContentGroup::BulletListGroup { items: _ } | ContentGroup::NumberedListGroup { items: _ } => {
                                            // For list groups, always render the list - individual items handle their own editing
                                            rsx! {
                                                RenderContentGroup {
                                                    key: "{group_index}-render",
                                                    group: group.clone(),
                                                    on_file_select: on_file_select,
                                                    on_focus: {
                                                        let mut focused_anchor_id = focused_anchor_id;
                                                        move |block: RenderBlock| {
                                                            // Track focus signal changes

                                                            // Focus this list item for editing
                                                            focused_anchor_id.set(Some(block.id));

                                                            // Document state tracking for debugging when needed
                                                        }
                                                    },
                                                    on_command: on_command,
                                                    focused_anchor_id,
                                                    document: document.clone()
                                                }
                                            }
                                        }
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
                        onclick: {
                            let on_command = on_command;
                            move |_| {
                                // Add a basic bullet point to start editing
                                let cmd = crate::editing::Cmd::InsertText {
                                    at: 0,
                                    text: "- ".to_string(),
                                };
                                on_command.call(cmd);
                            }
                        },
                        "Add first block +"
                    }
                }
            }
        }
    }
}

/// Component to render a content group (either a single block or a list group)
#[component]
pub fn RenderContentGroup(
    group: ContentGroup,
    on_file_select: Option<Callback<PathBuf>>,
    on_focus: Callback<RenderBlock>,
    on_command: Callback<Cmd>,
    focused_anchor_id: Signal<Option<AnchorId>>,
    document: Document,
) -> Element {
    match group {
        ContentGroup::SingleBlock(block) => {
            let block_clone = block.clone();
            rsx! {
                RenderBlockComponent {
                    block,
                    on_file_select,
                    on_focus: {
                        move |_| on_focus.call(block_clone.clone())
                    }
                }
            }
        }
        ContentGroup::BulletListGroup { items } => {
            rsx! {
                RenderListGroup {
                    items,
                    list_type: "ul",
                    on_file_select,
                    on_focus,
                    on_command,
                    focused_anchor_id,
                    document
                }
            }
        }
        ContentGroup::NumberedListGroup { items } => {
            rsx! {
                RenderListGroup {
                    items,
                    list_type: "ol",
                    on_file_select,
                    on_focus,
                    on_command,
                    focused_anchor_id,
                    document
                }
            }
        }
    }
}

/// Component to render a nested list group as proper HTML ul/ol structure
#[component]
pub fn RenderListGroup(
    items: Vec<ListItem>,
    list_type: &'static str,
    on_file_select: Option<Callback<PathBuf>>,
    on_focus: Callback<RenderBlock>,
    on_command: Callback<Cmd>,
    focused_anchor_id: Signal<Option<AnchorId>>,
    document: Document,
) -> Element {
    match list_type {
        "ol" => rsx! {
            ol {
                class: "markdown-list",
                for item in items {
                    RenderListItem {
                        item,
                        on_file_select,
                        on_focus,
                        on_command,
                        focused_anchor_id,
                        document: document.clone()
                    }
                }
            }
        },
        _ => rsx! {
            ul {
                class: "markdown-list",
                for item in items {
                    RenderListItem {
                        item,
                        on_file_select,
                        on_focus,
                        on_command,
                        focused_anchor_id,
                        document: document.clone()
                    }
                }
            }
        },
    }
}

/// Component to render a single list item with potential nested children
#[component]
pub fn RenderListItem(
    item: ListItem,
    on_file_select: Option<Callback<PathBuf>>,
    on_focus: Callback<RenderBlock>,
    on_command: Callback<Cmd>,
    focused_anchor_id: Signal<Option<AnchorId>>,
    document: Document,
) -> Element {
    // Track render state for debugging when needed

    let is_focused = focused_anchor_id.read().as_ref() == Some(&item.block.id);

    // Track focused items for debugging when needed

    rsx! {
        li {
            class: "markdown-list-item",
            if is_focused {
                // This list item is focused - show editor
                EditorBlock {
                    block: item.block.clone(),
                    content_text: document.slice_to_cow(item.block.byte_range.clone()).to_string(),
                    on_command: on_command,
                    on_cancel: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |_| {
                            // Clear focus to exit editing mode
                            focused_anchor_id.set(None);
                        }
                    }
                }
            } else {
                // Show the list item content (clickable)
                span {
                    class: "list-content clickable-block",
                    onclick: {
                        let block = item.block.clone();
                        move |evt: MouseEvent| {
                            evt.stop_propagation();
                            // Track clicks for debugging when needed
                            // Use the centralized focus system
                            on_focus.call(block.clone());
                        }
                    },
                    "{item.block.content}"
                }
            }
            if !item.children.is_empty() {
                {
                    // Determine list type for children based on the first child's marker
                    let child_list_type = if let Some(first_child) = item.children.first() {
                        if let BlockKind::ListItem { marker: Marker::Numbered, .. } = &first_child.block.kind {
                            "ol"
                        } else {
                            "ul"
                        }
                    } else {
                        "ul"
                    };

                    rsx! {
                        RenderListGroup {
                            items: item.children,
                            list_type: child_list_type,
                            on_file_select,
                            on_focus,
                            on_command,
                            focused_anchor_id,
                            document: document.clone()
                        }
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
        BlockKind::ListItem { .. } => {
            panic!(
                "ListItem blocks should be grouped into proper ul/ol structure, not rendered individually"
            )
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
/// switch to raw markdown editing mode with textarea element
#[component]
pub fn EditorBlock(
    block: RenderBlock,
    content_text: String,
    on_command: Callback<crate::editing::Cmd>,
    on_cancel: Callback<()>,
) -> Element {
    use dioxus::prelude::*;

    // Local state for textarea content - only commit changes on specific events
    let local_content = use_signal(|| content_text.clone());

    // Helper to commit current changes to the document
    let commit_changes = {
        let on_command = on_command;
        let block_byte_range = block.byte_range.clone();
        move || {
            let current_text = local_content.read().clone();
            let replace_cmd = Cmd::ReplaceRange {
                range: block_byte_range.clone(),
                text: current_text,
            };
            on_command.call(replace_cmd);
        }
    };

    rsx! {
        div {
            class: "editor-block",

            // Render indent blocks for proper CSS-based alignment
            for _ in 0..block.depth {
                div { class: "indent-block" }
            }

            // Uncontrolled textarea that manages its own content locally
            textarea {
                class: "editor-textarea",
                value: local_content.read().clone(),
                spellcheck: false,
                rows: calculate_textarea_rows(&local_content.read()),
                autofocus: true,

                // Update local state only - no commands triggered on regular typing
                oninput: {
                    let mut local_content = local_content;
                    move |event: Event<FormData>| {
                        local_content.set(event.value());
                    }
                },

                // Handle special keyboard commands via keydown (Tab, Shift+Tab, Enter, Escape)
                onkeydown: {
                    let block_byte_range = block.byte_range.clone();
                    let block_content_range = block.content_range.clone();
                    let block_kind = block.kind.clone();
                    let on_command = on_command;
                    let on_cancel = on_cancel;
                    let commit_changes = commit_changes.clone();
                    move |event: Event<KeyboardData>| {
                        match event.key() {
                            Key::Tab => {
                                event.prevent_default();

                                // First commit any current changes
                                commit_changes();

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
                                    // Shift+Enter: allow default newline behavior
                                } else {
                                    event.prevent_default();

                                    // First commit any current changes
                                    commit_changes();

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
                                // Commit changes before canceling
                                commit_changes();
                                on_cancel.call(());
                            },
                            _ => {}
                        }
                    }
                },

                // Simple blur handler to commit changes when focus is lost
                onblur: {
                    let commit_changes = commit_changes.clone();
                    move |_| {
                        commit_changes();
                    }
                },

                // Handle focus for editor lifecycle
                onfocus: move |_| {
                    // Editor is now active
                },

                // ADR-0004: Composition event handling for IME support
                oncompositionstart: {
                    move |_| {
                        // IME composition started - let browser handle input until compositionend
                        // Disable our command processing during composition
                    }
                },

                oncompositionend: {
                    let on_command = on_command;
                    let block = block.clone();
                    move |event: Event<CompositionData>| {
                        // IME composition finished - apply the composed text as a command
                        let composition_data = event.data();
                        let composed_text = composition_data.data();

                        if !composed_text.is_empty() {
                            // Apply composition result as insert command
                            let cmd = Cmd::InsertText {
                                at: block.content_range.end, // Insert at end for now
                                text: composed_text,
                            };
                            on_command.call(cmd);
                        }
                    }
                },
            }
        }
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
