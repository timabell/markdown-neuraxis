use dioxus::events::Key;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{BlockKind, Cmd, RenderBlock};

/// EditorBlock component for raw markdown editing when a block is focused
/// This implements the editing pattern from ADR-0004 where focused blocks
/// switch to raw markdown editing mode with textarea element
#[component]
pub fn EditorBlock(
    block: RenderBlock,
    content_text: String,
    on_command: Callback<markdown_neuraxis_engine::editing::Cmd>,
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
                        handle_editor_keydown(
                            event,
                            &block_byte_range,
                            &block_content_range,
                            &block_kind,
                            &on_command,
                            &on_cancel,
                            &commit_changes,
                        );
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

// Helper function to handle editor keyboard events
fn handle_editor_keydown(
    event: Event<KeyboardData>,
    block_byte_range: &std::ops::Range<usize>,
    block_content_range: &std::ops::Range<usize>,
    block_kind: &BlockKind,
    on_command: &Callback<Cmd>,
    on_cancel: &Callback<()>,
    commit_changes: &impl Fn(),
) {
    match event.key() {
        Key::Tab => handle_tab_key(event, block_byte_range, on_command, commit_changes),
        Key::Enter => handle_enter_key(
            event,
            block_content_range,
            block_kind,
            on_command,
            commit_changes,
        ),
        Key::Escape => {
            commit_changes();
            on_cancel.call(());
        }
        _ => {}
    }
}

// Handle Tab key press for indentation
fn handle_tab_key(
    event: Event<KeyboardData>,
    block_byte_range: &std::ops::Range<usize>,
    on_command: &Callback<Cmd>,
    commit_changes: &impl Fn(),
) {
    event.prevent_default();
    commit_changes();

    let cmd = if event.modifiers().shift() {
        Cmd::OutdentLines {
            range: block_byte_range.clone(),
        }
    } else {
        Cmd::IndentLines {
            range: block_byte_range.clone(),
        }
    };
    on_command.call(cmd);
}

// Handle Enter key press for new lines or list item splitting
fn handle_enter_key(
    event: Event<KeyboardData>,
    block_content_range: &std::ops::Range<usize>,
    block_kind: &BlockKind,
    on_command: &Callback<Cmd>,
    commit_changes: &impl Fn(),
) {
    // Shift+Enter: allow default newline behavior
    if event.modifiers().shift() {
        return;
    }

    event.prevent_default();
    commit_changes();

    let cmd = match block_kind {
        BlockKind::ListItem { .. } => Cmd::SplitListItem {
            at: block_content_range.end,
        },
        _ => Cmd::InsertText {
            at: block_content_range.end,
            text: "\n".to_string(),
        },
    };
    on_command.call(cmd);
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
