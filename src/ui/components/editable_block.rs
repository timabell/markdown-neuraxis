use crate::models::{BlockId, ContentBlock, ListItem};
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn EditableBlock(
    block: ContentBlock,
    block_id: BlockId,
    editing_raw: Option<String>, // Some(raw) if this block is being edited
    is_selected: bool,           // Whether this block is selected for navigation
    on_edit: Callback<BlockId>,
    on_save: Callback<(BlockId, String)>,
    on_editing_end: Option<Callback<()>>, // Called when editing ends to restore document focus
    on_file_select: Option<Callback<PathBuf>>,
    document_state: crate::models::DocumentState, // Needed to check which list items are being edited
) -> Element {
    if let Some(raw) = editing_raw {
        let mut content = use_signal(|| raw.clone());

        let save_content = move || {
            on_save.call((block_id, content.read().clone()));
            // Request document focus after editing ends
            if let Some(focus_callback) = on_editing_end {
                focus_callback.call(());
            }
        };

        rsx! {
            textarea {
                class: "block-editor",
                value: content.read().clone(),
                autofocus: true,
                rows: content.read().lines().count().max(3),
                onmounted: move |evt| {
                    // Force focus when textarea is mounted
                    spawn(async move {
                        let _ = evt.set_focus(true).await;
                    });
                },
                oninput: move |evt| {
                    content.set(evt.value());
                },
                onblur: move |_| {
                    save_content();
                },
                onkeydown: move |evt| {
                    if evt.key() == Key::Escape || (evt.key() == Key::Enter && evt.data().modifiers().ctrl()) {
                        save_content();
                    }
                }
            }
        }
    } else {
        let block_class = if is_selected {
            "editable-block selected"
        } else {
            "editable-block"
        };

        match &block {
            ContentBlock::BulletList { items } => {
                rsx! {
                    div {
                        class: "bullet-list",
                        for (item_block_id, item) in items {
                            EditableListItem {
                                key: "{item_block_id:?}",
                                item: item.clone(),
                                block_id: *item_block_id,
                                editing_raw: document_state.is_editing(*item_block_id).cloned(),
                                is_selected: document_state.selected_block() == Some(*item_block_id),
                                on_edit: on_edit,
                                on_save: on_save,
                                on_editing_end: on_editing_end,
                                on_file_select: on_file_select,
                                is_numbered: false,
                                item_number: None,
                            }
                        }
                    }
                }
            }
            ContentBlock::NumberedList { items } => {
                rsx! {
                    div {
                        class: "numbered-list",
                        for (idx, (item_block_id, item)) in items.iter().enumerate() {
                            EditableListItem {
                                key: "{item_block_id:?}",
                                item: item.clone(),
                                block_id: *item_block_id,
                                editing_raw: document_state.is_editing(*item_block_id).cloned(),
                                is_selected: document_state.selected_block() == Some(*item_block_id),
                                on_edit: on_edit,
                                on_save: on_save,
                                on_editing_end: on_editing_end,
                                on_file_select: on_file_select,
                                is_numbered: true,
                                item_number: Some(idx + 1),
                            }
                        }
                    }
                }
            }
            _ => {
                rsx! {
                    div {
                        class: "{block_class}",
                        onclick: move |_| on_edit.call(block_id),
                        // Render non-list blocks normally using existing components
                        super::ContentBlockComponent {
                            block: block,
                            on_file_select: on_file_select
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn EditableListItem(
    item: ListItem,
    block_id: BlockId,
    editing_raw: Option<String>,
    is_selected: bool,
    on_edit: Callback<BlockId>,
    on_save: Callback<(BlockId, String)>,
    on_editing_end: Option<Callback<()>>,
    on_file_select: Option<Callback<PathBuf>>,
    is_numbered: bool,
    item_number: Option<usize>,
) -> Element {
    if let Some(raw) = editing_raw {
        // Use the exact same editing interface as other blocks
        let mut content = use_signal(|| raw.clone());

        let save_content = move || {
            on_save.call((block_id, content.read().clone()));
            if let Some(focus_callback) = on_editing_end {
                focus_callback.call(());
            }
        };

        rsx! {
            textarea {
                class: "block-editor",
                value: content.read().clone(),
                autofocus: true,
                rows: content.read().lines().count().max(3),
                onmounted: move |evt| {
                    spawn(async move {
                        let _ = evt.set_focus(true).await;
                    });
                },
                oninput: move |evt| {
                    content.set(evt.value());
                },
                onblur: move |_| {
                    save_content();
                },
                onkeydown: move |evt| {
                    if evt.key() == Key::Escape || (evt.key() == Key::Enter && evt.data().modifiers().ctrl()) {
                        save_content();
                    }
                }
            }
        }
    } else {
        let item_class = if is_selected {
            "editable-block selected"
        } else {
            "editable-block"
        };

        rsx! {
            div {
                class: "{item_class}",
                onclick: move |_| on_edit.call(block_id),
                super::OutlineItemComponent {
                    item: item,
                    indent: 0,
                    is_numbered: is_numbered,
                    item_number: item_number,
                    on_file_select: on_file_select
                }
            }
        }
    }
}
