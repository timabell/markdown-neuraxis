use crate::models::{BlockId, ContentBlock, DocumentState, ListItem};
use dioxus::prelude::*;
use std::path::PathBuf;

use super::{NestedContentComponent, TextSegmentComponent};

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
                                document_state: document_state.clone(),
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
                                document_state: document_state.clone(),
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
    document_state: DocumentState,
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
        let _item_class = if is_selected {
            "editable-block selected"
        } else {
            "editable-block"
        };

        rsx! {
            EditableOutlineItemComponent {
                item: item,
                block_id: block_id,
                indent: 0,
                is_numbered: is_numbered,
                item_number: item_number,
                on_edit: on_edit,
                on_save: on_save,
                on_editing_end: on_editing_end,
                on_file_select: on_file_select,
                document_state: document_state.clone(),
            }
        }
    }
}

#[component]
pub fn EditableOutlineItemComponent(
    item: ListItem,
    block_id: BlockId,
    indent: usize,
    is_numbered: bool,
    item_number: Option<usize>,
    on_edit: Callback<BlockId>,
    on_save: Callback<(BlockId, String)>,
    on_editing_end: Option<Callback<()>>,
    on_file_select: Option<Callback<PathBuf>>,
    document_state: DocumentState,
) -> Element {
    let is_editing = document_state.is_editing(block_id).is_some();
    let is_selected = document_state.selected_block() == Some(block_id);

    if is_editing {
        let editing_raw = document_state
            .is_editing(block_id)
            .cloned()
            .unwrap_or_default();
        let mut content = use_signal(|| editing_raw.clone());

        let save_content = move || {
            on_save.call((block_id, content.read().clone()));
            if let Some(focus_callback) = on_editing_end {
                focus_callback.call(());
            }
        };

        rsx! {
            textarea {
                class: "block-editor",
                style: "margin-left: {indent * 24}px;",
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
                div {
                    class: "list-item {item_class}",
                    style: "margin-left: {indent * 24}px;",
                    onclick: move |_| on_edit.call(block_id),
                    div {
                        class: "list-item-content",
                        if is_numbered {
                            if let Some(num) = item_number {
                                span { class: "list-marker numbered", "{num}. " }
                            } else {
                                span { class: "list-marker numbered", "• " }
                            }
                        } else {
                            span { class: "list-marker bullet", "• " }
                        }
                        span {
                            class: "list-text",
                            if let Some(ref segments) = item.segments {
                                for segment in segments {
                                    TextSegmentComponent { segment: segment.clone(), on_file_select: on_file_select }
                                }
                            } else {
                                "{item.content}"
                            }
                        }
                    }
                    // Render nested content (like code blocks)
                    if !item.nested_content.is_empty() {
                        div {
                            class: "nested-content",
                            style: "margin-left: {(indent + 1) * 24}px;",
                            for content in &item.nested_content {
                                NestedContentComponent { content: content.clone(), on_file_select: on_file_select }
                            }
                        }
                    }
                }
                // Render nested children as individually editable items
                if !item.children.is_empty() {
                    div {
                        class: "nested-list",
                        for (idx, (child_id, child)) in item.children.iter().enumerate() {
                            EditableOutlineItemComponent {
                                key: "{child_id:?}",
                                item: child.clone(),
                                block_id: *child_id,
                                indent: indent + 1,
                                is_numbered: is_numbered,
                                item_number: if is_numbered { Some(idx + 1) } else { None },
                                on_edit: on_edit,
                                on_save: on_save,
                                on_editing_end: on_editing_end,
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
