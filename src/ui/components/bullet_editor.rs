use crate::models::{BlockId, BulletOperation, ListItem};
use dioxus::prelude::*;
use std::path::PathBuf;

use super::TextSegmentComponent;

#[component]
pub fn BulletEditor(
    item: ListItem,
    block_id: BlockId,
    indent_level: usize,
    is_numbered: bool,
    item_number: Option<usize>,
    on_edit: Callback<BlockId>,
    on_bullet_operation: Callback<BulletOperation>,
    on_file_select: Option<Callback<PathBuf>>,
    document_state: crate::models::DocumentState,
) -> Element {
    // Check if this bullet is being edited (same as normal blocks)
    let editing_raw = document_state.is_editing(block_id);

    if let Some(raw_content) = editing_raw {
        let mut content = use_signal(|| raw_content.clone());

        // Update content if it changes externally
        let raw_content_clone = raw_content.clone();
        use_effect(move || {
            if content.read().clone() != raw_content_clone {
                content.set(raw_content_clone.clone());
            }
        });

        let handle_input = move |evt: FormEvent| {
            content.set(evt.value());
        };

        let save_content = move || {
            let final_content = content.read().clone();
            on_bullet_operation.call(BulletOperation::UpdateContent(block_id, final_content));
        };

        rsx! {
            div {
                class: "bullet-container",
                style: "margin-left: {indent_level * 24}px;",

                div {
                    class: "bullet-content",

                    // Bullet marker
                    span {
                        class: "bullet-marker",
                        if is_numbered {
                            if let Some(num) = item_number {
                                "{num}. "
                            } else {
                                "• "
                            }
                        } else {
                            "• "
                        }
                    }

                    // Textarea for editing (same as normal blocks)
                    textarea {
                        class: "bullet-input",
                        value: content.read().clone(),
                        autofocus: true,
                        rows: 1,
                        onmounted: move |evt| {
                            spawn(async move {
                                let _ = evt.set_focus(true).await;
                            });
                        },
                        oninput: handle_input,
                        onblur: move |_| {
                            save_content();
                        },
                        onkeydown: move |evt| {
                            let current_content = content.read().clone();
                            // For now, assume cursor is at end (proper tracking can be added later)
                            let cursor_pos = current_content.len();

                            match evt.key() {
                                Key::Escape => {
                                    save_content();
                                }
                                Key::Enter => {
                                    if evt.data().modifiers().ctrl() {
                                        save_content();
                                    } else if evt.data().modifiers().shift() {
                                        // Shift+Enter: Allow natural newline
                                    } else {
                                        // Enter: Split bullet at cursor
                                        evt.prevent_default();
                                        save_content(); // Save current content first
                                        on_bullet_operation.call(BulletOperation::SplitAtCursor(
                                            block_id,
                                            current_content,
                                            cursor_pos,
                                        ));
                                    }
                                }
                                Key::Tab => {
                                    evt.prevent_default();
                                    save_content(); // Save current content first
                                    if evt.data().modifiers().shift() {
                                        // Shift+Tab: Outdent
                                        on_bullet_operation.call(BulletOperation::Outdent(block_id));
                                    } else {
                                        // Tab: Indent
                                        on_bullet_operation.call(BulletOperation::Indent(block_id));
                                    }
                                }
                                Key::Backspace => {
                                    if cursor_pos == 0 && !current_content.is_empty() {
                                        // Backspace at start: Merge with previous bullet
                                        evt.prevent_default();
                                        save_content(); // Save current content first
                                        on_bullet_operation.call(BulletOperation::MergeWithPrevious(block_id));
                                    } else if cursor_pos == 0 && current_content.is_empty() {
                                        // Backspace on empty bullet: Delete it
                                        evt.prevent_default();
                                        on_bullet_operation.call(BulletOperation::DeleteEmpty(block_id));
                                    }
                                }
                                _ => {}
                            }
                        },
                    }
                }

                // Render nested children
                if !item.children.is_empty() {
                    div {
                        class: "nested-bullets",
                        for (_child_id, child) in &item.children {
                            BulletEditor {
                                key: "{_child_id:?}",
                                item: child.clone(),
                                block_id: *_child_id,
                                indent_level: indent_level + 1,
                                is_numbered: is_numbered,
                                item_number: None, // Nested bullets don't show numbers
                                on_edit: on_edit,
                                on_bullet_operation: on_bullet_operation,
                                on_file_select: on_file_select,
                                document_state: document_state.clone(),
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Not being edited - render as clickable text (same as normal blocks)
        rsx! {
            div {
                class: "bullet-container",
                style: "margin-left: {indent_level * 24}px;",

                div {
                    class: "bullet-content",

                    // Bullet marker
                    span {
                        class: "bullet-marker",
                        if is_numbered {
                            if let Some(num) = item_number {
                                "{num}. "
                            } else {
                                "• "
                            }
                        } else {
                            "• "
                        }
                    }

                    // Clickable text content (same as normal blocks)
                    span {
                        class: "bullet-text",
                        onclick: move |_| {
                            on_edit.call(block_id);
                        },

                        // Render content with text segments if available
                        if let Some(ref segments) = item.segments {
                            for segment in segments {
                                TextSegmentComponent {
                                    segment: segment.clone(),
                                    on_file_select: on_file_select
                                }
                            }
                        } else {
                            "{item.content}"
                        }
                    }
                }

                // Render nested children
                if !item.children.is_empty() {
                    div {
                        class: "nested-bullets",
                        for (_child_id, child) in &item.children {
                            BulletEditor {
                                key: "{_child_id:?}",
                                item: child.clone(),
                                block_id: *_child_id,
                                indent_level: indent_level + 1,
                                is_numbered: is_numbered,
                                item_number: None, // Nested bullets don't show numbers
                                on_edit: on_edit,
                                on_bullet_operation: on_bullet_operation,
                                on_file_select: on_file_select,
                                document_state: document_state.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}
